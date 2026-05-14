// PolicyRuleRepo persists types.PolicyRule entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// PolicyRuleRepo handles CRUD for policy rules.
type PolicyRuleRepo struct{ q Querier }

// Upsert inserts or replaces a policy rule by (workspace_id, api_name).
func (r *PolicyRuleRepo) Upsert(ctx context.Context, pr types.PolicyRule) (types.PolicyRule, error) {
	roles, operations, rowFilter, conditions, redactions, err := marshalPolicyRule(pr)
	if err != nil {
		return types.PolicyRule{}, err
	}
	const q = `
INSERT INTO policy_rules (id, workspace_id, api_name, display_name, description, effect, roles, operations, object_type, action_type, row_filter, conditions, redactions, priority)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
ON CONFLICT (workspace_id, api_name) DO UPDATE SET
    display_name=EXCLUDED.display_name,
    description=EXCLUDED.description,
    effect=EXCLUDED.effect,
    roles=EXCLUDED.roles,
    operations=EXCLUDED.operations,
    object_type=EXCLUDED.object_type,
    action_type=EXCLUDED.action_type,
    row_filter=EXCLUDED.row_filter,
    conditions=EXCLUDED.conditions,
    redactions=EXCLUDED.redactions,
    priority=EXCLUDED.priority,
    updated_at=now()
RETURNING id, created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, pr.ID, pr.WorkspaceID, pr.APIName, pr.DisplayName, pr.Description,
		pr.Effect, roles, operations, pr.ObjectType, pr.ActionType, rowFilter, conditions, redactions, pr.Priority).
		Scan(&pr.ID, &pr.CreatedAt, &pr.UpdatedAt); err != nil {
		return types.PolicyRule{}, classifyError(err)
	}
	return pr, nil
}

// GetByAPIName returns the policy rule with the given api_name.
func (r *PolicyRuleRepo) GetByAPIName(ctx context.Context, ws types.WorkspaceID, name types.APIName) (types.PolicyRule, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, effect, roles, operations, object_type, action_type, row_filter, conditions, redactions, priority, created_at, updated_at
FROM policy_rules WHERE workspace_id = $1 AND api_name = $2`
	return scanPolicyRule(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every policy rule for a workspace.
func (r *PolicyRuleRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.PolicyRule, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, effect, roles, operations, object_type, action_type, row_filter, conditions, redactions, priority, created_at, updated_at
FROM policy_rules WHERE workspace_id = $1 ORDER BY api_name`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.PolicyRule
	for rows.Next() {
		pr, err := scanPolicyRule(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, pr)
	}
	return out, rows.Err()
}

// Delete removes a policy rule by api_name.
func (r *PolicyRuleRepo) Delete(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	const q = `DELETE FROM policy_rules WHERE workspace_id = $1 AND api_name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func marshalPolicyRule(pr types.PolicyRule) (roles, operations, rowFilter, conditions, redactions []byte, err error) {
	if roles, err = json.Marshal(pr.Roles); err != nil {
		return nil, nil, nil, nil, nil, fmt.Errorf("marshal roles: %w", err)
	}
	if operations, err = json.Marshal(pr.Operations); err != nil {
		return nil, nil, nil, nil, nil, fmt.Errorf("marshal operations: %w", err)
	}
	if !pr.RowFilter.IsZero() {
		if rowFilter, err = json.Marshal(pr.RowFilter); err != nil {
			return nil, nil, nil, nil, nil, fmt.Errorf("marshal row_filter: %w", err)
		}
	}
	if conditions, err = json.Marshal(pr.Conditions); err != nil {
		return nil, nil, nil, nil, nil, fmt.Errorf("marshal conditions: %w", err)
	}
	if redactions, err = json.Marshal(pr.Redactions); err != nil {
		return nil, nil, nil, nil, nil, fmt.Errorf("marshal redactions: %w", err)
	}
	return roles, operations, rowFilter, conditions, redactions, nil
}

func scanPolicyRule(row rowScanner) (types.PolicyRule, error) {
	var pr types.PolicyRule
	var roles, operations, rowFilter, conditions, redactions []byte
	if err := row.Scan(&pr.ID, &pr.WorkspaceID, &pr.APIName, &pr.DisplayName, &pr.Description,
		&pr.Effect, &roles, &operations, &pr.ObjectType, &pr.ActionType, &rowFilter, &conditions, &redactions, &pr.Priority,
		&pr.CreatedAt, &pr.UpdatedAt); err != nil {
		return types.PolicyRule{}, classifyError(err)
	}
	if len(roles) > 0 {
		if err := json.Unmarshal(roles, &pr.Roles); err != nil {
			return types.PolicyRule{}, fmt.Errorf("unmarshal roles: %w", err)
		}
	}
	if len(operations) > 0 {
		if err := json.Unmarshal(operations, &pr.Operations); err != nil {
			return types.PolicyRule{}, fmt.Errorf("unmarshal operations: %w", err)
		}
	}
	if len(rowFilter) > 0 {
		if err := json.Unmarshal(rowFilter, &pr.RowFilter); err != nil {
			return types.PolicyRule{}, fmt.Errorf("unmarshal row_filter: %w", err)
		}
	}
	if len(conditions) > 0 {
		if err := json.Unmarshal(conditions, &pr.Conditions); err != nil {
			return types.PolicyRule{}, fmt.Errorf("unmarshal conditions: %w", err)
		}
	}
	if len(redactions) > 0 {
		if err := json.Unmarshal(redactions, &pr.Redactions); err != nil {
			return types.PolicyRule{}, fmt.Errorf("unmarshal redactions: %w", err)
		}
	}
	return pr, nil
}
