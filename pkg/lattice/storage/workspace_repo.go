// WorkspaceRepo is the metadata persistence for types.Workspace entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// WorkspaceRepo handles CRUD for workspaces.
type WorkspaceRepo struct{ q Querier }

// Create inserts a new workspace.
func (r *WorkspaceRepo) Create(ctx context.Context, ws types.Workspace) (types.Workspace, error) {
	settings, err := json.Marshal(ws.Settings)
	if err != nil {
		return types.Workspace{}, fmt.Errorf("marshal settings: %w", err)
	}
	const q = `
INSERT INTO workspaces (id, api_name, display_name, description, settings)
VALUES ($1, $2, $3, $4, $5)
RETURNING created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, ws.ID, ws.APIName, ws.DisplayName, ws.Description, settings).
		Scan(&ws.CreatedAt, &ws.UpdatedAt); err != nil {
		return types.Workspace{}, classifyError(err)
	}
	return ws, nil
}

// GetByID returns the workspace with the given id.
func (r *WorkspaceRepo) GetByID(ctx context.Context, id types.WorkspaceID) (types.Workspace, error) {
	const q = `
SELECT id, api_name, display_name, description, settings, created_at, updated_at
FROM workspaces WHERE id = $1`
	return scanWorkspace(r.q.QueryRow(ctx, q, id))
}

// GetByAPIName returns the workspace with the given api_name.
func (r *WorkspaceRepo) GetByAPIName(ctx context.Context, name types.APIName) (types.Workspace, error) {
	const q = `
SELECT id, api_name, display_name, description, settings, created_at, updated_at
FROM workspaces WHERE api_name = $1`
	return scanWorkspace(r.q.QueryRow(ctx, q, name))
}

// List returns every workspace, ordered by api_name.
func (r *WorkspaceRepo) List(ctx context.Context) ([]types.Workspace, error) {
	const q = `
SELECT id, api_name, display_name, description, settings, created_at, updated_at
FROM workspaces ORDER BY api_name`
	rows, err := r.q.Query(ctx, q)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.Workspace
	for rows.Next() {
		ws, err := scanWorkspace(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, ws)
	}
	return out, rows.Err()
}

// Update writes mutable fields (display_name, description, settings).
func (r *WorkspaceRepo) Update(ctx context.Context, ws types.Workspace) (types.Workspace, error) {
	settings, err := json.Marshal(ws.Settings)
	if err != nil {
		return types.Workspace{}, fmt.Errorf("marshal settings: %w", err)
	}
	const q = `
UPDATE workspaces SET display_name=$2, description=$3, settings=$4, updated_at=now()
WHERE id=$1 RETURNING updated_at`
	if err := r.q.QueryRow(ctx, q, ws.ID, ws.DisplayName, ws.Description, settings).
		Scan(&ws.UpdatedAt); err != nil {
		return types.Workspace{}, classifyError(err)
	}
	return ws, nil
}

// Delete removes a workspace and (via ON DELETE CASCADE) all its children.
func (r *WorkspaceRepo) Delete(ctx context.Context, id types.WorkspaceID) error {
	const q = `DELETE FROM workspaces WHERE id = $1`
	tag, err := r.q.Exec(ctx, q, id)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

// scanWorkspace is the shared scan helper used by all read methods.
type rowScanner interface {
	Scan(dest ...any) error
}

func scanWorkspace(row rowScanner) (types.Workspace, error) {
	var ws types.Workspace
	var settings []byte
	if err := row.Scan(&ws.ID, &ws.APIName, &ws.DisplayName, &ws.Description, &settings,
		&ws.CreatedAt, &ws.UpdatedAt); err != nil {
		return types.Workspace{}, classifyError(err)
	}
	if len(settings) > 0 {
		if err := json.Unmarshal(settings, &ws.Settings); err != nil {
			return types.Workspace{}, fmt.Errorf("unmarshal settings: %w", err)
		}
	}
	return ws, nil
}
