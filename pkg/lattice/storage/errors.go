// Domain-level error sentinels that the store layer translates Postgres
// codes into.

package storage

import (
	"errors"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
)

// ErrNotFound is returned when a SELECT-by-id misses or a Get returns no rows.
var ErrNotFound = errors.New("store: not found")

// ErrConflict is returned when a UNIQUE constraint is violated (typically on
// (workspace_id, api_name) or on the action runs idempotency key).
var ErrConflict = errors.New("store: conflict")

// IsNotFound reports whether err is or wraps ErrNotFound.
func IsNotFound(err error) bool { return errors.Is(err, ErrNotFound) }

// IsConflict reports whether err is or wraps ErrConflict.
func IsConflict(err error) bool { return errors.Is(err, ErrConflict) }

// classifyError maps pgx's NoRows and Postgres unique-violation codes onto
// the store sentinels. Other errors pass through unchanged.
func classifyError(err error) error {
	if err == nil {
		return nil
	}
	if errors.Is(err, pgx.ErrNoRows) {
		return ErrNotFound
	}
	var pgErr *pgconn.PgError
	if errors.As(err, &pgErr) && pgErr.Code == "23505" {
		return ErrConflict
	}
	return err
}
