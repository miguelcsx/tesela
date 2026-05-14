// ActionRunRepo persists types.ActionRun entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// ActionRunRepo handles CRUD for action runs.
type ActionRunRepo struct{ q Querier }

// Create inserts an action run.
func (r *ActionRunRepo) Create(ctx context.Context, ar types.ActionRun) (types.ActionRun, error) {
	actorRoles, input, err := marshalActionRun(ar)
	if err != nil {
		return types.ActionRun{}, err
	}
	const q = `
INSERT INTO action_runs (id, workspace_id, action_type, idempotency_key, subject, actor_user_id, actor_roles, input, status)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
RETURNING created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, ar.ID, ar.WorkspaceID, ar.ActionType, ar.IdempotencyKey, ar.Subject,
		ar.ActorUserID, actorRoles, input, ar.Status).
		Scan(&ar.CreatedAt, &ar.UpdatedAt); err != nil {
		return types.ActionRun{}, classifyError(err)
	}
	return ar, nil
}

// GetByID returns an action run by id within a workspace.
func (r *ActionRunRepo) GetByID(ctx context.Context, ws types.WorkspaceID, id types.ActionRunID) (types.ActionRun, error) {
	const q = `
SELECT id, workspace_id, action_type, idempotency_key, subject, actor_user_id, actor_roles, input, output, status, error_code, error_message, started_at, finished_at, created_at, updated_at
FROM action_runs WHERE workspace_id = $1 AND id = $2`
	return scanActionRun(r.q.QueryRow(ctx, q, ws, id))
}

// GetByIdempotencyKey returns an action run by its idempotency key.
func (r *ActionRunRepo) GetByIdempotencyKey(ctx context.Context, ws types.WorkspaceID, key string) (types.ActionRun, error) {
	const q = `
SELECT id, workspace_id, action_type, idempotency_key, subject, actor_user_id, actor_roles, input, output, status, error_code, error_message, started_at, finished_at, created_at, updated_at
FROM action_runs WHERE workspace_id = $1 AND idempotency_key = $2`
	return scanActionRun(r.q.QueryRow(ctx, q, ws, key))
}

// List returns action runs for a workspace, newest first.
func (r *ActionRunRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.ActionRun, error) {
	const q = `
SELECT id, workspace_id, action_type, idempotency_key, subject, actor_user_id, actor_roles, input, output, status, error_code, error_message, started_at, finished_at, created_at, updated_at
FROM action_runs WHERE workspace_id = $1 ORDER BY created_at DESC`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.ActionRun
	for rows.Next() {
		ar, err := scanActionRun(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, ar)
	}
	return out, rows.Err()
}

// ListPending returns the oldest action runs still in 'pending' status,
// across every workspace. Used by the worker poller.
func (r *ActionRunRepo) ListPending(ctx context.Context, batch int) ([]types.ActionRun, error) {
	if batch <= 0 {
		batch = 16
	}
	const q = `
SELECT id, workspace_id, action_type, idempotency_key, subject, actor_user_id, actor_roles, input, output, status, error_code, error_message, started_at, finished_at, created_at, updated_at
FROM action_runs WHERE status = 'pending' ORDER BY created_at ASC LIMIT $1`
	rows, err := r.q.Query(ctx, q, batch)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.ActionRun
	for rows.Next() {
		ar, err := scanActionRun(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, ar)
	}
	return out, rows.Err()
}

// Update writes mutable fields of an action run (state machine transition).
func (r *ActionRunRepo) Update(ctx context.Context, ar types.ActionRun) (types.ActionRun, error) {
	var output []byte
	if ar.Output != nil {
		var err error
		if output, err = json.Marshal(ar.Output); err != nil {
			return types.ActionRun{}, fmt.Errorf("marshal output: %w", err)
		}
	}
	const q = `
UPDATE action_runs SET
    status=$2,
    output=$3,
    error_code=$4,
    error_message=$5,
    started_at=$6,
    finished_at=$7,
    updated_at=now()
WHERE id=$1
RETURNING updated_at`
	if err := r.q.QueryRow(ctx, q, ar.ID, ar.Status, output, ar.ErrorCode, ar.ErrorMessage,
		ar.StartedAt, ar.FinishedAt).
		Scan(&ar.UpdatedAt); err != nil {
		return types.ActionRun{}, classifyError(err)
	}
	return ar, nil
}

// Delete removes an action run by id.
func (r *ActionRunRepo) Delete(ctx context.Context, ws types.WorkspaceID, id types.ActionRunID) error {
	const q = `DELETE FROM action_runs WHERE workspace_id = $1 AND id = $2`
	tag, err := r.q.Exec(ctx, q, ws, id)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func marshalActionRun(ar types.ActionRun) (actorRoles, input []byte, err error) {
	if actorRoles, err = json.Marshal(ar.ActorRoles); err != nil {
		return nil, nil, fmt.Errorf("marshal actor_roles: %w", err)
	}
	input = []byte(ar.Input)
	return actorRoles, input, nil
}

func scanActionRun(row rowScanner) (types.ActionRun, error) {
	var ar types.ActionRun
	var actorRoles, input, output []byte
	var startedAt, finishedAt *time.Time
	if err := row.Scan(&ar.ID, &ar.WorkspaceID, &ar.ActionType, &ar.IdempotencyKey, &ar.Subject,
		&ar.ActorUserID, &actorRoles, &input, &output, &ar.Status, &ar.ErrorCode, &ar.ErrorMessage,
		&startedAt, &finishedAt, &ar.CreatedAt, &ar.UpdatedAt); err != nil {
		return types.ActionRun{}, classifyError(err)
	}
	ar.StartedAt = startedAt
	ar.FinishedAt = finishedAt
	if len(actorRoles) > 0 {
		if err := json.Unmarshal(actorRoles, &ar.ActorRoles); err != nil {
			return types.ActionRun{}, fmt.Errorf("unmarshal actor_roles: %w", err)
		}
	}
	if len(input) > 0 {
		ar.Input = json.RawMessage(input)
	}
	if len(output) > 0 {
		ar.Output = json.RawMessage(output)
	}
	return ar, nil
}
