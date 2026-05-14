// Authenticator resolves an Authorization header into a types.Actor or returns
// an *errs.Error tagged CodeUnauthenticated.

package auth

import (
	"context"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Authenticator validates an inbound credential and produces an Actor.
type Authenticator interface {
	// Authenticate accepts the raw value of the HTTP Authorization header and
	// returns the corresponding actor, or an *errs.Error.
	Authenticate(ctx context.Context, authorizationHeader string) (types.Actor, error)
}
