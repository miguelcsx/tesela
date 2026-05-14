// JWTAuthenticator validates RS256/ES256 (and others, configurable) tokens
// against an OIDC issuer. JWKS rotation is handled by go-oidc's KeySet, which
// caches and refreshes the JWKS automatically.

package auth

import (
	"context"
	"fmt"

	"github.com/coreos/go-oidc/v3/oidc"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// JWTConfig configures NewJWTAuthenticator.
type JWTConfig struct {
	// Issuer is the OIDC issuer URL — must match the iss claim exactly.
	Issuer string
	// Audience is the expected aud claim value.
	Audience string
	// AcceptedAlgs is the allowlist of signature algorithms; empty means
	// {"RS256"}.
	AcceptedAlgs []string
	// RolesClaim names the JWT claim from which roles are read; empty
	// means "roles".
	RolesClaim string
	// UserIDClaim names the claim that maps to Actor.UserID; empty means
	// "sub".
	UserIDClaim string
	// WorkspaceClaim names the claim that maps to Actor.WorkspaceID; empty
	// means "workspace_id".
	WorkspaceClaim string
	// ClockSkewSeconds is forwarded to the OIDC verifier as the allowed leeway
	// for nbf/iat/exp checks (best-effort; some validators ignore it).
	ClockSkewSeconds int
}

// JWTAuthenticator validates inbound JWTs.
type JWTAuthenticator struct {
	cfg      JWTConfig
	verifier *oidc.IDTokenVerifier
}

// NewJWTAuthenticator initializes the authenticator by performing OIDC
// discovery against cfg.Issuer. Returns an error if discovery fails.
func NewJWTAuthenticator(ctx context.Context, cfg JWTConfig) (*JWTAuthenticator, error) {
	cfg = applyJWTDefaults(cfg)
	// Issuer URL is passed verbatim; OIDC requires exact iss matching against
	// what discovery returns, so trimming a trailing slash here would cause
	// the verifier to reject every token.
	provider, err := oidc.NewProvider(ctx, cfg.Issuer)
	if err != nil {
		return nil, fmt.Errorf("oidc discovery for %q: %w", cfg.Issuer, err)
	}
	verifier := provider.Verifier(&oidc.Config{
		ClientID:             cfg.Audience,
		SupportedSigningAlgs: cfg.AcceptedAlgs,
	})
	return &JWTAuthenticator{cfg: cfg, verifier: verifier}, nil
}

// Authenticate parses the bearer token, verifies its signature and standard
// claims via the OIDC verifier, and assembles a types.Actor.
func (a *JWTAuthenticator) Authenticate(ctx context.Context, header string) (types.Actor, error) {
	tok, ok := ParseBearerToken(header)
	if !ok {
		return types.Actor{}, errs.New(errs.CodeUnauthenticated, "missing or malformed Authorization header")
	}
	idTok, err := a.verifier.Verify(ctx, tok)
	if err != nil {
		return types.Actor{}, errs.Wrap(err, errs.CodeUnauthenticated, "token verification failed")
	}
	var rawClaims map[string]any
	if err := idTok.Claims(&rawClaims); err != nil {
		return types.Actor{}, errs.Wrap(err, errs.CodeUnauthenticated, "claims decode failed")
	}
	return assembleActor(a.cfg, rawClaims)
}

// applyJWTDefaults fills in unset configuration fields.
func applyJWTDefaults(cfg JWTConfig) JWTConfig {
	if len(cfg.AcceptedAlgs) == 0 {
		cfg.AcceptedAlgs = []string{"RS256"}
	}
	if cfg.RolesClaim == "" {
		cfg.RolesClaim = "roles"
	}
	if cfg.UserIDClaim == "" {
		cfg.UserIDClaim = "sub"
	}
	if cfg.WorkspaceClaim == "" {
		cfg.WorkspaceClaim = "workspace_id"
	}
	return cfg
}

// assembleActor maps a verified claims bag onto a types.Actor.
func assembleActor(cfg JWTConfig, claims map[string]any) (types.Actor, error) {
	userID, err := claimString(claims, cfg.UserIDClaim)
	if err != nil {
		return types.Actor{}, errs.Wrap(err, errs.CodeUnauthenticated, "user id claim")
	}
	if userID == "" {
		return types.Actor{}, errs.New(errs.CodeUnauthenticated, "user id claim is empty")
	}
	workspaceID, _ := claimString(claims, cfg.WorkspaceClaim)
	roles := claimStringSlice(claims, cfg.RolesClaim)
	return types.Actor{
		UserID:      userID,
		WorkspaceID: workspaceID,
		Roles:       roles,
		Claims:      claims,
	}, nil
}

func claimString(claims map[string]any, key string) (string, error) {
	v, ok := claims[key]
	if !ok || v == nil {
		return "", nil
	}
	s, ok := v.(string)
	if !ok {
		return "", fmt.Errorf("claim %q is not a string", key)
	}
	return s, nil
}

func claimStringSlice(claims map[string]any, key string) []string {
	v, ok := claims[key]
	if !ok || v == nil {
		return nil
	}
	switch typed := v.(type) {
	case []string:
		return typed
	case string:
		return []string{typed}
	case []any:
		out := make([]string, 0, len(typed))
		for _, raw := range typed {
			if s, ok := raw.(string); ok {
				out = append(out, s)
			}
		}
		return out
	default:
		return nil
	}
}
