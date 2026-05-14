// Migration runner. Wraps goose so callers receive a typed error and never
// import goose directly.

package storage

import (
	"context"
	"database/sql"
	"errors"
	"fmt"

	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/jackc/pgx/v5/stdlib"
	"github.com/pressly/goose/v3"

	"github.com/miguelcsx/lattice/pkg/lattice/storage/migrations"
)

// MigrateUp applies all pending migrations against the metadata database.
// Caller supplies the resolved DSN (no env reads here).
func MigrateUp(ctx context.Context, dsn string) error {
	db, err := openSQL(dsn)
	if err != nil {
		return err
	}
	defer db.Close()
	return runGoose(ctx, db, "up", 0)
}

// MigrateDown rolls back the most recent migration. Use MigrateDownTo for
// targeted rollback.
func MigrateDown(ctx context.Context, dsn string) error {
	db, err := openSQL(dsn)
	if err != nil {
		return err
	}
	defer db.Close()
	return runGoose(ctx, db, "down", 0)
}

// MigrateDownTo rolls back migrations until the target version is current.
func MigrateDownTo(ctx context.Context, dsn string, target int64) error {
	db, err := openSQL(dsn)
	if err != nil {
		return err
	}
	defer db.Close()
	return runGoose(ctx, db, "down-to", target)
}

// MigrationStatus returns the current applied version.
func MigrationStatus(ctx context.Context, dsn string) (int64, error) {
	db, err := openSQL(dsn)
	if err != nil {
		return 0, err
	}
	defer db.Close()
	if err := configureGoose(); err != nil {
		return 0, err
	}
	v, err := goose.GetDBVersionContext(ctx, db)
	if err != nil {
		return 0, fmt.Errorf("goose version: %w", err)
	}
	return v, nil
}

// MigrateUpUsing applies migrations on an existing pool — used by Store.Open
// when migrate_on_start is enabled, to avoid opening a second connection.
func MigrateUpUsing(ctx context.Context, pool *pgxpool.Pool) error {
	connStr := pool.Config().ConnString()
	return MigrateUp(ctx, connStr)
}

// openSQL returns a *sql.DB backed by pgx, suitable for goose.
func openSQL(dsn string) (*sql.DB, error) {
	db, err := sql.Open("pgx", dsn)
	if err != nil {
		return nil, fmt.Errorf("sql open: %w", err)
	}
	return db, nil
}

// runGoose runs the named goose command (up, down, down-to) once.
func runGoose(ctx context.Context, db *sql.DB, command string, target int64) error {
	if err := configureGoose(); err != nil {
		return err
	}
	switch command {
	case "up":
		return goose.UpContext(ctx, db, ".")
	case "down":
		return goose.DownContext(ctx, db, ".")
	case "down-to":
		return goose.DownToContext(ctx, db, ".", target)
	default:
		return fmt.Errorf("unknown migration command %q", command)
	}
}

// configureGoose sets the embedded FS as the migration source. Goose's globals
// are configured once per process; subsequent calls are cheap.
var configureGoose = configureGooseOnce

func configureGooseOnce() error {
	goose.SetBaseFS(migrations.FS)
	if err := goose.SetDialect("postgres"); err != nil {
		return fmt.Errorf("goose dialect: %w", err)
	}
	return nil
}

// Ensure pgx's stdlib driver is registered when this package is imported.
var _ = stdlib.Driver{}

// ErrNoChange signals that a migration command had no effect.
var ErrNoChange = errors.New("no change")
