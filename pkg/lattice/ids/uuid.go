// Random RFC 4122 v4 identifier in canonical 36-char hyphenated form.

package ids

import "github.com/google/uuid"

// NewUUID returns a UUIDv4 (random) in canonical lowercase hyphenated form.
func NewUUID() string {
	return uuid.NewString()
}
