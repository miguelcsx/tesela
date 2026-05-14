// AuditRecordRepo persists types.AuditRecord entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// AuditRecordRepo handles append-only writes and reads for audit records.
type AuditRecordRepo struct{ q Querier }

// Create inserts an audit record.
func (r *AuditRecordRepo) Create(ctx context.Context, rec types.AuditRecord) (types.AuditRecord, error) {
	actorRoles, matchedRules, redactedProperties, metadata, err := marshalAuditRecord(rec)
	if err != nil {
		return types.AuditRecord{}, err
	}
	const q = `
INSERT INTO audit_records (id, workspace_id, occurred_at, request_id, trace_id, actor_user_id, actor_roles, operation, resource_kind, resource_api_name, subject_key, policy_decision, matched_rules, redacted_properties, result_count, duration_ms, error_code, action_run_id, agent_run_id, metadata)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
RETURNING id, occurred_at`
	if err := r.q.QueryRow(ctx, q, rec.ID, rec.WorkspaceID, rec.OccurredAt, rec.RequestID, rec.TraceID,
		rec.ActorUserID, actorRoles, rec.Operation, rec.ResourceKind, rec.ResourceAPIName, rec.SubjectKey,
		rec.PolicyDecision, matchedRules, redactedProperties, rec.ResultCount, rec.DurationMS, rec.ErrorCode,
		rec.ActionRunID, rec.AgentRunID, metadata).
		Scan(&rec.ID, &rec.OccurredAt); err != nil {
		return types.AuditRecord{}, classifyError(err)
	}
	return rec, nil
}

// List returns audit records for a workspace, newest first.
func (r *AuditRecordRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.AuditRecord, error) {
	const q = `
SELECT id, workspace_id, occurred_at, request_id, trace_id, actor_user_id, actor_roles, operation, resource_kind, resource_api_name, subject_key, policy_decision, matched_rules, redacted_properties, result_count, duration_ms, error_code, action_run_id, agent_run_id, metadata
FROM audit_records WHERE workspace_id = $1 ORDER BY occurred_at DESC`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.AuditRecord
	for rows.Next() {
		rec, err := scanAuditRecord(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, rec)
	}
	return out, rows.Err()
}

func marshalAuditRecord(rec types.AuditRecord) (actorRoles, matchedRules, redactedProperties, metadata []byte, err error) {
	if actorRoles, err = json.Marshal(rec.ActorRoles); err != nil {
		return nil, nil, nil, nil, fmt.Errorf("marshal actor_roles: %w", err)
	}
	if matchedRules, err = json.Marshal(rec.MatchedRules); err != nil {
		return nil, nil, nil, nil, fmt.Errorf("marshal matched_rules: %w", err)
	}
	if redactedProperties, err = json.Marshal(rec.RedactedProperties); err != nil {
		return nil, nil, nil, nil, fmt.Errorf("marshal redacted_properties: %w", err)
	}
	if metadata, err = json.Marshal(rec.Metadata); err != nil {
		return nil, nil, nil, nil, fmt.Errorf("marshal metadata: %w", err)
	}
	return actorRoles, matchedRules, redactedProperties, metadata, nil
}

func scanAuditRecord(row rowScanner) (types.AuditRecord, error) {
	var rec types.AuditRecord
	var actorRoles, matchedRules, redactedProperties, metadata []byte
	if err := row.Scan(&rec.ID, &rec.WorkspaceID, &rec.OccurredAt, &rec.RequestID, &rec.TraceID,
		&rec.ActorUserID, &actorRoles, &rec.Operation, &rec.ResourceKind, &rec.ResourceAPIName, &rec.SubjectKey,
		&rec.PolicyDecision, &matchedRules, &redactedProperties, &rec.ResultCount, &rec.DurationMS, &rec.ErrorCode,
		&rec.ActionRunID, &rec.AgentRunID, &metadata); err != nil {
		return types.AuditRecord{}, classifyError(err)
	}
	if len(actorRoles) > 0 {
		if err := json.Unmarshal(actorRoles, &rec.ActorRoles); err != nil {
			return types.AuditRecord{}, fmt.Errorf("unmarshal actor_roles: %w", err)
		}
	}
	if len(matchedRules) > 0 {
		if err := json.Unmarshal(matchedRules, &rec.MatchedRules); err != nil {
			return types.AuditRecord{}, fmt.Errorf("unmarshal matched_rules: %w", err)
		}
	}
	if len(redactedProperties) > 0 {
		if err := json.Unmarshal(redactedProperties, &rec.RedactedProperties); err != nil {
			return types.AuditRecord{}, fmt.Errorf("unmarshal redacted_properties: %w", err)
		}
	}
	if len(metadata) > 0 {
		if err := json.Unmarshal(metadata, &rec.Metadata); err != nil {
			return types.AuditRecord{}, fmt.Errorf("unmarshal metadata: %w", err)
		}
	}
	return rec, nil
}
