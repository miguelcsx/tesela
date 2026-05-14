// DatasourceRepo persists types.Datasource entities. Sealed credentials are
// stored as a BYTEA column; the application layer encrypts/decrypts them via
// internal/crypto.Sealer before/after store calls.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// DatasourceRepo handles CRUD for datasources.
type DatasourceRepo struct{ q Querier }

// Create inserts a datasource.
func (r *DatasourceRepo) Create(ctx context.Context, ds types.Datasource) (types.Datasource, error) {
	cfg, err := json.Marshal(ds.Config)
	if err != nil {
		return types.Datasource{}, fmt.Errorf("marshal config: %w", err)
	}
	const q = `
INSERT INTO datasources (id, workspace_id, api_name, display_name, adapter_type, config, sealed_credentials)
VALUES ($1, $2, $3, $4, $5, $6, $7)
RETURNING created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, ds.ID, ds.WorkspaceID, ds.APIName, ds.DisplayName,
		ds.AdapterType, cfg, ds.SealedCredentials).Scan(&ds.CreatedAt, &ds.UpdatedAt); err != nil {
		return types.Datasource{}, classifyError(err)
	}
	return ds, nil
}

// GetByID returns a datasource within a workspace.
func (r *DatasourceRepo) GetByID(ctx context.Context, ws types.WorkspaceID, id types.DatasourceID) (types.Datasource, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, adapter_type, config, sealed_credentials, created_at, updated_at
FROM datasources WHERE workspace_id = $1 AND id = $2`
	return scanDatasource(r.q.QueryRow(ctx, q, ws, id))
}

// GetByAPIName returns a datasource by api_name within a workspace.
func (r *DatasourceRepo) GetByAPIName(ctx context.Context, ws types.WorkspaceID, name types.APIName) (types.Datasource, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, adapter_type, config, sealed_credentials, created_at, updated_at
FROM datasources WHERE workspace_id = $1 AND api_name = $2`
	return scanDatasource(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every datasource for the given workspace.
func (r *DatasourceRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.Datasource, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, adapter_type, config, sealed_credentials, created_at, updated_at
FROM datasources WHERE workspace_id = $1 ORDER BY api_name`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.Datasource
	for rows.Next() {
		ds, err := scanDatasource(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, ds)
	}
	return out, rows.Err()
}

// Upsert performs INSERT ... ON CONFLICT (workspace_id, api_name) DO UPDATE.
// Used by ontology apply.
func (r *DatasourceRepo) Upsert(ctx context.Context, ds types.Datasource) (types.Datasource, error) {
	cfg, err := json.Marshal(ds.Config)
	if err != nil {
		return types.Datasource{}, fmt.Errorf("marshal config: %w", err)
	}
	const q = `
INSERT INTO datasources (id, workspace_id, api_name, display_name, adapter_type, config, sealed_credentials)
VALUES ($1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (workspace_id, api_name) DO UPDATE SET
    display_name=EXCLUDED.display_name,
    adapter_type=EXCLUDED.adapter_type,
    config=EXCLUDED.config,
    sealed_credentials=COALESCE(EXCLUDED.sealed_credentials, datasources.sealed_credentials),
    updated_at=now()
RETURNING id, created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, ds.ID, ds.WorkspaceID, ds.APIName, ds.DisplayName,
		ds.AdapterType, cfg, ds.SealedCredentials).Scan(&ds.ID, &ds.CreatedAt, &ds.UpdatedAt); err != nil {
		return types.Datasource{}, classifyError(err)
	}
	return ds, nil
}

// Delete removes a datasource by api_name.
func (r *DatasourceRepo) Delete(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	const q = `DELETE FROM datasources WHERE workspace_id = $1 AND api_name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func scanDatasource(row rowScanner) (types.Datasource, error) {
	var ds types.Datasource
	var cfg []byte
	if err := row.Scan(&ds.ID, &ds.WorkspaceID, &ds.APIName, &ds.DisplayName,
		&ds.AdapterType, &cfg, &ds.SealedCredentials, &ds.CreatedAt, &ds.UpdatedAt); err != nil {
		return types.Datasource{}, classifyError(err)
	}
	if len(cfg) > 0 {
		if err := json.Unmarshal(cfg, &ds.Config); err != nil {
			return types.Datasource{}, fmt.Errorf("unmarshal config: %w", err)
		}
	}
	return ds, nil
}
