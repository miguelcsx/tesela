// AgentRunRepo persists types.AgentRun and types.ToolCallTrace entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// AgentRunRepo handles CRUD for agent runs.
type AgentRunRepo struct{ q Querier }

// Create inserts an agent run.
func (r *AgentRunRepo) Create(ctx context.Context, ar types.AgentRun) (types.AgentRun, error) {
	actorRoles, input, plan, contextRefs, memoryRefs, err := marshalAgentRun(ar)
	if err != nil {
		return types.AgentRun{}, err
	}
	const q = `
INSERT INTO agent_runs (id, workspace_id, agent, parent_run_id, actor_user_id, actor_roles, input, plan, context_refs, memory_refs, status)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
RETURNING created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, ar.ID, ar.WorkspaceID, ar.Agent, nullableAgentRunID(ar.ParentRunID), ar.ActorUserID, actorRoles, input, plan, contextRefs, memoryRefs, ar.Status).
		Scan(&ar.CreatedAt, &ar.UpdatedAt); err != nil {
		return types.AgentRun{}, classifyError(err)
	}
	return ar, nil
}

// GetByID returns an agent run by id within a workspace.
func (r *AgentRunRepo) GetByID(ctx context.Context, ws types.WorkspaceID, id types.AgentRunID) (types.AgentRun, error) {
	const q = `
SELECT id, workspace_id, agent, parent_run_id, actor_user_id, actor_roles, input, plan, context_refs, memory_refs, final_response, status, error_code, error_message, tokens_used, tool_call_count, cost_usd, started_at, finished_at, created_at, updated_at
FROM agent_runs WHERE workspace_id = $1 AND id = $2`
	return scanAgentRun(r.q.QueryRow(ctx, q, ws, id))
}

// List returns agent runs for a workspace, newest first.
func (r *AgentRunRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.AgentRun, error) {
	const q = `
SELECT id, workspace_id, agent, parent_run_id, actor_user_id, actor_roles, input, plan, context_refs, memory_refs, final_response, status, error_code, error_message, tokens_used, tool_call_count, cost_usd, started_at, finished_at, created_at, updated_at
FROM agent_runs WHERE workspace_id = $1 ORDER BY created_at DESC`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.AgentRun
	for rows.Next() {
		ar, err := scanAgentRun(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, ar)
	}
	return out, rows.Err()
}

// Update writes mutable fields of an agent run.
func (r *AgentRunRepo) Update(ctx context.Context, ar types.AgentRun) (types.AgentRun, error) {
	const q = `
UPDATE agent_runs SET
    status=$2,
    final_response=$3,
    error_code=$4,
    error_message=$5,
    tokens_used=$6,
    tool_call_count=$7,
    cost_usd=$8,
    started_at=$9,
    finished_at=$10,
    plan=$11,
    context_refs=$12,
    memory_refs=$13,
    updated_at=now()
WHERE id=$1
RETURNING updated_at`
	if err := r.q.QueryRow(ctx, q, ar.ID, ar.Status, ar.FinalResponse, ar.ErrorCode, ar.ErrorMessage,
		ar.TokensUsed, ar.ToolCallCount, ar.CostUSD, ar.StartedAt, ar.FinishedAt, jsonOrEmptyObject(ar.Plan), jsonOrEmptyArray(ar.ContextRefs), jsonOrEmptyArray(ar.MemoryRefs)).
		Scan(&ar.UpdatedAt); err != nil {
		return types.AgentRun{}, classifyError(err)
	}
	return ar, nil
}

// Delete removes an agent run by id.
func (r *AgentRunRepo) Delete(ctx context.Context, ws types.WorkspaceID, id types.AgentRunID) error {
	const q = `DELETE FROM agent_runs WHERE workspace_id = $1 AND id = $2`
	tag, err := r.q.Exec(ctx, q, ws, id)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

// InsertToolCall inserts a tool call trace for an agent run.
func (r *AgentRunRepo) InsertToolCall(ctx context.Context, tc types.ToolCallTrace) error {
	input, output, err := marshalToolCallTrace(tc)
	if err != nil {
		return err
	}
	const q = `
INSERT INTO agent_run_tool_calls (id, agent_run_id, sequence, tool_name, input, output, status, policy_decision, duration_ms, error_message)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`
	_, err = r.q.Exec(ctx, q, tc.ID, tc.AgentRunID, tc.Sequence, tc.ToolName, input, output,
		tc.Status, tc.PolicyDecision, tc.DurationMS, tc.ErrorMessage)
	return classifyError(err)
}

// ListToolCalls returns tool call traces for an agent run, ordered by sequence.
func (r *AgentRunRepo) ListToolCalls(ctx context.Context, runID types.AgentRunID) ([]types.ToolCallTrace, error) {
	const q = `
SELECT id, agent_run_id, sequence, tool_name, input, output, status, policy_decision, duration_ms, error_message, occurred_at
FROM agent_run_tool_calls WHERE agent_run_id = $1 ORDER BY sequence`
	rows, err := r.q.Query(ctx, q, runID)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.ToolCallTrace
	for rows.Next() {
		tc, err := scanToolCallTrace(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, tc)
	}
	return out, rows.Err()
}

// InsertMessage inserts a message or state trace for an agent run.
func (r *AgentRunRepo) InsertMessage(ctx context.Context, msg types.AgentMessageTrace) error {
	metadata := jsonOrEmptyObject(msg.Metadata)
	const q = `
INSERT INTO agent_run_messages (id, agent_run_id, sequence, role, kind, content, name, tool_call_id, metadata)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`
	_, err := r.q.Exec(ctx, q, msg.ID, msg.AgentRunID, msg.Sequence, msg.Role, msg.Kind, msg.Content, msg.Name, msg.ToolCallID, metadata)
	return classifyError(err)
}

// ListMessages returns the ordered agent message trace for a run.
func (r *AgentRunRepo) ListMessages(ctx context.Context, runID types.AgentRunID) ([]types.AgentMessageTrace, error) {
	const q = `
SELECT id, agent_run_id, sequence, role, kind, content, name, tool_call_id, metadata, occurred_at
FROM agent_run_messages WHERE agent_run_id = $1 ORDER BY sequence`
	rows, err := r.q.Query(ctx, q, runID)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.AgentMessageTrace
	for rows.Next() {
		msg, err := scanAgentMessageTrace(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, msg)
	}
	return out, rows.Err()
}

// PutMemory upserts a memory record keyed by id.
func (r *AgentRunRepo) PutMemory(ctx context.Context, rec types.AgentMemoryRecord) (types.AgentMemoryRecord, error) {
	metadata, err := json.Marshal(rec.Metadata)
	if err != nil {
		return types.AgentMemoryRecord{}, fmt.Errorf("marshal memory metadata: %w", err)
	}
	const q = `
INSERT INTO agent_memory_records (id, workspace_id, namespace, scope, actor_user_id, agent, kind, content, summary, metadata)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
ON CONFLICT (id) DO UPDATE SET
    summary=EXCLUDED.summary,
    content=EXCLUDED.content,
    metadata=EXCLUDED.metadata,
    updated_at=now()
RETURNING created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, rec.ID, rec.WorkspaceID, rec.Namespace, rec.Scope, rec.ActorUserID, rec.Agent, rec.Kind, rec.Content, rec.Summary, metadata).
		Scan(&rec.CreatedAt, &rec.UpdatedAt); err != nil {
		return types.AgentMemoryRecord{}, classifyError(err)
	}
	return rec, nil
}

// ListMemory returns memory records for a namespace, newest first.
func (r *AgentRunRepo) ListMemory(ctx context.Context, ws types.WorkspaceID, namespace string, limit int) ([]types.AgentMemoryRecord, error) {
	if limit <= 0 {
		limit = 50
	}
	const q = `
SELECT id, workspace_id, namespace, scope, actor_user_id, agent, kind, content, summary, metadata, created_at, updated_at
FROM agent_memory_records WHERE workspace_id = $1 AND namespace = $2
ORDER BY created_at DESC LIMIT $3`
	rows, err := r.q.Query(ctx, q, ws, namespace, limit)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.AgentMemoryRecord
	for rows.Next() {
		rec, err := scanAgentMemoryRecord(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, rec)
	}
	return out, rows.Err()
}

func marshalAgentRun(ar types.AgentRun) (actorRoles, input, plan, contextRefs, memoryRefs []byte, err error) {
	if actorRoles, err = json.Marshal(ar.ActorRoles); err != nil {
		return nil, nil, nil, nil, nil, fmt.Errorf("marshal actor_roles: %w", err)
	}
	input = []byte(ar.Input)
	plan = jsonOrEmptyObject(ar.Plan)
	contextRefs = jsonOrEmptyArray(ar.ContextRefs)
	memoryRefs = jsonOrEmptyArray(ar.MemoryRefs)
	return actorRoles, input, plan, contextRefs, memoryRefs, nil
}

func scanAgentRun(row rowScanner) (types.AgentRun, error) {
	var ar types.AgentRun
	var actorRoles, input, plan, contextRefs, memoryRefs []byte
	var parentRunID *types.AgentRunID
	var startedAt, finishedAt *time.Time
	if err := row.Scan(&ar.ID, &ar.WorkspaceID, &ar.Agent, &parentRunID, &ar.ActorUserID, &actorRoles,
		&input, &plan, &contextRefs, &memoryRefs, &ar.FinalResponse, &ar.Status, &ar.ErrorCode, &ar.ErrorMessage,
		&ar.TokensUsed, &ar.ToolCallCount, &ar.CostUSD, &startedAt, &finishedAt,
		&ar.CreatedAt, &ar.UpdatedAt); err != nil {
		return types.AgentRun{}, classifyError(err)
	}
	if parentRunID != nil {
		ar.ParentRunID = *parentRunID
	}
	ar.StartedAt = startedAt
	ar.FinishedAt = finishedAt
	if len(actorRoles) > 0 {
		if err := json.Unmarshal(actorRoles, &ar.ActorRoles); err != nil {
			return types.AgentRun{}, fmt.Errorf("unmarshal actor_roles: %w", err)
		}
	}
	if len(input) > 0 {
		ar.Input = json.RawMessage(input)
	}
	if len(plan) > 0 {
		ar.Plan = json.RawMessage(plan)
	}
	if len(contextRefs) > 0 {
		ar.ContextRefs = json.RawMessage(contextRefs)
	}
	if len(memoryRefs) > 0 {
		ar.MemoryRefs = json.RawMessage(memoryRefs)
	}
	return ar, nil
}

func marshalToolCallTrace(tc types.ToolCallTrace) (input, output []byte, err error) {
	input = []byte(tc.Input)
	if tc.Output != nil {
		output = []byte(tc.Output)
	}
	return input, output, nil
}

func scanToolCallTrace(row rowScanner) (types.ToolCallTrace, error) {
	var tc types.ToolCallTrace
	var input, output []byte
	if err := row.Scan(&tc.ID, &tc.AgentRunID, &tc.Sequence, &tc.ToolName,
		&input, &output, &tc.Status, &tc.PolicyDecision, &tc.DurationMS, &tc.ErrorMessage, &tc.OccurredAt); err != nil {
		return types.ToolCallTrace{}, classifyError(err)
	}
	if len(input) > 0 {
		tc.Input = json.RawMessage(input)
	}
	if len(output) > 0 {
		tc.Output = json.RawMessage(output)
	}
	return tc, nil
}

func scanAgentMessageTrace(row rowScanner) (types.AgentMessageTrace, error) {
	var msg types.AgentMessageTrace
	var metadata []byte
	if err := row.Scan(&msg.ID, &msg.AgentRunID, &msg.Sequence, &msg.Role, &msg.Kind, &msg.Content, &msg.Name, &msg.ToolCallID, &metadata, &msg.OccurredAt); err != nil {
		return types.AgentMessageTrace{}, classifyError(err)
	}
	if len(metadata) > 0 {
		msg.Metadata = json.RawMessage(metadata)
	}
	return msg, nil
}

func scanAgentMemoryRecord(row rowScanner) (types.AgentMemoryRecord, error) {
	var rec types.AgentMemoryRecord
	var metadata []byte
	if err := row.Scan(&rec.ID, &rec.WorkspaceID, &rec.Namespace, &rec.Scope, &rec.ActorUserID, &rec.Agent, &rec.Kind, &rec.Content, &rec.Summary, &metadata, &rec.CreatedAt, &rec.UpdatedAt); err != nil {
		return types.AgentMemoryRecord{}, classifyError(err)
	}
	if len(metadata) > 0 {
		if err := json.Unmarshal(metadata, &rec.Metadata); err != nil {
			return types.AgentMemoryRecord{}, fmt.Errorf("unmarshal memory metadata: %w", err)
		}
	}
	return rec, nil
}

func nullableAgentRunID(id types.AgentRunID) any {
	if id == "" {
		return nil
	}
	return id
}

func jsonOrEmptyObject(raw json.RawMessage) []byte {
	if len(raw) == 0 {
		return []byte(`{}`)
	}
	return raw
}

func jsonOrEmptyArray(raw json.RawMessage) []byte {
	if len(raw) == 0 {
		return []byte(`[]`)
	}
	return raw
}
