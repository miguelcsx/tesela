// ActionTypeRepo persists types.ActionType entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// ActionTypeRepo handles CRUD for action types.
type ActionTypeRepo struct{ q Querier }

// Upsert inserts or replaces an action type.
func (r *ActionTypeRepo) Upsert(ctx context.Context, at types.ActionType) (types.ActionType, error) {
	handler, err := json.Marshal(at.Handler)
	if err != nil {
		return types.ActionType{}, fmt.Errorf("marshal handler: %w", err)
	}
	const q = `
INSERT INTO action_types (id, workspace_id, api_name, display_name, description, subject, input_schema, output_schema, permission_key, idempotency_key_template, execution_mode, handler, version, deprecated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
ON CONFLICT (workspace_id, api_name) DO UPDATE SET
    display_name=EXCLUDED.display_name,
    description=EXCLUDED.description,
    subject=EXCLUDED.subject,
    input_schema=EXCLUDED.input_schema,
    output_schema=EXCLUDED.output_schema,
    permission_key=EXCLUDED.permission_key,
    idempotency_key_template=EXCLUDED.idempotency_key_template,
    execution_mode=EXCLUDED.execution_mode,
    handler=EXCLUDED.handler,
    version=action_types.version + 1,
    deprecated_at=EXCLUDED.deprecated_at,
    updated_at=now()
RETURNING id, version, created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, at.ID, at.WorkspaceID, at.APIName, at.DisplayName, at.Description,
		at.Subject, at.InputSchema, at.OutputSchema, at.PermissionKey, at.IdempotencyKeyTemplate,
		at.ExecutionMode, handler, at.Version, at.DeprecatedAt).
		Scan(&at.ID, &at.Version, &at.CreatedAt, &at.UpdatedAt); err != nil {
		return types.ActionType{}, classifyError(err)
	}
	return at, nil
}

// GetByAPIName returns the action type with the given api_name.
func (r *ActionTypeRepo) GetByAPIName(ctx context.Context, ws types.WorkspaceID, name types.APIName) (types.ActionType, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, subject, input_schema, output_schema, permission_key, idempotency_key_template, execution_mode, handler, version, deprecated_at, created_at, updated_at
FROM action_types WHERE workspace_id = $1 AND api_name = $2`
	return scanActionType(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every action type for a workspace.
func (r *ActionTypeRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.ActionType, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, subject, input_schema, output_schema, permission_key, idempotency_key_template, execution_mode, handler, version, deprecated_at, created_at, updated_at
FROM action_types WHERE workspace_id = $1 ORDER BY api_name`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.ActionType
	for rows.Next() {
		at, err := scanActionType(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, at)
	}
	return out, rows.Err()
}

// Delete removes an action type by api_name.
func (r *ActionTypeRepo) Delete(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	const q = `DELETE FROM action_types WHERE workspace_id = $1 AND api_name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func scanActionType(row rowScanner) (types.ActionType, error) {
	var at types.ActionType
	var handler []byte
	if err := row.Scan(&at.ID, &at.WorkspaceID, &at.APIName, &at.DisplayName, &at.Description,
		&at.Subject, &at.InputSchema, &at.OutputSchema, &at.PermissionKey, &at.IdempotencyKeyTemplate,
		&at.ExecutionMode, &handler, &at.Version, &at.DeprecatedAt, &at.CreatedAt, &at.UpdatedAt); err != nil {
		return types.ActionType{}, classifyError(err)
	}
	if len(handler) > 0 {
		if err := json.Unmarshal(handler, &at.Handler); err != nil {
			return types.ActionType{}, fmt.Errorf("unmarshal handler: %w", err)
		}
	}
	return at, nil
}
