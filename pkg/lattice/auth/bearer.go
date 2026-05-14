// Bearer-token parsing helpers. Kept separate from the JWT verifier so other
// authenticator implementations (mTLS, API keys) can reuse the parser.

package auth

import "strings"

// bearerScheme is the case-insensitive scheme prefix recognized by ParseBearerToken.
const bearerScheme = "bearer"

// ParseBearerToken extracts the token component of a "Bearer <token>" header.
// Returns ok=false when the header is missing, malformed, or carries an empty
// token.
func ParseBearerToken(authorizationHeader string) (string, bool) {
	header := strings.TrimSpace(authorizationHeader)
	if header == "" {
		return "", false
	}
	parts := strings.Fields(header)
	if len(parts) < 2 {
		return "", false
	}
	if !strings.EqualFold(parts[0], bearerScheme) {
		return "", false
	}
	tok := strings.TrimSpace(strings.Join(parts[1:], " "))
	if tok == "" {
		return "", false
	}
	return tok, true
}
