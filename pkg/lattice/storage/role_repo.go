// RoleRepo persists types.Role entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// RoleRepo handles CRUD for roles.
type RoleRepo struct{ q Querier }

// Upsert inserts or replaces a role by (workspace_id, api_name).
func (r *RoleRepo) Upsert(ctx context.Context, role types.Role) (types.Role, error) {
	inherits, err := json.Marshal(role.Inherits)
	if err != nil {
		return types.Role{}, fmt.Errorf("marshal inherits: %w", err)
	}
	const q = `
INSERT INTO roles (id, workspace_id, api_name, display_name, description, inherits)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (workspace_id, api_name) DO UPDATE SET
    display_name=EXCLUDED.display_name,
    description=EXCLUDED.description,
    inherits=EXCLUDED.inherits,
    updated_at=now()
RETURNING id, created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, role.ID, role.WorkspaceID, role.APIName, role.DisplayName, role.Description, inherits).
		Scan(&role.ID, &role.CreatedAt, &role.UpdatedAt); err != nil {
		return types.Role{}, classifyError(err)
	}
	return role, nil
}

// GetByAPIName returns the role with the given api_name.
func (r *RoleRepo) GetByAPIName(ctx context.Context, ws types.WorkspaceID, name types.APIName) (types.Role, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, inherits, created_at, updated_at
FROM roles WHERE workspace_id = $1 AND api_name = $2`
	return scanRole(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every role for a workspace.
func (r *RoleRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.Role, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, inherits, created_at, updated_at
FROM roles WHERE workspace_id = $1 ORDER BY api_name`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.Role
	for rows.Next() {
		role, err := scanRole(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, role)
	}
	return out, rows.Err()
}

// Delete removes a role by api_name.
func (r *RoleRepo) Delete(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	const q = `DELETE FROM roles WHERE workspace_id = $1 AND api_name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func scanRole(row rowScanner) (types.Role, error) {
	var role types.Role
	var inherits []byte
	if err := row.Scan(&role.ID, &role.WorkspaceID, &role.APIName, &role.DisplayName, &role.Description,
		&inherits, &role.CreatedAt, &role.UpdatedAt); err != nil {
		return types.Role{}, classifyError(err)
	}
	if len(inherits) > 0 {
		if err := json.Unmarshal(inherits, &role.Inherits); err != nil {
			return types.Role{}, fmt.Errorf("unmarshal inherits: %w", err)
		}
	}
	return role, nil
}
