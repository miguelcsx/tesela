// OntologyVersionRepo persists types.OntologyVersion entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// OntologyVersionRepo handles CRUD for ontology versions.
type OntologyVersionRepo struct{ q Querier }

// Create inserts an ontology version.
func (r *OntologyVersionRepo) Create(ctx context.Context, ov types.OntologyVersion) (types.OntologyVersion, error) {
	snapshot, err := json.Marshal(ov.Snapshot)
	if err != nil {
		return types.OntologyVersion{}, fmt.Errorf("marshal snapshot: %w", err)
	}
	const q = `
INSERT INTO ontology_versions (id, workspace_id, name, snapshot, created_by, notes)
VALUES ($1, $2, $3, $4, $5, $6)
RETURNING id, created_at`
	if err := r.q.QueryRow(ctx, q, ov.ID, ov.WorkspaceID, ov.Name, snapshot, ov.CreatedBy, ov.Notes).
		Scan(&ov.ID, &ov.CreatedAt); err != nil {
		return types.OntologyVersion{}, classifyError(err)
	}
	return ov, nil
}

// GetByName returns an ontology version by name within a workspace.
func (r *OntologyVersionRepo) GetByName(ctx context.Context, ws types.WorkspaceID, name string) (types.OntologyVersion, error) {
	const q = `
SELECT id, workspace_id, name, snapshot, created_by, created_at, notes
FROM ontology_versions WHERE workspace_id = $1 AND name = $2`
	return scanOntologyVersion(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every ontology version for a workspace, newest first.
func (r *OntologyVersionRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.OntologyVersion, error) {
	const q = `
SELECT id, workspace_id, name, snapshot, created_by, created_at, notes
FROM ontology_versions WHERE workspace_id = $1 ORDER BY created_at DESC`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.OntologyVersion
	for rows.Next() {
		ov, err := scanOntologyVersion(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, ov)
	}
	return out, rows.Err()
}

// Delete removes an ontology version by name.
func (r *OntologyVersionRepo) Delete(ctx context.Context, ws types.WorkspaceID, name string) error {
	const q = `DELETE FROM ontology_versions WHERE workspace_id = $1 AND name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func scanOntologyVersion(row rowScanner) (types.OntologyVersion, error) {
	var ov types.OntologyVersion
	var snapshot []byte
	if err := row.Scan(&ov.ID, &ov.WorkspaceID, &ov.Name, &snapshot, &ov.CreatedBy, &ov.CreatedAt, &ov.Notes); err != nil {
		return types.OntologyVersion{}, classifyError(err)
	}
	if len(snapshot) > 0 {
		if err := json.Unmarshal(snapshot, &ov.Snapshot); err != nil {
			return types.OntologyVersion{}, fmt.Errorf("unmarshal snapshot: %w", err)
		}
	}
	return ov, nil
}
