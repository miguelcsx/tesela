// toolDispatcher routes a tool call to the corresponding pipeline (query or
// actions) based on the tool's Kind.

package agents

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/actions"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/modelproviders"
	"github.com/miguelcsx/lattice/pkg/lattice/query"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// errToolDenied is returned when a policy decision rejects a tool call. The
// loop logs and continues so the model can recover.
var errToolDenied = errors.New("tool denied by policy")

type toolDispatcher struct {
	snap  *types.Ontology
	tools map[string]Tool
	deps  Deps
	actor types.Actor
	wsID  types.WorkspaceID
	runID types.AgentRunID
	rt    *Runtime
	// customToolCallbacks holds FFI-registered Python/Node/Rust closures.
	customToolCallbacks map[string]func(context.Context, map[string]any) (map[string]any, error)
}

func newToolDispatcher(snap *types.Ontology, tools []Tool, deps Deps, actor types.Actor, ws types.WorkspaceID) *toolDispatcher {
	return newToolDispatcherWithCallbacks(snap, tools, deps, actor, ws, nil)
}

func newToolDispatcherWithCallbacks(snap *types.Ontology, tools []Tool, deps Deps, actor types.Actor, ws types.WorkspaceID, callbacks map[string]func(context.Context, map[string]any) (map[string]any, error)) *toolDispatcher {
	out := &toolDispatcher{snap: snap, deps: deps, actor: actor, wsID: ws, customToolCallbacks: callbacks}
	out.tools = make(map[string]Tool, len(tools))
	for _, t := range tools {
		out.tools[t.Descriptor.Name] = t
	}
	return out
}

func newToolDispatcherWithRuntime(snap *types.Ontology, tools []Tool, deps Deps, actor types.Actor, ws types.WorkspaceID, runID types.AgentRunID, rt *Runtime) *toolDispatcher {
	out := newToolDispatcherWithCallbacks(snap, tools, deps, actor, ws, nil)
	out.runID = runID
	out.rt = rt
	return out
}

// dispatch finds the tool by name and runs it.
func (d *toolDispatcher) dispatch(ctx context.Context, call modelproviders.ToolCall) (map[string]any, error) {
	t, ok := d.tools[call.Name]
	if !ok {
		return nil, fmt.Errorf("unknown tool %q", call.Name)
	}
	args := decodeToolArgs(call.Arguments)
	switch t.Kind {
	case ToolKindSearch:
		return d.search(ctx, t, args)
	case ToolKindGet:
		return d.get(ctx, t, args)
	case ToolKindTraverse:
		return d.traverse(ctx, t, args)
	case ToolKindExecute:
		return d.execute(ctx, t, args)
	case ToolKindCustomWebhook:
		return d.customWebhook(ctx, t, args)
	case ToolKindCustomSQL:
		return d.customExternal(ctx, t, args)
	case ToolKindCustomCallback:
		return d.customCallback(ctx, t, args)
	case ToolKindCustomComposite:
		return d.customComposite(ctx, t, args)
	case ToolKindMemoryRead:
		return d.memoryRead(ctx, args)
	case ToolKindMemoryWrite:
		return d.memoryWrite(ctx, args)
	case ToolKindDelegateAgent:
		return d.delegate(ctx, t, args)
	case ToolKindCommunicate:
		return d.communicate(ctx, t, args)
	default:
		return nil, fmt.Errorf("tool kind %q not supported", t.Kind)
	}
}

func (d *toolDispatcher) search(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	limit := 50
	if v, ok := args["limit"].(float64); ok {
		limit = int(v)
	}
	spec := types.QuerySpec{Page: types.PageSpec{Limit: limit}}
	if raw, ok := args["filter"]; ok {
		if f, err := decodeFilter(raw); err == nil {
			spec.Filter = f
		}
	}
	page, err := d.deps.QueryPipeline.Search(ctx, query.SearchRequest{
		Actor: d.actor, WorkspaceID: d.wsID, ObjectType: t.ObjectType,
		Spec: spec,
	})
	if err != nil {
		return nil, err
	}
	return map[string]any{"records": page.Records, "next_cursor": page.NextCursor}, nil
}

func (d *toolDispatcher) get(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	pk, _ := args["primary_key"].(string)
	rec, err := d.deps.QueryPipeline.Get(ctx, query.GetRequest{
		Actor: d.actor, WorkspaceID: d.wsID, ObjectType: t.ObjectType, PrimaryKey: pk,
	})
	if err != nil {
		return nil, err
	}
	return map[string]any{"record": rec}, nil
}

func (d *toolDispatcher) traverse(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	src, _ := args["source_key"].(string)
	spec := types.QuerySpec{}
	if raw, ok := args["filter"]; ok {
		if f, err := decodeFilter(raw); err == nil {
			spec.Filter = f
		}
	}
	page, err := d.deps.QueryPipeline.Traverse(ctx, query.TraverseRequest{
		Actor: d.actor, WorkspaceID: d.wsID, LinkType: t.LinkType, SourceKey: src, Spec: spec,
	})
	if err != nil {
		return nil, err
	}
	return map[string]any{"records": page.Records, "next_cursor": page.NextCursor}, nil
}

func (d *toolDispatcher) execute(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	res, err := d.deps.ActionPipeline.Execute(ctx, actions.ExecuteRequest{
		Actor: d.actor, WorkspaceID: d.wsID, ActionTypeName: t.ActionType, Input: args,
	})
	if err != nil {
		return nil, err
	}
	return map[string]any{"run_id": res.RunID, "status": res.Status, "output": res.Output}, nil
}

func (d *toolDispatcher) customCallback(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	if d.customToolCallbacks == nil {
		return nil, fmt.Errorf("custom tool callbacks not configured")
	}
	fn, ok := d.customToolCallbacks[string(t.CustomTool)]
	if !ok {
		return nil, fmt.Errorf("no callback registered for custom tool %q", t.CustomTool)
	}
	return fn(ctx, args)
}

func (d *toolDispatcher) customWebhook(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	ct, ok := findCustomTool(d.snap, t.CustomTool)
	if !ok || ct.Webhook == nil {
		return nil, fmt.Errorf("webhook custom tool %q not found", t.CustomTool)
	}
	body, _ := json.Marshal(args)
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, ct.Webhook.URL, bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	payload, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}
	var out map[string]any
	if len(payload) > 0 && json.Unmarshal(payload, &out) == nil {
		out["_status_code"] = resp.StatusCode
		return out, nil
	}
	return map[string]any{"status_code": resp.StatusCode, "body": string(payload)}, nil
}

func (d *toolDispatcher) customExternal(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	if d.deps.CustomToolExecutors == nil {
		return nil, fmt.Errorf("custom tool executors not configured")
	}
	exec := d.deps.CustomToolExecutors[string(t.CustomTool)]
	if exec == nil {
		exec = d.deps.CustomToolExecutors[string(ToolKindCustomSQL)]
	}
	if exec == nil {
		return nil, fmt.Errorf("no executor registered for custom tool %q", t.CustomTool)
	}
	ct, ok := findCustomTool(d.snap, t.CustomTool)
	if !ok {
		return nil, fmt.Errorf("custom tool %q not found", t.CustomTool)
	}
	return exec.Execute(ctx, ct, args)
}

func (d *toolDispatcher) customComposite(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	ct, ok := findCustomTool(d.snap, t.CustomTool)
	if !ok || ct.Composite == nil {
		return nil, fmt.Errorf("composite custom tool %q not found", t.CustomTool)
	}
	out := map[string]any{"steps": make([]map[string]any, 0, len(ct.Composite.Steps))}
	for _, step := range ct.Composite.Steps {
		res, err := d.execute(ctx, Tool{Kind: ToolKindExecute, ActionType: step.ActionRef}, args)
		stepResult := map[string]any{"name": step.Name, "action_ref": step.ActionRef, "result": res}
		if err != nil {
			stepResult["error"] = err.Error()
			out["steps"] = append(out["steps"].([]map[string]any), stepResult)
			if step.OnFailure != types.CompositeOnFailureSkip {
				return out, err
			}
			continue
		}
		out["steps"] = append(out["steps"].([]map[string]any), stepResult)
	}
	return out, nil
}

func (d *toolDispatcher) memoryRead(ctx context.Context, args map[string]any) (map[string]any, error) {
	if d.deps.MemoryStore == nil {
		return nil, fmt.Errorf("memory store not configured")
	}
	limit := 10
	if v, ok := args["limit"].(float64); ok {
		limit = int(v)
	}
	namespace := ""
	for _, agent := range d.snap.Agents {
		if namespace == "" && agent.Memory.Enabled {
			namespace = memoryNamespace(agent, d.actor)
		}
	}
	recs, err := d.deps.MemoryStore.List(ctx, d.wsID, namespace, limit)
	if err != nil {
		return nil, err
	}
	queryString, _ := args["query"].(string)
	if queryString != "" {
		filtered := recs[:0]
		for _, rec := range recs {
			if strings.Contains(strings.ToLower(rec.Content), strings.ToLower(queryString)) || strings.Contains(strings.ToLower(rec.Summary), strings.ToLower(queryString)) {
				filtered = append(filtered, rec)
			}
		}
		recs = filtered
	}
	return map[string]any{"records": recs}, nil
}

func (d *toolDispatcher) memoryWrite(ctx context.Context, args map[string]any) (map[string]any, error) {
	if d.deps.MemoryStore == nil {
		return nil, fmt.Errorf("memory store not configured")
	}
	content, _ := args["content"].(string)
	if content == "" {
		return nil, fmt.Errorf("content is required")
	}
	summary, _ := args["summary"].(string)
	kind, _ := args["kind"].(string)
	if kind == "" {
		kind = "note"
	}
	rec, err := d.deps.MemoryStore.Put(ctx, types.AgentMemoryRecord{
		ID:          ids.NewULID(),
		WorkspaceID: d.wsID,
		Namespace:   "agent",
		ActorUserID: d.actor.UserID,
		Kind:        kind,
		Content:     content,
		Summary:     summary,
	})
	if err != nil {
		return nil, err
	}
	return map[string]any{"memory_id": rec.ID, "summary": rec.Summary}, nil
}

func (d *toolDispatcher) delegate(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	if d.rt == nil {
		return nil, fmt.Errorf("runtime not configured for delegation")
	}
	input, _ := args["input"].(string)
	if input == "" {
		input = fmt.Sprint(args)
	}
	res, err := d.rt.Start(ctx, StartRequest{
		Actor:       d.actor,
		WorkspaceID: d.wsID,
		Agent:       t.AgentRef,
		Input:       input,
		ParentRunID: d.runID,
	})
	if err != nil {
		return nil, err
	}
	return map[string]any{"run_id": res.RunID, "status": res.Status, "agent": t.AgentRef}, nil
}

func (d *toolDispatcher) communicate(ctx context.Context, t Tool, args map[string]any) (map[string]any, error) {
	message, _ := args["message"].(string)
	recipient, _ := args["recipient"].(string)
	if d.deps.Store != nil {
		raw, _ := json.Marshal(map[string]any{"channel": t.Channel, "recipient": recipient})
		_ = d.deps.Store.AgentRuns().InsertMessage(ctx, types.AgentMessageTrace{
			ID:         ids.NewULID(),
			AgentRunID: d.runID,
			Sequence:   int(time.Now().UnixNano() % 1000000),
			Role:       "assistant",
			Kind:       "communication",
			Content:    message,
			Name:       t.Channel,
			Metadata:   raw,
		})
	}
	return map[string]any{"status": "queued", "channel": t.Channel, "recipient": recipient}, nil
}

func decodeToolArgs(raw json.RawMessage) map[string]any {
	out := make(map[string]any)
	if len(raw) == 0 {
		return out
	}
	_ = json.Unmarshal(raw, &out)
	return out
}

func decodeFilter(raw any) (types.Filter, error) {
	var out types.Filter
	buf, err := json.Marshal(raw)
	if err != nil {
		return types.Filter{}, err
	}
	if err := json.Unmarshal(buf, &out); err != nil {
		return types.Filter{}, err
	}
	return out, out.Validate()
}
