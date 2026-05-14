// SecretProvider is the contract every secret-store integration implements.

package secrets

import (
	"context"
	"errors"
)

// SecretProvider resolves a reference (e.g., a key name or a path) to a
// secret value. Implementations must be safe for concurrent use.
type SecretProvider interface {
	// Name returns a stable identifier for the provider implementation.
	// Used in telemetry and audit metadata.
	Name() string

	// Lookup resolves a reference to its current value. Returns an error
	// satisfying IsNotFound when the reference does not exist.
	Lookup(ctx context.Context, reference string) (string, error)
}

// ErrNotFound is returned (often wrapped) when a referenced secret is absent.
var ErrNotFound = errors.New("secret not found")

// IsNotFound reports whether err signals a missing-reference condition.
func IsNotFound(err error) bool { return errors.Is(err, ErrNotFound) }
