// AssetRepo persists types.Asset entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// AssetRepo handles CRUD for assets.
type AssetRepo struct{ q Querier }

// Upsert inserts or replaces an asset by (workspace_id, api_name).
func (r *AssetRepo) Upsert(ctx context.Context, a types.Asset) (types.Asset, error) {
	metadata, tags, properties, qualityRules, dependencies, sink, savedColumnMapping, err := marshalAsset(a)
	if err != nil {
		return types.Asset{}, err
	}
	const q = `
INSERT INTO assets (id, workspace_id, api_name, display_name, description, metadata, tags, properties, quality_rules, dependencies, sink, saved_column_mapping, unmapped_column_policy)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
ON CONFLICT (workspace_id, api_name) DO UPDATE SET
    display_name=EXCLUDED.display_name,
    description=EXCLUDED.description,
    metadata=EXCLUDED.metadata,
    tags=EXCLUDED.tags,
    properties=EXCLUDED.properties,
    quality_rules=EXCLUDED.quality_rules,
    dependencies=EXCLUDED.dependencies,
    sink=EXCLUDED.sink,
    saved_column_mapping=EXCLUDED.saved_column_mapping,
    unmapped_column_policy=EXCLUDED.unmapped_column_policy,
    updated_at=now()
RETURNING id, created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, a.ID, a.WorkspaceID, a.APIName, a.DisplayName, a.Description,
		metadata, tags, properties, qualityRules, dependencies, sink, savedColumnMapping, a.UnmappedColumnPolicy).
		Scan(&a.ID, &a.CreatedAt, &a.UpdatedAt); err != nil {
		return types.Asset{}, classifyError(err)
	}
	return a, nil
}

// GetByAPIName returns the asset with the given api_name.
func (r *AssetRepo) GetByAPIName(ctx context.Context, ws types.WorkspaceID, name types.APIName) (types.Asset, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, metadata, tags, properties, quality_rules, dependencies, sink, saved_column_mapping, unmapped_column_policy, created_at, updated_at
FROM assets WHERE workspace_id = $1 AND api_name = $2`
	return scanAsset(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every asset for a workspace.
func (r *AssetRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.Asset, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, metadata, tags, properties, quality_rules, dependencies, sink, saved_column_mapping, unmapped_column_policy, created_at, updated_at
FROM assets WHERE workspace_id = $1 ORDER BY api_name`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.Asset
	for rows.Next() {
		a, err := scanAsset(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, a)
	}
	return out, rows.Err()
}

// Delete removes an asset by api_name.
func (r *AssetRepo) Delete(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	const q = `DELETE FROM assets WHERE workspace_id = $1 AND api_name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func marshalAsset(a types.Asset) (metadata, tags, properties, qualityRules, dependencies, sink, savedColumnMapping []byte, err error) {
	if metadata, err = json.Marshal(a.Metadata); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal metadata: %w", err)
	}
	if tags, err = json.Marshal(a.Tags); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal tags: %w", err)
	}
	if properties, err = json.Marshal(a.Properties); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal properties: %w", err)
	}
	if qualityRules, err = json.Marshal(a.QualityRules); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal quality_rules: %w", err)
	}
	if dependencies, err = json.Marshal(a.Dependencies); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal dependencies: %w", err)
	}
	if sink, err = json.Marshal(a.Sink); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal sink: %w", err)
	}
	if savedColumnMapping, err = json.Marshal(a.SavedColumnMapping); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal saved_column_mapping: %w", err)
	}
	return metadata, tags, properties, qualityRules, dependencies, sink, savedColumnMapping, nil
}

func scanAsset(row rowScanner) (types.Asset, error) {
	var a types.Asset
	var metadata, tags, properties, qualityRules, dependencies, sink, savedColumnMapping []byte
	if err := row.Scan(&a.ID, &a.WorkspaceID, &a.APIName, &a.DisplayName, &a.Description,
		&metadata, &tags, &properties, &qualityRules, &dependencies, &sink, &savedColumnMapping, &a.UnmappedColumnPolicy,
		&a.CreatedAt, &a.UpdatedAt); err != nil {
		return types.Asset{}, classifyError(err)
	}
	if len(metadata) > 0 {
		if err := json.Unmarshal(metadata, &a.Metadata); err != nil {
			return types.Asset{}, fmt.Errorf("unmarshal metadata: %w", err)
		}
	}
	if len(tags) > 0 {
		if err := json.Unmarshal(tags, &a.Tags); err != nil {
			return types.Asset{}, fmt.Errorf("unmarshal tags: %w", err)
		}
	}
	if len(properties) > 0 {
		if err := json.Unmarshal(properties, &a.Properties); err != nil {
			return types.Asset{}, fmt.Errorf("unmarshal properties: %w", err)
		}
	}
	if len(qualityRules) > 0 {
		if err := json.Unmarshal(qualityRules, &a.QualityRules); err != nil {
			return types.Asset{}, fmt.Errorf("unmarshal quality_rules: %w", err)
		}
	}
	if len(dependencies) > 0 {
		if err := json.Unmarshal(dependencies, &a.Dependencies); err != nil {
			return types.Asset{}, fmt.Errorf("unmarshal dependencies: %w", err)
		}
	}
	if len(sink) > 0 {
		if err := json.Unmarshal(sink, &a.Sink); err != nil {
			return types.Asset{}, fmt.Errorf("unmarshal sink: %w", err)
		}
	}
	if len(savedColumnMapping) > 0 {
		if err := json.Unmarshal(savedColumnMapping, &a.SavedColumnMapping); err != nil {
			return types.Asset{}, fmt.Errorf("unmarshal saved_column_mapping: %w", err)
		}
	}
	return a, nil
}
