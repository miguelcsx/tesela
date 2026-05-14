// UploadRepo persists types.Upload entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// UploadRepo handles CRUD for uploads.
type UploadRepo struct{ q Querier }

// Create inserts an upload.
func (r *UploadRepo) Create(ctx context.Context, u types.Upload) (types.Upload, error) {
	discoveredSchema, columnMapping, proposedMapping, modelConfig, metadata, err := marshalUpload(u)
	if err != nil {
		return types.Upload{}, err
	}
	const q = `
INSERT INTO uploads (id, workspace_id, asset, status, storage_url, signed_url, signed_url_expires, discovered_schema, column_mapping, proposed_column_mapping, mapping_confidence, mapping_proposed_at, mapping_model_config, error_report_url, error_message, actor_user_id, metadata)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
RETURNING created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, u.ID, u.WorkspaceID, u.Asset, u.Status, u.StorageURL, u.SignedURL,
		u.SignedURLExpires, discoveredSchema, columnMapping, proposedMapping, u.MappingConfidence, u.MappingProposedAt, modelConfig, u.ErrorReportURL, u.ErrorMessage, u.ActorUserID, metadata).
		Scan(&u.CreatedAt, &u.UpdatedAt); err != nil {
		return types.Upload{}, classifyError(err)
	}
	return u, nil
}

// GetByID returns an upload within a workspace.
func (r *UploadRepo) GetByID(ctx context.Context, ws types.WorkspaceID, id types.UploadID) (types.Upload, error) {
	const q = `
SELECT id, workspace_id, asset, status, storage_url, signed_url, signed_url_expires, discovered_schema, column_mapping, proposed_column_mapping, mapping_confidence, mapping_proposed_at, mapping_model_config, error_report_url, error_message, actor_user_id, metadata, created_at, updated_at
FROM uploads WHERE workspace_id = $1 AND id = $2`
	return scanUpload(r.q.QueryRow(ctx, q, ws, id))
}

// List returns every upload for a workspace, newest first.
func (r *UploadRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.Upload, error) {
	const q = `
SELECT id, workspace_id, asset, status, storage_url, signed_url, signed_url_expires, discovered_schema, column_mapping, proposed_column_mapping, mapping_confidence, mapping_proposed_at, mapping_model_config, error_report_url, error_message, actor_user_id, metadata, created_at, updated_at
FROM uploads WHERE workspace_id = $1 ORDER BY created_at DESC`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.Upload
	for rows.Next() {
		u, err := scanUpload(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, u)
	}
	return out, rows.Err()
}

// Update writes mutable fields of an upload.
func (r *UploadRepo) Update(ctx context.Context, u types.Upload) (types.Upload, error) {
	discoveredSchema, columnMapping, proposedMapping, modelConfig, metadata, err := marshalUpload(u)
	if err != nil {
		return types.Upload{}, err
	}
	const q = `
UPDATE uploads SET
    status=$2,
    storage_url=$3,
    signed_url=$4,
    signed_url_expires=$5,
    discovered_schema=$6,
    column_mapping=$7,
    proposed_column_mapping=$8,
    mapping_confidence=$9,
    mapping_proposed_at=$10,
    mapping_model_config=$11,
    error_report_url=$12,
    error_message=$13,
    metadata=$14,
    updated_at=now()
WHERE id=$1
RETURNING updated_at`
	if err := r.q.QueryRow(ctx, q, u.ID, u.Status, u.StorageURL, u.SignedURL,
		u.SignedURLExpires, discoveredSchema, columnMapping, proposedMapping, u.MappingConfidence, u.MappingProposedAt, modelConfig, u.ErrorReportURL, u.ErrorMessage, metadata).
		Scan(&u.UpdatedAt); err != nil {
		return types.Upload{}, classifyError(err)
	}
	return u, nil
}

// ListByStatus returns uploads in the given status, newest first.
func (r *UploadRepo) ListByStatus(ctx context.Context, status types.UploadStatus, limit int) ([]types.Upload, error) {
	const q = `
SELECT id, workspace_id, asset, status, storage_url, signed_url, signed_url_expires, discovered_schema, column_mapping, proposed_column_mapping, mapping_confidence, mapping_proposed_at, mapping_model_config, error_report_url, error_message, actor_user_id, metadata, created_at, updated_at
FROM uploads WHERE status = $1 ORDER BY created_at DESC LIMIT $2`
	rows, err := r.q.Query(ctx, q, status, limit)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.Upload
	for rows.Next() {
		u, err := scanUpload(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, u)
	}
	return out, rows.Err()
}

// Delete removes an upload by id.
func (r *UploadRepo) Delete(ctx context.Context, ws types.WorkspaceID, id types.UploadID) error {
	const q = `DELETE FROM uploads WHERE workspace_id = $1 AND id = $2`
	tag, err := r.q.Exec(ctx, q, ws, id)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func marshalUpload(u types.Upload) (discoveredSchema, columnMapping, proposedMapping, modelConfig, metadata []byte, err error) {
	if u.DiscoveredSchema != nil {
		if discoveredSchema, err = json.Marshal(u.DiscoveredSchema); err != nil {
			return nil, nil, nil, nil, nil, fmt.Errorf("marshal discovered_schema: %w", err)
		}
	}
	if columnMapping, err = json.Marshal(u.ColumnMapping); err != nil {
		return nil, nil, nil, nil, nil, fmt.Errorf("marshal column_mapping: %w", err)
	}
	if proposedMapping, err = json.Marshal(u.ProposedColumnMapping); err != nil {
		return nil, nil, nil, nil, nil, fmt.Errorf("marshal proposed_column_mapping: %w", err)
	}
	if u.MappingModelConfig != nil {
		if modelConfig, err = json.Marshal(u.MappingModelConfig); err != nil {
			return nil, nil, nil, nil, nil, fmt.Errorf("marshal mapping_model_config: %w", err)
		}
	}
	if u.Metadata != nil {
		if metadata, err = json.Marshal(u.Metadata); err != nil {
			return nil, nil, nil, nil, nil, fmt.Errorf("marshal metadata: %w", err)
		}
	}
	return discoveredSchema, columnMapping, proposedMapping, modelConfig, metadata, nil
}

func scanUpload(row rowScanner) (types.Upload, error) {
	var u types.Upload
	var discoveredSchema, columnMapping, proposedMapping, modelConfig, metadata []byte
	var signedURLExpires *time.Time
	if err := row.Scan(&u.ID, &u.WorkspaceID, &u.Asset, &u.Status, &u.StorageURL, &u.SignedURL,
		&signedURLExpires, &discoveredSchema, &columnMapping, &proposedMapping, &u.MappingConfidence, &u.MappingProposedAt, &modelConfig, &u.ErrorReportURL, &u.ErrorMessage, &u.ActorUserID, &metadata,
		&u.CreatedAt, &u.UpdatedAt); err != nil {
		return types.Upload{}, classifyError(err)
	}
	u.SignedURLExpires = signedURLExpires
	if len(discoveredSchema) > 0 {
		if err := json.Unmarshal(discoveredSchema, &u.DiscoveredSchema); err != nil {
			return types.Upload{}, fmt.Errorf("unmarshal discovered_schema: %w", err)
		}
	}
	if len(columnMapping) > 0 {
		if err := json.Unmarshal(columnMapping, &u.ColumnMapping); err != nil {
			return types.Upload{}, fmt.Errorf("unmarshal column_mapping: %w", err)
		}
	}
	if len(proposedMapping) > 0 {
		if err := json.Unmarshal(proposedMapping, &u.ProposedColumnMapping); err != nil {
			return types.Upload{}, fmt.Errorf("unmarshal proposed_column_mapping: %w", err)
		}
	}
	if len(modelConfig) > 0 {
		if err := json.Unmarshal(modelConfig, &u.MappingModelConfig); err != nil {
			return types.Upload{}, fmt.Errorf("unmarshal mapping_model_config: %w", err)
		}
	}
	if len(metadata) > 0 {
		if err := json.Unmarshal(metadata, &u.Metadata); err != nil {
			return types.Upload{}, fmt.Errorf("unmarshal metadata: %w", err)
		}
	}
	return u, nil
}
