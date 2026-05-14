// CustomToolRepo persists types.CustomTool entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// CustomToolRepo handles CRUD for custom tools.
type CustomToolRepo struct{ q Querier }

// Upsert inserts or replaces a custom tool by (workspace_id, api_name).
func (r *CustomToolRepo) Upsert(ctx context.Context, ct types.CustomTool) (types.CustomTool, error) {
	inputSchema, outputSchema, sqlSpec, webhook, composite, err := marshalCustomTool(ct)
	if err != nil {
		return types.CustomTool{}, err
	}
	const q = `
INSERT INTO custom_tools (id, workspace_id, api_name, display_name, description, kind, input_schema, output_schema, sql_spec, webhook, composite)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
ON CONFLICT (workspace_id, api_name) DO UPDATE SET
    display_name=EXCLUDED.display_name,
    description=EXCLUDED.description,
    kind=EXCLUDED.kind,
    input_schema=EXCLUDED.input_schema,
    output_schema=EXCLUDED.output_schema,
    sql_spec=EXCLUDED.sql_spec,
    webhook=EXCLUDED.webhook,
    composite=EXCLUDED.composite,
    updated_at=now()
RETURNING id, created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, ct.ID, ct.WorkspaceID, ct.APIName, ct.DisplayName, ct.Description,
		ct.Kind, inputSchema, outputSchema, sqlSpec, webhook, composite).
		Scan(&ct.ID, &ct.CreatedAt, &ct.UpdatedAt); err != nil {
		return types.CustomTool{}, classifyError(err)
	}
	return ct, nil
}

// GetByAPIName returns the custom tool with the given api_name.
func (r *CustomToolRepo) GetByAPIName(ctx context.Context, ws types.WorkspaceID, name types.APIName) (types.CustomTool, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, kind, input_schema, output_schema, sql_spec, webhook, composite, created_at, updated_at
FROM custom_tools WHERE workspace_id = $1 AND api_name = $2`
	return scanCustomTool(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every custom tool for a workspace.
func (r *CustomToolRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.CustomTool, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, kind, input_schema, output_schema, sql_spec, webhook, composite, created_at, updated_at
FROM custom_tools WHERE workspace_id = $1 ORDER BY api_name`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.CustomTool
	for rows.Next() {
		ct, err := scanCustomTool(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, ct)
	}
	return out, rows.Err()
}

// Delete removes a custom tool by api_name.
func (r *CustomToolRepo) Delete(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	const q = `DELETE FROM custom_tools WHERE workspace_id = $1 AND api_name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func marshalCustomTool(ct types.CustomTool) (inputSchema, outputSchema, sqlSpec, webhook, composite []byte, err error) {
	inputSchema = []byte(ct.InputSchema)
	if ct.OutputSchema != nil {
		outputSchema = []byte(ct.OutputSchema)
	}
	if ct.SQL != nil {
		if sqlSpec, err = json.Marshal(ct.SQL); err != nil {
			return nil, nil, nil, nil, nil, fmt.Errorf("marshal sql_spec: %w", err)
		}
	}
	if ct.Webhook != nil {
		if webhook, err = json.Marshal(ct.Webhook); err != nil {
			return nil, nil, nil, nil, nil, fmt.Errorf("marshal webhook: %w", err)
		}
	}
	if ct.Composite != nil {
		if composite, err = json.Marshal(ct.Composite); err != nil {
			return nil, nil, nil, nil, nil, fmt.Errorf("marshal composite: %w", err)
		}
	}
	return inputSchema, outputSchema, sqlSpec, webhook, composite, nil
}

func scanCustomTool(row rowScanner) (types.CustomTool, error) {
	var ct types.CustomTool
	var inputSchema, outputSchema, sqlSpec, webhook, composite []byte
	if err := row.Scan(&ct.ID, &ct.WorkspaceID, &ct.APIName, &ct.DisplayName, &ct.Description,
		&ct.Kind, &inputSchema, &outputSchema, &sqlSpec, &webhook, &composite,
		&ct.CreatedAt, &ct.UpdatedAt); err != nil {
		return types.CustomTool{}, classifyError(err)
	}
	if len(inputSchema) > 0 {
		ct.InputSchema = json.RawMessage(inputSchema)
	}
	if len(outputSchema) > 0 {
		ct.OutputSchema = json.RawMessage(outputSchema)
	}
	if len(sqlSpec) > 0 {
		if err := json.Unmarshal(sqlSpec, &ct.SQL); err != nil {
			return types.CustomTool{}, fmt.Errorf("unmarshal sql_spec: %w", err)
		}
	}
	if len(webhook) > 0 {
		if err := json.Unmarshal(webhook, &ct.Webhook); err != nil {
			return types.CustomTool{}, fmt.Errorf("unmarshal webhook: %w", err)
		}
	}
	if len(composite) > 0 {
		if err := json.Unmarshal(composite, &ct.Composite); err != nil {
			return types.CustomTool{}, fmt.Errorf("unmarshal composite: %w", err)
		}
	}
	return ct, nil
}
