// Runtime is the agent execution loop. Run starts a fresh AgentRun, builds
// tools + system prompt, drives the model+tool loop, and persists the
// resulting trace.

package agents

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/actions"
	"github.com/miguelcsx/lattice/pkg/lattice/audit"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/modelproviders"
	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/policy"
	"github.com/miguelcsx/lattice/pkg/lattice/query"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Deps bundles the runtime's dependencies.
type Deps struct {
	Store               *storage.Store
	Ontology            *ontology.Registry
	Policies            PolicyResolver
	QueryPipeline       *query.Pipeline
	ActionPipeline      *actions.Pipeline
	Providers           map[string]modelproviders.Provider
	ContextProviders    map[string]ContextProvider
	MemoryStore         MemoryStore
	Planner             Planner
	Compactor           Compactor
	CustomToolExecutors map[string]CustomToolExecutor
	Audit               *audit.Writer
	Now                 func() time.Time
}

// PolicyResolver duplicated to avoid import cycle.
type PolicyResolver interface {
	For(snap *types.Ontology) (*policy.Evaluator, error)
}

// Runtime is the agent runtime.
type Runtime struct{ deps Deps }

// NewRuntime constructs a Runtime.
func NewRuntime(d Deps) *Runtime {
	if d.Now == nil {
		d.Now = time.Now
	}
	if d.MemoryStore == nil && d.Store != nil {
		d.MemoryStore = repoMemoryStore{repo: d.Store.AgentRuns()}
	}
	return &Runtime{deps: d}
}

// StartRequest is the input to Start.
type StartRequest struct {
	Actor       types.Actor
	WorkspaceID types.WorkspaceID
	Agent       types.APIName
	Input       string
	RequestID   string
	ParentRunID types.AgentRunID
}

// StartResult is what the API returns to the client.
type StartResult struct {
	RunID  types.AgentRunID `json:"run_id"`
	Status types.RunStatus  `json:"status"`
}

// Start runs the agent synchronously. Async dispatch wraps Start in a worker
// task; the API endpoint returns immediately with run_id and the worker
// polls for completion.
func (r *Runtime) Start(ctx context.Context, req StartRequest) (StartResult, error) {
	snap, err := r.deps.Ontology.Snapshot(ctx, req.WorkspaceID)
	if err != nil {
		return StartResult{}, errs.Wrap(err, errs.CodeInternal, "ontology")
	}
	agent, ok := agentByName(snap, req.Agent)
	if !ok {
		return StartResult{}, errs.Newf(errs.CodeNotFound, "agent %q not found", req.Agent)
	}
	eval, err := r.deps.Policies.For(snap)
	if err != nil {
		return StartResult{}, errs.Wrap(err, errs.CodeInternal, "policy")
	}
	tools, err := AssembleTools(snap, agent, req.Actor, eval)
	if err != nil {
		return StartResult{}, errs.Wrap(err, errs.CodeInternal, "tools")
	}
	provider, ok := r.deps.Providers[agent.Model.Provider]
	if !ok {
		return StartResult{}, errs.Newf(errs.CodeValidation, "model provider %q not registered", agent.Model.Provider)
	}
	contextItems, memoryRecords := r.resolveContext(ctx, snap.Workspace.ID, agent, req)
	planState, err := r.plan(ctx, agent, provider, req.Input, contextItems, nil)
	if err != nil {
		return StartResult{}, errs.Wrap(err, errs.CodeInternal, "plan")
	}
	run, err := r.createRun(ctx, snap.Workspace.ID, agent, req, planState, contextItems, memoryRecords)
	if err != nil {
		return StartResult{}, err
	}
	go r.execute(context.Background(), snap, agent, req, run, tools, provider, contextItems, memoryRecords, planState)
	return StartResult{RunID: run.ID, Status: run.Status}, nil
}

func (r *Runtime) createRun(ctx context.Context, ws types.WorkspaceID, agent types.Agent, req StartRequest, plan map[string]any, contextItems []ContextItem, memoryRecords []types.AgentMemoryRecord) (types.AgentRun, error) {
	input, _ := json.Marshal(map[string]string{"input": req.Input})
	planRaw, _ := json.Marshal(plan)
	contextRaw, _ := json.Marshal(contextItems)
	memoryRaw, _ := json.Marshal(memoryRecords)
	ar := types.AgentRun{
		ID:          types.AgentRunID(ids.NewULID()),
		WorkspaceID: ws,
		Agent:       agent.APIName,
		ParentRunID: req.ParentRunID,
		ActorUserID: req.Actor.UserID,
		ActorRoles:  append([]string(nil), req.Actor.Roles...),
		Input:       input,
		Plan:        planRaw,
		ContextRefs: contextRaw,
		MemoryRefs:  memoryRaw,
		Status:      types.RunStatusRunning,
	}
	out, err := r.deps.Store.AgentRuns().Create(ctx, ar)
	if err != nil {
		return types.AgentRun{}, errs.Wrap(err, errs.CodeInternal, "create agent run")
	}
	return out, nil
}

// execute is the per-run loop that drives the model and dispatches tools.
func (r *Runtime) execute(ctx context.Context, snap *types.Ontology, agent types.Agent, req StartRequest, run types.AgentRun, tools []Tool, provider modelproviders.Provider, contextItems []ContextItem, memoryRecords []types.AgentMemoryRecord, planState map[string]any) {
	deadline := time.Now().Add(time.Duration(agent.Limits.TimeoutSeconds) * time.Second)
	if agent.Limits.TimeoutSeconds <= 0 {
		deadline = time.Now().Add(5 * time.Minute)
	}
	ctx, cancel := context.WithDeadline(ctx, deadline)
	defer cancel()
	startedAt := r.deps.Now().UTC()
	run.StartedAt = &startedAt
	if _, err := r.deps.Store.AgentRuns().Update(ctx, run); err != nil {
		// Best effort.
	}

	messages := []modelproviders.Message{
		{Role: "system", Content: composeSystemPrompt(agent, tools, contextItems, memoryRecords, planState)},
		{Role: "user", Content: req.Input},
	}
	dispatcher := newToolDispatcherWithRuntime(snap, tools, r.deps, req.Actor, snap.Workspace.ID, run.ID, r)
	sequence := 0
	sequence = r.traceMessage(ctx, run.ID, sequence, "system", "prompt", messages[0].Content, "", "", map[string]any{"context_items": len(contextItems), "memory_items": len(memoryRecords)})
	sequence = r.traceMessage(ctx, run.ID, sequence, "user", "input", req.Input, "", "", nil)

	stop := false
	toolCalls := 0
	for !stop {
		if compacted, nextMessages, summary := r.maybeCompact(ctx, agent, provider, messages); compacted {
			messages = nextMessages
			sequence = r.traceMessage(ctx, run.ID, sequence, "system", "compaction", summary, "", "", nil)
		}
		if agent.Limits.MaxToolCalls > 0 && toolCalls >= agent.Limits.MaxToolCalls {
			r.fail(ctx, run, fmt.Errorf("max_tool_calls=%d exceeded", agent.Limits.MaxToolCalls))
			return
		}
		resp, err := provider.Call(ctx, modelproviders.CallRequest{
			Model:       agent.Model.Model,
			Messages:    messages,
			Tools:       toolDescriptors(tools),
			Temperature: agent.Model.Temperature,
			MaxTokens:   agent.Model.MaxTokens,
		})
		if err != nil {
			r.fail(ctx, run, fmt.Errorf("model: %w", err))
			return
		}
		messages = append(messages, resp.Message)
		run.TokensUsed += resp.Usage.InputTokens + resp.Usage.OutputTokens
		sequence = r.traceMessage(ctx, run.ID, sequence, resp.Message.Role, "assistant", resp.Message.Content, resp.Message.Name, "", map[string]any{"tool_calls": len(resp.Message.ToolCalls)})
		if len(resp.Message.ToolCalls) == 0 {
			run.FinalResponse = resp.Message.Content
			stop = true
			break
		}
		for _, call := range resp.Message.ToolCalls {
			toolCalls++
			run.ToolCallCount++
			sequence = r.traceMessage(ctx, run.ID, sequence, "assistant", "tool_call", string(call.Arguments), call.Name, call.ID, nil)
			result, err := dispatcher.dispatch(ctx, call)
			messages = append(messages, modelproviders.Message{
				Role: "tool", ToolCallID: call.ID, Content: encodeToolResult(result, err),
			})
			sequence = r.traceMessage(ctx, run.ID, sequence, "tool", "tool_result", encodeToolResult(result, err), call.Name, call.ID, nil)
			if err != nil && errors.Is(err, errToolDenied) {
				continue
			}
			if err != nil {
				continue
			}
			if agent.Planning.Enabled && agent.Planning.ReplanAfterToolCalls > 0 && toolCalls%agent.Planning.ReplanAfterToolCalls == 0 {
				nextPlan, planErr := r.plan(ctx, agent, provider, req.Input, contextItems, planState)
				if planErr == nil {
					planState = nextPlan
					run.Plan, _ = json.Marshal(planState)
					sequence = r.traceMessage(ctx, run.ID, sequence, "system", "plan_update", string(run.Plan), "", "", nil)
				}
			}
		}
	}
	run.Plan, _ = json.Marshal(planState)
	if agent.Memory.Enabled && r.deps.MemoryStore != nil && agent.Memory.WriteMode != "disabled" {
		if rec, err := rememberRunResult(ctx, r.deps.MemoryStore, run, agent, req.Actor); err == nil {
			run.MemoryRefs, _ = json.Marshal(append(memoryRecords, rec))
			sequence = r.traceMessage(ctx, run.ID, sequence, "system", "memory_write", rec.Summary, "", "", rec.Metadata)
		}
	}
	r.complete(ctx, run)
}

func (r *Runtime) complete(ctx context.Context, run types.AgentRun) {
	now := r.deps.Now().UTC()
	run.Status = types.RunStatusDone
	run.FinishedAt = &now
	if _, err := r.deps.Store.AgentRuns().Update(ctx, run); err != nil {
		// Best-effort.
	}
}

func (r *Runtime) fail(ctx context.Context, run types.AgentRun, err error) {
	now := r.deps.Now().UTC()
	run.Status = types.RunStatusFailed
	run.ErrorMessage = err.Error()
	run.FinishedAt = &now
	_, _ = r.deps.Store.AgentRuns().Update(ctx, run)
}

func toolDescriptors(in []Tool) []modelproviders.Tool {
	out := make([]modelproviders.Tool, 0, len(in))
	for _, t := range in {
		out = append(out, t.Descriptor)
	}
	return out
}

func encodeToolResult(result map[string]any, err error) string {
	if err != nil {
		b, _ := json.Marshal(map[string]string{"error": err.Error()})
		return string(b)
	}
	b, _ := json.Marshal(result)
	return string(b)
}

func agentByName(snap *types.Ontology, name types.APIName) (types.Agent, bool) {
	for _, a := range snap.Agents {
		if a.APIName == name {
			return a, true
		}
	}
	return types.Agent{}, false
}

func composeSystemPrompt(agent types.Agent, tools []Tool, contextItems []ContextItem, memoryRecords []types.AgentMemoryRecord, plan map[string]any) string {
	out := agent.SystemPrompt
	if len(contextItems) > 0 {
		out += "\n\nRuntime context:\n"
		for _, item := range contextItems {
			out += fmt.Sprintf("  - %s: %s\n", item.Name, item.Content)
		}
	}
	if len(memoryRecords) > 0 {
		out += "\nPersistent memory:\n"
		for _, item := range memoryRecords {
			line := item.Summary
			if line == "" {
				line = item.Content
			}
			out += fmt.Sprintf("  - [%s] %s\n", item.Kind, line)
		}
	}
	if len(plan) > 0 {
		raw, _ := json.Marshal(plan)
		out += "\nCurrent plan:\n" + string(raw) + "\n"
	}
	out += "\n\nAvailable tools:\n"
	for _, t := range tools {
		out += fmt.Sprintf("  - %s: %s\n", t.Descriptor.Name, t.Descriptor.Description)
	}
	return out
}

func (r *Runtime) resolveContext(ctx context.Context, ws types.WorkspaceID, agent types.Agent, req StartRequest) ([]ContextItem, []types.AgentMemoryRecord) {
	var items []ContextItem
	var memories []types.AgentMemoryRecord
	if agent.Memory.Enabled && agent.Memory.IncludeInPrompt && r.deps.MemoryStore != nil {
		recs, err := r.deps.MemoryStore.List(ctx, ws, memoryNamespace(agent, req.Actor), maxInt(agent.Memory.MaxEntries, 10))
		if err == nil {
			memories = recs
		}
	}
	for _, src := range agent.ContextSources {
		provider := r.deps.ContextProviders[src.Kind]
		if provider == nil {
			continue
		}
		resolved, err := provider.Resolve(ctx, ContextRequest{
			WorkspaceID: ws,
			Actor:       req.Actor,
			Agent:       agent,
			Source:      src,
			Input:       req.Input,
		})
		if err == nil {
			items = append(items, resolved...)
		}
	}
	return items, memories
}

func (r *Runtime) plan(ctx context.Context, agent types.Agent, provider modelproviders.Provider, input string, contextItems []ContextItem, current map[string]any) (map[string]any, error) {
	if !agent.Planning.Enabled {
		return current, nil
	}
	planner := r.deps.Planner
	if planner == nil {
		planner = modelPlanner{provider: provider}
	}
	return planner.Generate(ctx, PlannerRequest{
		Agent:   agent,
		Input:   input,
		Context: contextItems,
		Plan:    current,
	})
}

func (r *Runtime) maybeCompact(ctx context.Context, agent types.Agent, provider modelproviders.Provider, messages []modelproviders.Message) (bool, []modelproviders.Message, string) {
	if !agent.Compaction.Enabled || agent.Compaction.TriggerTokens <= 0 {
		return false, messages, ""
	}
	if approximateTokens(messages) < agent.Compaction.TriggerTokens {
		return false, messages, ""
	}
	compactor := r.deps.Compactor
	if compactor == nil {
		compactor = modelCompactor{provider: provider}
	}
	res, err := compactor.Compact(ctx, CompactRequest{Agent: agent, Messages: messages})
	if err != nil {
		return false, messages, ""
	}
	return true, res.Retained, res.Summary
}

func approximateTokens(messages []modelproviders.Message) int {
	total := 0
	for _, msg := range messages {
		total += len(msg.Content) / 4
	}
	return total
}

func (r *Runtime) traceMessage(ctx context.Context, runID types.AgentRunID, seq int, role, kind, content, name, toolCallID string, metadata map[string]any) int {
	if r.deps.Store == nil {
		return seq
	}
	raw, _ := json.Marshal(metadata)
	_ = r.deps.Store.AgentRuns().InsertMessage(ctx, types.AgentMessageTrace{
		ID:         ids.NewULID(),
		AgentRunID: runID,
		Sequence:   seq + 1,
		Role:       role,
		Kind:       kind,
		Content:    content,
		Name:       name,
		ToolCallID: toolCallID,
		Metadata:   raw,
	})
	return seq + 1
}
