package agents

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/modelproviders"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// ContextProvider resolves one configured context source into textual context
// blocks the runtime may inject into the model prompt.
type ContextProvider interface {
	Resolve(ctx context.Context, req ContextRequest) ([]ContextItem, error)
}

// ContextRequest is passed to a ContextProvider.
type ContextRequest struct {
	WorkspaceID types.WorkspaceID
	Actor       types.Actor
	Agent       types.Agent
	Source      types.AgentContextSource
	Input       string
}

// ContextItem is one resolved chunk of context.
type ContextItem struct {
	Name     string         `json:"name"`
	Ref      string         `json:"ref,omitempty"`
	Content  string         `json:"content"`
	Metadata map[string]any `json:"metadata,omitempty"`
}

// MemoryStore is the persistent memory interface used by the runtime.
type MemoryStore interface {
	Put(ctx context.Context, rec types.AgentMemoryRecord) (types.AgentMemoryRecord, error)
	List(ctx context.Context, ws types.WorkspaceID, namespace string, limit int) ([]types.AgentMemoryRecord, error)
}

// Planner produces or updates explicit plans for long-horizon runs.
type Planner interface {
	Generate(ctx context.Context, req PlannerRequest) (map[string]any, error)
}

// PlannerRequest is the planner input.
type PlannerRequest struct {
	Agent   types.Agent
	Input   string
	Context []ContextItem
	Plan    map[string]any
}

// Compactor summarizes prior messages when the working set grows too large.
type Compactor interface {
	Compact(ctx context.Context, req CompactRequest) (CompactResult, error)
}

// CustomToolExecutor executes a non-callback custom tool kind.
type CustomToolExecutor interface {
	Execute(ctx context.Context, tool types.CustomTool, input map[string]any) (map[string]any, error)
}

// CompactRequest is the compaction input.
type CompactRequest struct {
	Agent    types.Agent
	Messages []modelproviders.Message
}

// CompactResult contains the retained and summarized state after compaction.
type CompactResult struct {
	Summary        string
	Retained       []modelproviders.Message
	CompactedCount int
}

type repoMemoryStore struct {
	repo *storage.AgentRunRepo
}

func (s repoMemoryStore) Put(ctx context.Context, rec types.AgentMemoryRecord) (types.AgentMemoryRecord, error) {
	return s.repo.PutMemory(ctx, rec)
}

func (s repoMemoryStore) List(ctx context.Context, ws types.WorkspaceID, namespace string, limit int) ([]types.AgentMemoryRecord, error) {
	return s.repo.ListMemory(ctx, ws, namespace, limit)
}

type modelPlanner struct {
	provider modelproviders.Provider
}

func (p modelPlanner) Generate(ctx context.Context, req PlannerRequest) (map[string]any, error) {
	var contextLines []string
	for _, item := range req.Context {
		contextLines = append(contextLines, fmt.Sprintf("- %s: %s", item.Name, item.Content))
	}
	existingPlan := "{}"
	if len(req.Plan) > 0 {
		raw, _ := json.Marshal(req.Plan)
		existingPlan = string(raw)
	}
	prompt := strings.TrimSpace(`
Return JSON with keys summary, goals, tasks, risks, status.
Keep it compact and execution-oriented.
If an existing plan is provided, update it instead of restarting from scratch.
`)
	resp, err := p.provider.Call(ctx, modelproviders.CallRequest{
		Model: req.Agent.Model.Model,
		Messages: []modelproviders.Message{
			{Role: "system", Content: prompt},
			{Role: "user", Content: fmt.Sprintf("Agent: %s\nInput: %s\nContext:\n%s\nExisting plan:\n%s", req.Agent.APIName, req.Input, strings.Join(contextLines, "\n"), existingPlan)},
		},
		Temperature: 0,
		MaxTokens:   maxInt(req.Agent.Model.MaxTokens/4, 512),
	})
	if err != nil {
		return nil, err
	}
	var out map[string]any
	if err := json.Unmarshal([]byte(resp.Message.Content), &out); err == nil {
		return out, nil
	}
	return map[string]any{
		"summary": resp.Message.Content,
		"tasks":   []string{req.Input},
		"status":  "active",
	}, nil
}

type modelCompactor struct {
	provider modelproviders.Provider
}

func (c modelCompactor) Compact(ctx context.Context, req CompactRequest) (CompactResult, error) {
	if len(req.Messages) <= 2 {
		return CompactResult{Retained: req.Messages}, nil
	}
	raw, _ := json.Marshal(req.Messages)
	resp, err := c.provider.Call(ctx, modelproviders.CallRequest{
		Model: req.Agent.Model.Model,
		Messages: []modelproviders.Message{
			{Role: "system", Content: "Summarize the conversation for continuity. Preserve decisions, open issues, plan state, and unresolved blockers. Omit redundant tool output."},
			{Role: "user", Content: string(raw)},
		},
		Temperature: 0,
		MaxTokens:   maxInt(req.Agent.Model.MaxTokens/4, 512),
	})
	if err != nil {
		return CompactResult{}, err
	}
	keep := req.Agent.Compaction.PreserveRecentMessages
	if keep <= 0 || keep > len(req.Messages) {
		keep = minInt(5, len(req.Messages))
	}
	retained := []modelproviders.Message{
		{Role: "system", Content: fmt.Sprintf("Compacted summary:\n%s", resp.Message.Content)},
	}
	retained = append(retained, req.Messages[len(req.Messages)-keep:]...)
	return CompactResult{
		Summary:        resp.Message.Content,
		Retained:       retained,
		CompactedCount: len(req.Messages) - keep,
	}, nil
}

func memoryNamespace(agent types.Agent, actor types.Actor) string {
	if agent.Memory.Namespace != "" {
		return agent.Memory.Namespace
	}
	scope := agent.Memory.Scope
	if scope == "" {
		scope = "agent"
	}
	switch scope {
	case "user":
		return fmt.Sprintf("%s:%s", agent.APIName, actor.UserID)
	case "organization":
		if orgID, ok := actor.Claim("org_id"); ok {
			return fmt.Sprintf("%s:%v", agent.APIName, orgID)
		}
	}
	return string(agent.APIName)
}

func rememberRunResult(ctx context.Context, store MemoryStore, run types.AgentRun, agent types.Agent, actor types.Actor) (types.AgentMemoryRecord, error) {
	content := run.FinalResponse
	if content == "" {
		content = run.ErrorMessage
	}
	rec := types.AgentMemoryRecord{
		ID:          ids.NewULID(),
		WorkspaceID: run.WorkspaceID,
		Namespace:   memoryNamespace(agent, actor),
		Scope:       agent.Memory.Scope,
		ActorUserID: actor.UserID,
		Agent:       agent.APIName,
		Kind:        "run_summary",
		Content:     content,
		Summary:     truncateString(content, 512),
	}
	return store.Put(ctx, rec)
}

func truncateString(s string, n int) string {
	if n <= 0 || len(s) <= n {
		return s
	}
	return s[:n]
}

func maxInt(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func minInt(a, b int) int {
	if a < b {
		return a
	}
	return b
}
