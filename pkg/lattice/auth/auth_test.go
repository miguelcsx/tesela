package auth_test

import (
	"context"
	"crypto/rand"
	"crypto/rsa"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"math/big"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/auth"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
)

// testIssuer spins up a local OIDC discovery + JWKS server backed by a
// generated RSA key. It returns the configured authenticator plus a token
// minter callers can shape per test.
type testIssuer struct {
	URL    string
	Server *httptest.Server
	Key    *rsa.PrivateKey
	KeyID  string
}

func newTestIssuer(t *testing.T) *testIssuer {
	t.Helper()

	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatalf("rsa key: %v", err)
	}
	const kid = "test-key-1"

	mux := http.NewServeMux()
	srv := httptest.NewServer(mux)

	mux.HandleFunc("/.well-known/openid-configuration", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"issuer":                                srv.URL + "/",
			"jwks_uri":                              srv.URL + "/jwks",
			"id_token_signing_alg_values_supported": []string{"RS256"},
			"authorization_endpoint":                srv.URL + "/authorize",
			"token_endpoint":                        srv.URL + "/token",
			"response_types_supported":              []string{"id_token"},
			"subject_types_supported":               []string{"public"},
		})
	})
	mux.HandleFunc("/jwks", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"keys": []any{
				map[string]any{
					"kty": "RSA",
					"kid": kid,
					"alg": "RS256",
					"use": "sig",
					"n":   base64.RawURLEncoding.EncodeToString(key.PublicKey.N.Bytes()),
					"e":   base64.RawURLEncoding.EncodeToString(big.NewInt(int64(key.PublicKey.E)).Bytes()),
				},
			},
		})
	})

	return &testIssuer{URL: srv.URL + "/", Server: srv, Key: key, KeyID: kid}
}

func (ti *testIssuer) close() { ti.Server.Close() }

// mintToken builds a signed JWT with the supplied claims and the issuer's KID.
func (ti *testIssuer) mintToken(t *testing.T, claims jwt.MapClaims) string {
	t.Helper()
	tok := jwt.NewWithClaims(jwt.SigningMethodRS256, claims)
	tok.Header["kid"] = ti.KeyID
	signed, err := tok.SignedString(ti.Key)
	if err != nil {
		t.Fatalf("sign: %v", err)
	}
	return signed
}

func baseClaims(issuer string, audience string) jwt.MapClaims {
	now := time.Now()
	return jwt.MapClaims{
		"iss":          issuer,
		"sub":          "user-1",
		"aud":          audience,
		"iat":          now.Unix(),
		"exp":          now.Add(5 * time.Minute).Unix(),
		"nbf":          now.Add(-1 * time.Minute).Unix(),
		"roles":        []any{"admin", "analyst"},
		"workspace_id": "ws-demo",
	}
}

func TestJWTAuthenticator_ResolvesActor(t *testing.T) {
	ti := newTestIssuer(t)
	defer ti.close()

	authn, err := auth.NewJWTAuthenticator(context.Background(), auth.JWTConfig{
		Issuer:       ti.URL,
		Audience:     "lattice-test",
		RolesClaim:   "roles",
		UserIDClaim:  "sub",
		AcceptedAlgs: []string{"RS256"},
	})
	if err != nil {
		t.Fatalf("NewJWTAuthenticator: %v", err)
	}

	tok := ti.mintToken(t, baseClaims(ti.URL, "lattice-test"))
	actor, err := authn.Authenticate(context.Background(), "Bearer "+tok)
	if err != nil {
		t.Fatalf("Authenticate: %v", err)
	}
	if actor.UserID != "user-1" {
		t.Fatalf("UserID: %q", actor.UserID)
	}
	if !actor.HasRole("admin") || !actor.HasRole("analyst") {
		t.Fatalf("Roles: %v", actor.Roles)
	}
	if actor.WorkspaceID != "ws-demo" {
		t.Fatalf("WorkspaceID: %q", actor.WorkspaceID)
	}
	if v, _ := actor.Claim("workspace_id"); v != "ws-demo" {
		t.Fatalf("Claim workspace_id: %v", v)
	}
}

func TestJWTAuthenticator_RejectsExpiredToken(t *testing.T) {
	ti := newTestIssuer(t)
	defer ti.close()

	authn, err := auth.NewJWTAuthenticator(context.Background(), auth.JWTConfig{
		Issuer:   ti.URL,
		Audience: "lattice-test",
	})
	if err != nil {
		t.Fatalf("NewJWTAuthenticator: %v", err)
	}

	claims := baseClaims(ti.URL, "lattice-test")
	claims["exp"] = time.Now().Add(-1 * time.Hour).Unix()
	tok := ti.mintToken(t, claims)

	if _, err := authn.Authenticate(context.Background(), "Bearer "+tok); err == nil {
		t.Fatal("expected error for expired token")
	} else if !errs.Is(err, errs.CodeUnauthenticated) {
		t.Fatalf("expected CodeUnauthenticated, got %v", err)
	}
}

func TestJWTAuthenticator_RejectsBadAudience(t *testing.T) {
	ti := newTestIssuer(t)
	defer ti.close()

	authn, _ := auth.NewJWTAuthenticator(context.Background(), auth.JWTConfig{
		Issuer:   ti.URL,
		Audience: "lattice-test",
	})

	claims := baseClaims(ti.URL, "wrong-audience")
	tok := ti.mintToken(t, claims)
	_, err := authn.Authenticate(context.Background(), "Bearer "+tok)
	if !errs.Is(err, errs.CodeUnauthenticated) {
		t.Fatalf("expected CodeUnauthenticated, got %v", err)
	}
}

func TestJWTAuthenticator_RejectsMissingHeader(t *testing.T) {
	ti := newTestIssuer(t)
	defer ti.close()

	authn, _ := auth.NewJWTAuthenticator(context.Background(), auth.JWTConfig{
		Issuer:   ti.URL,
		Audience: "lattice-test",
	})
	for _, h := range []string{"", "Token foo", "Bearer", "Bearer "} {
		if _, err := authn.Authenticate(context.Background(), h); !errs.Is(err, errs.CodeUnauthenticated) {
			t.Fatalf("header %q: expected unauthenticated, got %v", h, err)
		}
	}
}

func TestJWTAuthenticator_RejectsBadSignature(t *testing.T) {
	ti := newTestIssuer(t)
	defer ti.close()

	authn, _ := auth.NewJWTAuthenticator(context.Background(), auth.JWTConfig{
		Issuer:   ti.URL,
		Audience: "lattice-test",
	})

	otherKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	tok := jwt.NewWithClaims(jwt.SigningMethodRS256, baseClaims(ti.URL, "lattice-test"))
	tok.Header["kid"] = ti.KeyID
	signed, err := tok.SignedString(otherKey)
	if err != nil {
		t.Fatalf("sign: %v", err)
	}

	if _, err := authn.Authenticate(context.Background(), "Bearer "+signed); !errs.Is(err, errs.CodeUnauthenticated) {
		t.Fatalf("expected unauthenticated, got %v", err)
	}
}

func TestJWTAuthenticator_AllowsCustomRolesClaim(t *testing.T) {
	ti := newTestIssuer(t)
	defer ti.close()

	authn, _ := auth.NewJWTAuthenticator(context.Background(), auth.JWTConfig{
		Issuer:      ti.URL,
		Audience:    "lattice-test",
		RolesClaim:  "groups",
		UserIDClaim: "sub",
	})
	claims := baseClaims(ti.URL, "lattice-test")
	delete(claims, "roles")
	claims["groups"] = []any{"viewer"}
	tok := ti.mintToken(t, claims)

	actor, err := authn.Authenticate(context.Background(), "Bearer "+tok)
	if err != nil {
		t.Fatalf("Authenticate: %v", err)
	}
	if !actor.HasRole("viewer") {
		t.Fatalf("Roles: %v", actor.Roles)
	}
}

func TestJWTAuthenticator_NewFailsWhenIssuerUnreachable(t *testing.T) {
	if _, err := auth.NewJWTAuthenticator(context.Background(), auth.JWTConfig{
		Issuer:   "http://127.0.0.1:1/",
		Audience: "x",
	}); err == nil {
		t.Fatal("expected error for unreachable issuer")
	}
}

func TestParseBearerToken(t *testing.T) {
	t.Parallel()

	cases := []struct {
		header string
		want   string
		ok     bool
	}{
		{"Bearer abc", "abc", true},
		{"bearer abc", "abc", true},
		{"BEARER abc", "abc", true},
		{"Bearer  abc", "abc", true},
		{" Bearer abc ", "abc", true},
		{"Token abc", "", false},
		{"Bearer", "", false},
		{"", "", false},
	}
	for _, c := range cases {
		got, ok := auth.ParseBearerToken(c.header)
		if got != c.want || ok != c.ok {
			t.Fatalf("ParseBearerToken(%q) = (%q,%v), want (%q,%v)", c.header, got, ok, c.want, c.ok)
		}
	}
}

// Verify our test setup is sound: the issuer must serve discovery.
func TestTestIssuer_Discoverable(t *testing.T) {
	ti := newTestIssuer(t)
	defer ti.close()

	resp, err := http.Get(ti.URL + ".well-known/openid-configuration")
	if err != nil {
		t.Fatalf("get: %v", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("status: %d", resp.StatusCode)
	}
	var doc map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&doc); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if got := fmt.Sprint(doc["issuer"]); got != ti.URL {
		t.Fatalf("issuer mismatch: %s", got)
	}
}
