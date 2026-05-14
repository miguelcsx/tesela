// LinkTypeRepo persists types.LinkType entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// LinkTypeRepo handles CRUD for link types.
type LinkTypeRepo struct{ q Querier }

// Upsert inserts or replaces a link type by (workspace_id, api_name).
func (r *LinkTypeRepo) Upsert(ctx context.Context, lt types.LinkType) (types.LinkType, error) {
	mappings, junction, err := marshalLinkType(lt)
	if err != nil {
		return types.LinkType{}, err
	}
	const q = `
INSERT INTO link_types (id, workspace_id, api_name, display_name, from_object_type, to_object_type, cardinality, property_mappings, junction, version, deprecated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
ON CONFLICT (workspace_id, api_name) DO UPDATE SET
    display_name=EXCLUDED.display_name,
    from_object_type=EXCLUDED.from_object_type,
    to_object_type=EXCLUDED.to_object_type,
    cardinality=EXCLUDED.cardinality,
    property_mappings=EXCLUDED.property_mappings,
    junction=EXCLUDED.junction,
    version=link_types.version + 1,
    deprecated_at=EXCLUDED.deprecated_at,
    updated_at=now()
RETURNING id, version, created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, lt.ID, lt.WorkspaceID, lt.APIName, lt.DisplayName,
		lt.FromObjectType, lt.ToObjectType, lt.Cardinality, mappings, junction,
		lt.Version, lt.DeprecatedAt).
		Scan(&lt.ID, &lt.Version, &lt.CreatedAt, &lt.UpdatedAt); err != nil {
		return types.LinkType{}, classifyError(err)
	}
	return lt, nil
}

// GetByAPIName returns the link type with the given api_name.
func (r *LinkTypeRepo) GetByAPIName(ctx context.Context, ws types.WorkspaceID, name types.APIName) (types.LinkType, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, from_object_type, to_object_type, cardinality, property_mappings, junction, version, deprecated_at, created_at, updated_at
FROM link_types WHERE workspace_id = $1 AND api_name = $2`
	return scanLinkType(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every link type for a workspace.
func (r *LinkTypeRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.LinkType, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, from_object_type, to_object_type, cardinality, property_mappings, junction, version, deprecated_at, created_at, updated_at
FROM link_types WHERE workspace_id = $1 ORDER BY api_name`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.LinkType
	for rows.Next() {
		lt, err := scanLinkType(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, lt)
	}
	return out, rows.Err()
}

// Delete removes a link type by api_name.
func (r *LinkTypeRepo) Delete(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	const q = `DELETE FROM link_types WHERE workspace_id = $1 AND api_name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func marshalLinkType(lt types.LinkType) (mappings, junction []byte, err error) {
	if mappings, err = json.Marshal(lt.PropertyMappings); err != nil {
		return nil, nil, fmt.Errorf("marshal property_mappings: %w", err)
	}
	if lt.Junction != nil {
		if junction, err = json.Marshal(lt.Junction); err != nil {
			return nil, nil, fmt.Errorf("marshal junction: %w", err)
		}
	}
	return mappings, junction, nil
}

func scanLinkType(row rowScanner) (types.LinkType, error) {
	var lt types.LinkType
	var mappings, junction []byte
	if err := row.Scan(&lt.ID, &lt.WorkspaceID, &lt.APIName, &lt.DisplayName,
		&lt.FromObjectType, &lt.ToObjectType, &lt.Cardinality, &mappings, &junction,
		&lt.Version, &lt.DeprecatedAt, &lt.CreatedAt, &lt.UpdatedAt); err != nil {
		return types.LinkType{}, classifyError(err)
	}
	if len(mappings) > 0 {
		if err := json.Unmarshal(mappings, &lt.PropertyMappings); err != nil {
			return types.LinkType{}, fmt.Errorf("unmarshal property_mappings: %w", err)
		}
	}
	if len(junction) > 0 {
		var j types.JunctionConfig
		if err := json.Unmarshal(junction, &j); err != nil {
			return types.LinkType{}, fmt.Errorf("unmarshal junction: %w", err)
		}
		lt.Junction = &j
	}
	return lt, nil
}
