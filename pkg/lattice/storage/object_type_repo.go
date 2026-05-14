// ObjectTypeRepo persists types.ObjectType. Properties are stored as a JSONB
// array on the object_types row to keep updates atomic.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// ObjectTypeRepo handles CRUD for object types.
type ObjectTypeRepo struct{ q Querier }

// Upsert inserts or replaces an object type by (workspace_id, api_name) and
// returns the row with timestamps populated.
func (r *ObjectTypeRepo) Upsert(ctx context.Context, ot types.ObjectType) (types.ObjectType, error) {
	src, props, envs, err := marshalObjectType(ot)
	if err != nil {
		return types.ObjectType{}, err
	}
	const q = `
INSERT INTO object_types (id, workspace_id, api_name, display_name, description, primary_key, source, properties, environments, version, deprecated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
ON CONFLICT (workspace_id, api_name) DO UPDATE SET
    display_name=EXCLUDED.display_name,
    description=EXCLUDED.description,
    primary_key=EXCLUDED.primary_key,
    source=EXCLUDED.source,
    properties=EXCLUDED.properties,
    environments=EXCLUDED.environments,
    version=object_types.version + 1,
    deprecated_at=EXCLUDED.deprecated_at,
    updated_at=now()
RETURNING id, version, created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, ot.ID, ot.WorkspaceID, ot.APIName, ot.DisplayName, ot.Description,
		ot.PrimaryKey, src, props, envs, ot.Version, ot.DeprecatedAt).
		Scan(&ot.ID, &ot.Version, &ot.CreatedAt, &ot.UpdatedAt); err != nil {
		return types.ObjectType{}, classifyError(err)
	}
	return ot, nil
}

// GetByAPIName returns the object type with the given api_name.
func (r *ObjectTypeRepo) GetByAPIName(ctx context.Context, ws types.WorkspaceID, name types.APIName) (types.ObjectType, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, primary_key, source, properties, environments, version, deprecated_at, created_at, updated_at
FROM object_types WHERE workspace_id = $1 AND api_name = $2`
	return scanObjectType(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every object type for a workspace.
func (r *ObjectTypeRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.ObjectType, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, primary_key, source, properties, environments, version, deprecated_at, created_at, updated_at
FROM object_types WHERE workspace_id = $1 ORDER BY api_name`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.ObjectType
	for rows.Next() {
		ot, err := scanObjectType(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, ot)
	}
	return out, rows.Err()
}

// Delete removes an object type by api_name.
func (r *ObjectTypeRepo) Delete(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	const q = `DELETE FROM object_types WHERE workspace_id = $1 AND api_name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func marshalObjectType(ot types.ObjectType) (src, props, envs []byte, err error) {
	if src, err = json.Marshal(ot.Source); err != nil {
		return nil, nil, nil, fmt.Errorf("marshal source: %w", err)
	}
	if props, err = json.Marshal(ot.Properties); err != nil {
		return nil, nil, nil, fmt.Errorf("marshal properties: %w", err)
	}
	if envs, err = json.Marshal(ot.Environments); err != nil {
		return nil, nil, nil, fmt.Errorf("marshal environments: %w", err)
	}
	return src, props, envs, nil
}

func scanObjectType(row rowScanner) (types.ObjectType, error) {
	var ot types.ObjectType
	var src, props, envs []byte
	if err := row.Scan(&ot.ID, &ot.WorkspaceID, &ot.APIName, &ot.DisplayName, &ot.Description,
		&ot.PrimaryKey, &src, &props, &envs, &ot.Version, &ot.DeprecatedAt,
		&ot.CreatedAt, &ot.UpdatedAt); err != nil {
		return types.ObjectType{}, classifyError(err)
	}
	if err := json.Unmarshal(src, &ot.Source); err != nil {
		return types.ObjectType{}, fmt.Errorf("unmarshal source: %w", err)
	}
	if len(props) > 0 {
		if err := json.Unmarshal(props, &ot.Properties); err != nil {
			return types.ObjectType{}, fmt.Errorf("unmarshal properties: %w", err)
		}
	}
	if len(envs) > 0 {
		if err := json.Unmarshal(envs, &ot.Environments); err != nil {
			return types.ObjectType{}, fmt.Errorf("unmarshal environments: %w", err)
		}
	}
	return ot, nil
}
