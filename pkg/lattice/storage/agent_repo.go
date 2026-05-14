// AgentRepo persists types.Agent entities.

package storage

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// AgentRepo handles CRUD for agents.
type AgentRepo struct{ q Querier }

// Upsert inserts or replaces an agent by (workspace_id, api_name).
func (r *AgentRepo) Upsert(ctx context.Context, ag types.Agent) (types.Agent, error) {
	model, fromObjectTypes, fromLinkTypes, fromActions, customTools, contextSources, memory, planning, compaction, subagents, communication, allowedRoles, limits, err := marshalAgent(ag)
	if err != nil {
		return types.Agent{}, err
	}
	const q = `
INSERT INTO agents (id, workspace_id, api_name, display_name, description, system_prompt, model, from_object_types, from_link_types, from_actions, custom_tools, context_sources, memory, planning, compaction, subagents, communication, allowed_roles, limits, require_approval_for_actions)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
ON CONFLICT (workspace_id, api_name) DO UPDATE SET
    display_name=EXCLUDED.display_name,
    description=EXCLUDED.description,
    system_prompt=EXCLUDED.system_prompt,
    model=EXCLUDED.model,
    from_object_types=EXCLUDED.from_object_types,
    from_link_types=EXCLUDED.from_link_types,
    from_actions=EXCLUDED.from_actions,
    custom_tools=EXCLUDED.custom_tools,
    context_sources=EXCLUDED.context_sources,
    memory=EXCLUDED.memory,
    planning=EXCLUDED.planning,
    compaction=EXCLUDED.compaction,
    subagents=EXCLUDED.subagents,
    communication=EXCLUDED.communication,
    allowed_roles=EXCLUDED.allowed_roles,
    limits=EXCLUDED.limits,
    require_approval_for_actions=EXCLUDED.require_approval_for_actions,
    updated_at=now()
RETURNING id, created_at, updated_at`
	if err := r.q.QueryRow(ctx, q, ag.ID, ag.WorkspaceID, ag.APIName, ag.DisplayName, ag.Description,
		ag.SystemPrompt, model, fromObjectTypes, fromLinkTypes, fromActions, customTools, contextSources, memory, planning, compaction, subagents, communication, allowedRoles, limits, ag.RequireApprovalForActions).
		Scan(&ag.ID, &ag.CreatedAt, &ag.UpdatedAt); err != nil {
		return types.Agent{}, classifyError(err)
	}
	return ag, nil
}

// GetByAPIName returns the agent with the given api_name.
func (r *AgentRepo) GetByAPIName(ctx context.Context, ws types.WorkspaceID, name types.APIName) (types.Agent, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, system_prompt, model, from_object_types, from_link_types, from_actions, custom_tools, context_sources, memory, planning, compaction, subagents, communication, allowed_roles, limits, require_approval_for_actions, created_at, updated_at
FROM agents WHERE workspace_id = $1 AND api_name = $2`
	return scanAgent(r.q.QueryRow(ctx, q, ws, name))
}

// List returns every agent for a workspace.
func (r *AgentRepo) List(ctx context.Context, ws types.WorkspaceID) ([]types.Agent, error) {
	const q = `
SELECT id, workspace_id, api_name, display_name, description, system_prompt, model, from_object_types, from_link_types, from_actions, custom_tools, context_sources, memory, planning, compaction, subagents, communication, allowed_roles, limits, require_approval_for_actions, created_at, updated_at
FROM agents WHERE workspace_id = $1 ORDER BY api_name`
	rows, err := r.q.Query(ctx, q, ws)
	if err != nil {
		return nil, classifyError(err)
	}
	defer rows.Close()
	var out []types.Agent
	for rows.Next() {
		ag, err := scanAgent(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, ag)
	}
	return out, rows.Err()
}

// Delete removes an agent by api_name.
func (r *AgentRepo) Delete(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	const q = `DELETE FROM agents WHERE workspace_id = $1 AND api_name = $2`
	tag, err := r.q.Exec(ctx, q, ws, name)
	if err != nil {
		return classifyError(err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func marshalAgent(ag types.Agent) (model, fromObjectTypes, fromLinkTypes, fromActions, customTools, contextSources, memory, planning, compaction, subagents, communication, allowedRoles, limits []byte, err error) {
	if model, err = json.Marshal(ag.Model); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal model: %w", err)
	}
	if fromObjectTypes, err = json.Marshal(ag.FromObjectTypes); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal from_object_types: %w", err)
	}
	if fromLinkTypes, err = json.Marshal(ag.FromLinkTypes); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal from_link_types: %w", err)
	}
	if fromActions, err = json.Marshal(ag.FromActions); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal from_actions: %w", err)
	}
	if customTools, err = json.Marshal(ag.CustomTools); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal custom_tools: %w", err)
	}
	if contextSources, err = json.Marshal(ag.ContextSources); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal context_sources: %w", err)
	}
	if memory, err = json.Marshal(ag.Memory); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal memory: %w", err)
	}
	if planning, err = json.Marshal(ag.Planning); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal planning: %w", err)
	}
	if compaction, err = json.Marshal(ag.Compaction); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal compaction: %w", err)
	}
	if subagents, err = json.Marshal(ag.Subagents); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal subagents: %w", err)
	}
	if communication, err = json.Marshal(ag.Communication); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal communication: %w", err)
	}
	if allowedRoles, err = json.Marshal(ag.AllowedRoles); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal allowed_roles: %w", err)
	}
	if limits, err = json.Marshal(ag.Limits); err != nil {
		return nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, fmt.Errorf("marshal limits: %w", err)
	}
	return model, fromObjectTypes, fromLinkTypes, fromActions, customTools, contextSources, memory, planning, compaction, subagents, communication, allowedRoles, limits, nil
}

func scanAgent(row rowScanner) (types.Agent, error) {
	var ag types.Agent
	var model, fromObjectTypes, fromLinkTypes, fromActions, customTools, contextSources, memory, planning, compaction, subagents, communication, allowedRoles, limits []byte
	if err := row.Scan(&ag.ID, &ag.WorkspaceID, &ag.APIName, &ag.DisplayName, &ag.Description,
		&ag.SystemPrompt, &model, &fromObjectTypes, &fromLinkTypes, &fromActions, &customTools, &contextSources, &memory, &planning, &compaction, &subagents, &communication, &allowedRoles, &limits, &ag.RequireApprovalForActions,
		&ag.CreatedAt, &ag.UpdatedAt); err != nil {
		return types.Agent{}, classifyError(err)
	}
	if len(model) > 0 {
		if err := json.Unmarshal(model, &ag.Model); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal model: %w", err)
		}
	}
	if len(fromObjectTypes) > 0 {
		if err := json.Unmarshal(fromObjectTypes, &ag.FromObjectTypes); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal from_object_types: %w", err)
		}
	}
	if len(fromLinkTypes) > 0 {
		if err := json.Unmarshal(fromLinkTypes, &ag.FromLinkTypes); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal from_link_types: %w", err)
		}
	}
	if len(fromActions) > 0 {
		if err := json.Unmarshal(fromActions, &ag.FromActions); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal from_actions: %w", err)
		}
	}
	if len(customTools) > 0 {
		if err := json.Unmarshal(customTools, &ag.CustomTools); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal custom_tools: %w", err)
		}
	}
	if len(contextSources) > 0 {
		if err := json.Unmarshal(contextSources, &ag.ContextSources); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal context_sources: %w", err)
		}
	}
	if len(memory) > 0 {
		if err := json.Unmarshal(memory, &ag.Memory); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal memory: %w", err)
		}
	}
	if len(planning) > 0 {
		if err := json.Unmarshal(planning, &ag.Planning); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal planning: %w", err)
		}
	}
	if len(compaction) > 0 {
		if err := json.Unmarshal(compaction, &ag.Compaction); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal compaction: %w", err)
		}
	}
	if len(subagents) > 0 {
		if err := json.Unmarshal(subagents, &ag.Subagents); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal subagents: %w", err)
		}
	}
	if len(communication) > 0 {
		if err := json.Unmarshal(communication, &ag.Communication); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal communication: %w", err)
		}
	}
	if len(allowedRoles) > 0 {
		if err := json.Unmarshal(allowedRoles, &ag.AllowedRoles); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal allowed_roles: %w", err)
		}
	}
	if len(limits) > 0 {
		if err := json.Unmarshal(limits, &ag.Limits); err != nil {
			return types.Agent{}, fmt.Errorf("unmarshal limits: %w", err)
		}
	}
	return ag, nil
}
