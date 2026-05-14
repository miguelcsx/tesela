// Store is the composition root for metadata persistence. It owns the pgxpool
// and exposes typed repositories for every entity. Each repo is fully
// transactional via WithTx.

package storage

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

// Config controls Open.
type Config struct {
	DSN              string
	MaxOpenConns     int
	MaxIdleConns     int
	ConnLifetime     time.Duration
	StatementTimeout time.Duration
	MigrateOnStart   bool
}

// Store is a handle on the metadata database. It is safe for concurrent use.
type Store struct {
	pool *pgxpool.Pool
	cfg  Config

	repos repos
}

// repos holds every typed repository. They are constructed once at Open time
// and shared across the process.
type repos struct {
	workspaces       *WorkspaceRepo
	datasources      *DatasourceRepo
	objectTypes      *ObjectTypeRepo
	linkTypes        *LinkTypeRepo
	actionTypes      *ActionTypeRepo
	roles            *RoleRepo
	policyRules      *PolicyRuleRepo
	customTools      *CustomToolRepo
	agents           *AgentRepo
	assets           *AssetRepo
	uploads          *UploadRepo
	actionRuns       *ActionRunRepo
	agentRuns        *AgentRunRepo
	auditRecords     *AuditRecordRepo
	ontologyVersions *OntologyVersionRepo
	assetVersions    *AssetVersionRepo
}

// Open builds a Store. Optionally runs MigrateUp before returning.
func Open(ctx context.Context, cfg Config) (*Store, error) {
	if cfg.DSN == "" {
		return nil, errors.New("store: DSN is required")
	}
	pool, err := buildPool(ctx, cfg)
	if err != nil {
		return nil, err
	}
	if cfg.MigrateOnStart {
		if err := MigrateUpUsing(ctx, pool); err != nil {
			pool.Close()
			return nil, fmt.Errorf("migrate on start: %w", err)
		}
	}
	s := &Store{pool: pool, cfg: cfg}
	s.repos = buildRepos(pool)
	return s, nil
}

// Close drains the pool. Safe to call once.
func (s *Store) Close() {
	if s != nil && s.pool != nil {
		s.pool.Close()
	}
}

// Pool returns the underlying pool. Callers should prefer the typed repos.
func (s *Store) Pool() *pgxpool.Pool { return s.pool }

// Ping verifies connectivity to the metadata database.
func (s *Store) Ping(ctx context.Context) error {
	if err := s.pool.Ping(ctx); err != nil {
		return fmt.Errorf("ping metadata db: %w", err)
	}
	return nil
}

// Repos accessors. Each returns a *Repo whose methods are documented on the
// repo file.
func (s *Store) Workspaces() *WorkspaceRepo             { return s.repos.workspaces }
func (s *Store) Datasources() *DatasourceRepo           { return s.repos.datasources }
func (s *Store) ObjectTypes() *ObjectTypeRepo           { return s.repos.objectTypes }
func (s *Store) LinkTypes() *LinkTypeRepo               { return s.repos.linkTypes }
func (s *Store) ActionTypes() *ActionTypeRepo           { return s.repos.actionTypes }
func (s *Store) Roles() *RoleRepo                       { return s.repos.roles }
func (s *Store) PolicyRules() *PolicyRuleRepo           { return s.repos.policyRules }
func (s *Store) CustomTools() *CustomToolRepo           { return s.repos.customTools }
func (s *Store) Agents() *AgentRepo                     { return s.repos.agents }
func (s *Store) Assets() *AssetRepo                     { return s.repos.assets }
func (s *Store) Uploads() *UploadRepo                   { return s.repos.uploads }
func (s *Store) ActionRuns() *ActionRunRepo             { return s.repos.actionRuns }
func (s *Store) AgentRuns() *AgentRunRepo               { return s.repos.agentRuns }
func (s *Store) AuditRecords() *AuditRecordRepo         { return s.repos.auditRecords }
func (s *Store) OntologyVersions() *OntologyVersionRepo { return s.repos.ontologyVersions }
func (s *Store) AssetVersions() *AssetVersionRepo       { return s.repos.assetVersions }

// WithTx runs fn inside a serializable transaction. Returns the transaction
// error so the caller can decide how to surface it.
func (s *Store) WithTx(ctx context.Context, fn func(ctx context.Context, tx pgx.Tx) error) error {
	tx, err := s.pool.BeginTx(ctx, pgx.TxOptions{IsoLevel: pgx.Serializable})
	if err != nil {
		return fmt.Errorf("begin tx: %w", err)
	}
	if err := fn(ctx, tx); err != nil {
		_ = tx.Rollback(ctx)
		return err
	}
	return tx.Commit(ctx)
}

func buildPool(ctx context.Context, cfg Config) (*pgxpool.Pool, error) {
	pcfg, err := pgxpool.ParseConfig(cfg.DSN)
	if err != nil {
		return nil, fmt.Errorf("parse dsn: %w", err)
	}
	if cfg.MaxOpenConns > 0 {
		pcfg.MaxConns = int32(cfg.MaxOpenConns)
	}
	if cfg.MaxIdleConns > 0 {
		pcfg.MinConns = int32(cfg.MaxIdleConns)
	}
	if cfg.ConnLifetime > 0 {
		pcfg.MaxConnLifetime = cfg.ConnLifetime
	}
	pool, err := pgxpool.NewWithConfig(ctx, pcfg)
	if err != nil {
		return nil, fmt.Errorf("new pool: %w", err)
	}
	if err := pool.Ping(ctx); err != nil {
		pool.Close()
		return nil, fmt.Errorf("ping: %w", err)
	}
	return pool, nil
}

func buildRepos(pool *pgxpool.Pool) repos {
	return repos{
		workspaces:       &WorkspaceRepo{q: pool},
		datasources:      &DatasourceRepo{q: pool},
		objectTypes:      &ObjectTypeRepo{q: pool},
		linkTypes:        &LinkTypeRepo{q: pool},
		actionTypes:      &ActionTypeRepo{q: pool},
		roles:            &RoleRepo{q: pool},
		policyRules:      &PolicyRuleRepo{q: pool},
		customTools:      &CustomToolRepo{q: pool},
		agents:           &AgentRepo{q: pool},
		assets:           &AssetRepo{q: pool},
		uploads:          &UploadRepo{q: pool},
		actionRuns:       &ActionRunRepo{q: pool},
		agentRuns:        &AgentRunRepo{q: pool},
		auditRecords:     &AuditRecordRepo{q: pool},
		ontologyVersions: &OntologyVersionRepo{q: pool},
		assetVersions:    &AssetVersionRepo{q: pool},
	}
}
