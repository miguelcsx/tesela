// EnvProvider resolves references against the process environment.

package secrets

import (
	"context"
	"errors"
	"fmt"
	"os"
)

// EnvProvider is a SecretProvider that reads from os.Getenv.
type EnvProvider struct{}

// NewEnvProvider returns the default environment-backed provider.
func NewEnvProvider() *EnvProvider { return &EnvProvider{} }

// Name implements SecretProvider.
func (*EnvProvider) Name() string { return "env" }

// Lookup implements SecretProvider.
func (*EnvProvider) Lookup(_ context.Context, reference string) (string, error) {
	if reference == "" {
		return "", errors.New("env provider: reference must not be empty")
	}
	v, ok := os.LookupEnv(reference)
	if !ok {
		return "", fmt.Errorf("%w: env %q", ErrNotFound, reference)
	}
	return v, nil
}
