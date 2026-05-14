// AssetVersionRepo persists types.AssetVersion entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// AssetVersionRepo handles CRUD for asset versions.
type AssetVersionRepo struct{ q Querier }

// Create inserts an asset version.
func (r *AssetVersionRepo) Create(ctx context.Context, v types.AssetVersion) (types.AssetVersion, error) {
	lineage, metadata, err := marshalAssetVersion(v)
	if err != nil {
		return types.AssetVersion{}, err
	}
	const q = `
INSERT INTO asset_versions (id, workspace_id, asset_id, upload_id, row_count, status, lineage, metadata, committed_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
RETURNING created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, v.ID, v.WorkspaceID, v.AssetID, v.UploadID, v.RowCount, v.Status, lineage, metadata, v.Committed).
		Scan(&v.CreatedAt, &v.UpdatedAt); err != nil {
		return types.AssetVersion{}, classifyError(err)
	}
	return v, nil
}

// GetByID returns an asset version by id within a workspace.
func (r *AssetVersionRepo) GetByID(ctx context.Context, ws types.WorkspaceID, id string) (types.AssetVersion, error) {
	const q = `
SELECT id, workspace_id, asset_id, upload_id, row_count, status, lineage, metadata, committed_at, created_at, updated_at
FROM asset_versions WHERE workspace_id = $1 AND id = $2`
	return scanAssetVersion(r.q.QueryRow(ctx, q, ws, id))
}

// ListByAsset returns all versions for an asset, newest first.
func (r *AssetVersionRepo) ListByAsset(ctx context.Context, ws types.WorkspaceID, asset types.AssetID) ([]types.AssetVersion, error) {
	const q = `
SELECT id, workspace_id, asset_id, upload_id, row_count, status, lineage, metadata, committed_at, created_at, updated_at
FROM asset_versions WHERE workspace_id = $1 AND asset_id = $2 ORDER BY created_at DESC`
	rows, err := r.q.Query(ctx, q, ws, asset)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.AssetVersion
	for rows.Next() {
		v, err := scanAssetVersion(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, v)
	}
	return out, rows.Err()
}

// GetLatestByAsset returns the most recent version for an asset.
func (r *AssetVersionRepo) GetLatestByAsset(ctx context.Context, ws types.WorkspaceID, asset types.AssetID) (types.AssetVersion, error) {
	const q = `
SELECT id, workspace_id, asset_id, upload_id, row_count, status, lineage, metadata, committed_at, created_at, updated_at
FROM asset_versions WHERE workspace_id = $1 AND asset_id = $2 ORDER BY created_at DESC LIMIT 1`
	return scanAssetVersion(r.q.QueryRow(ctx, q, ws, asset))
}

// UpdateStatus changes the status of a version (e.g., staging → published).
func (r *AssetVersionRepo) UpdateStatus(ctx context.Context, ws types.WorkspaceID, id string, status string) error {
	const q = `UPDATE asset_versions SET status=$3, updated_at=now() WHERE workspace_id=$1 AND id=$2`
	_, err := r.q.Exec(ctx, q, ws, id, status)
	return classifyError(err)
}

func marshalAssetVersion(v types.AssetVersion) (lineage, metadata []byte, err error) {
	if v.Lineage != nil {
		if lineage, err = json.Marshal(v.Lineage); err != nil {
			return nil, nil, fmt.Errorf("marshal lineage: %w", err)
		}
	}
	if v.Metadata != nil {
		if metadata, err = json.Marshal(v.Metadata); err != nil {
			return nil, nil, fmt.Errorf("marshal metadata: %w", err)
		}
	}
	return lineage, metadata, nil
}

func scanAssetVersion(row rowScanner) (types.AssetVersion, error) {
	var v types.AssetVersion
	var lineage, metadata []byte
	if err := row.Scan(&v.ID, &v.WorkspaceID, &v.AssetID, &v.UploadID, &v.RowCount, &v.Status,
		&lineage, &metadata, &v.Committed, &v.CreatedAt, &v.UpdatedAt); err != nil {
		return types.AssetVersion{}, classifyError(err)
	}
	if len(lineage) > 0 {
		if err := json.Unmarshal(lineage, &v.Lineage); err != nil {
			return types.AssetVersion{}, fmt.Errorf("unmarshal lineage: %w", err)
		}
	}
	if len(metadata) > 0 {
		if err := json.Unmarshal(metadata, &v.Metadata); err != nil {
			return types.AssetVersion{}, fmt.Errorf("unmarshal metadata: %w", err)
		}
	}
	return v, nil
}
