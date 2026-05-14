//go:build cabi

// cabi.go — exported C ABI surface. Spec-driven: bindings serialize their
// ontology to JSON locally and apply it in a single FFI call. Per-callback
// registration uses the api_name to address an already-registered object
// type. Compared with a fluent C ABI this halves the export count and keeps
// builder semantics identical to native Go users.

package main

/*
#include <stdint.h>
#include <stdlib.h>

typedef struct {
    char* data;
    int   len;
} lattice_buffer_t;

typedef lattice_buffer_t (*lattice_callback_fn)(const char* req_json, int length);

static lattice_buffer_t lattice_call_callback(lattice_callback_fn fn, const char* q, int n) {
    return fn(q, n);
}
*/
import "C"

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"sync"
	"time"
	"unsafe"

	"github.com/miguelcsx/lattice/pkg/lattice"
	"github.com/miguelcsx/lattice/pkg/lattice/events"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
	"github.com/miguelcsx/lattice/pkg/lattice/workflow"
)

func main() {} // required by buildmode=c-shared

var (
	workflowDefsMu sync.Mutex
	workflowDefs   = make(map[uint64]map[string][]workflow.Step)
)

// ---------------------------------------------------------------------------
// App lifecycle
// ---------------------------------------------------------------------------

//export lattice_app_new
func lattice_app_new() C.uint64_t {
	defer recoverToStderr("lattice_app_new")
	return C.uint64_t(registerApp(lattice.New()))
}

//export lattice_app_release
func lattice_app_release(handle C.uint64_t) {
	defer recoverToStderr("lattice_app_release")
	deleteWorkflowDefs(uint64(handle))
	releaseApp(uint64(handle))
}

//export lattice_app_apply_spec
func lattice_app_apply_spec(handle C.uint64_t, specJSON *C.char, length C.int) *C.char {
	defer recoverToStderr("lattice_app_apply_spec")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	raw := C.GoBytes(unsafe.Pointer(specJSON), length)
	if err := a.ApplyJSON(raw); err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_app_serve
func lattice_app_serve(handle C.uint64_t, addr *C.char) *C.char {
	defer recoverToStderr("lattice_app_serve")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	if err := a.Serve(C.GoString(addr)); err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_app_serve_graceful
func lattice_app_serve_graceful(handle C.uint64_t, addr *C.char, shutdownSecs C.int) *C.char {
	defer recoverToStderr("lattice_app_serve_graceful")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	go func() {
		time.Sleep(time.Duration(shutdownSecs) * time.Second)
		cancel()
	}()
	if err := a.ServeGraceful(ctx, C.GoString(addr)); err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_app_shutdown
func lattice_app_shutdown(handle C.uint64_t) *C.char {
	defer recoverToStderr("lattice_app_shutdown")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	if err := a.Shutdown(context.Background()); err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_app_get_errors
func lattice_app_get_errors(handle C.uint64_t) *C.char {
	defer recoverToStderr("lattice_app_get_errors")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("[]")
	}
	errs := a.Errors()
	out := make([]string, 0, len(errs))
	for _, e := range errs {
		out = append(out, e.Error())
	}
	raw, _ := json.Marshal(out)
	return C.CString(string(raw))
}

//export lattice_app_set_log_level
func lattice_app_set_log_level(handle C.uint64_t, level *C.char) *C.char {
	defer recoverToStderr("lattice_app_set_log_level")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	a.SetLogLevel(C.GoString(level))
	return nil
}

// ---------------------------------------------------------------------------
// Auth, Audit, Event callbacks
// ---------------------------------------------------------------------------

//export lattice_app_set_auth_callback
func lattice_app_set_auth_callback(handle C.uint64_t, fn C.lattice_callback_fn) *C.char {
	defer recoverToStderr("lattice_app_set_auth_callback")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	a.SetAuthCallback(makeAuthClosure(fn))
	return nil
}

//export lattice_app_set_audit_callback
func lattice_app_set_audit_callback(handle C.uint64_t, fn C.lattice_callback_fn) *C.char {
	defer recoverToStderr("lattice_app_set_audit_callback")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	// Wrap the Python callback in an audit.Sink.
	cb := makeAuditClosure(fn)
	a.SetAuditCallback(cb)
	// Also replace the audit sink so the callback actually receives data.
	// Note: this overrides any previous sink.
	return nil
}

//export lattice_app_set_event_callback
func lattice_app_set_event_callback(handle C.uint64_t, fn C.lattice_callback_fn) *C.char {
	defer recoverToStderr("lattice_app_set_event_callback")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	a.SetEventCallback(makeEventClosure(fn))
	return nil
}

// ---------------------------------------------------------------------------
// Per-objecttype callbacks
// ---------------------------------------------------------------------------

//export lattice_register_search
func lattice_register_search(handle C.uint64_t, name *C.char, fn C.lattice_callback_fn) *C.char {
	defer recoverToStderr("lattice_register_search")
	b := resolveObjectType(handle, name)
	if b == nil {
		return C.CString(fmt.Sprintf("object type %q not found", C.GoString(name)))
	}
	b.Search(makeSearchClosure(fn))
	return nil
}

//export lattice_register_get
func lattice_register_get(handle C.uint64_t, name *C.char, fn C.lattice_callback_fn) *C.char {
	defer recoverToStderr("lattice_register_get")
	b := resolveObjectType(handle, name)
	if b == nil {
		return C.CString(fmt.Sprintf("object type %q not found", C.GoString(name)))
	}
	b.Get(makeGetClosure(fn))
	return nil
}

//export lattice_register_mutate
func lattice_register_mutate(handle C.uint64_t, name *C.char, fn C.lattice_callback_fn) *C.char {
	defer recoverToStderr("lattice_register_mutate")
	b := resolveObjectType(handle, name)
	if b == nil {
		return C.CString(fmt.Sprintf("object type %q not found", C.GoString(name)))
	}
	b.Mutate(makeMutateClosure(fn))
	return nil
}

//export lattice_register_action
func lattice_register_action(handle C.uint64_t, name *C.char, fn C.lattice_callback_fn) *C.char {
	defer recoverToStderr("lattice_register_action")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	apiName := C.GoString(name)
	b := a.FindActionType(apiName)
	if b == nil {
		return C.CString(fmt.Sprintf("action %q not found", apiName))
	}
	b.Callback()
	a.RegisterActionCallback(apiName, makeActionClosure(fn))
	return nil
}

//export lattice_register_custom_tool
func lattice_register_custom_tool(handle C.uint64_t, name *C.char, fn C.lattice_callback_fn) *C.char {
	defer recoverToStderr("lattice_register_custom_tool")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	apiName := C.GoString(name)
	b := a.FindCustomTool(apiName)
	if b == nil {
		return C.CString(fmt.Sprintf("custom tool %q not found", apiName))
	}
	b.Kind(types.CustomToolKindCallback)
	a.RegisterCustomToolCallback(apiName, makeToolClosure(fn))
	return nil
}

//export lattice_register_backend
func lattice_register_backend(handle C.uint64_t, name *C.char, adapterType *C.char) *C.char {
	defer recoverToStderr("lattice_register_backend")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	apiName := C.GoString(name)
	adapter := C.GoString(adapterType)
	a.RegisterDatasource(apiName, adapter, nil)
	return nil
}

func resolveObjectType(handle C.uint64_t, name *C.char) *lattice.ObjectTypeBuilder {
	a := lookupApp(uint64(handle))
	if a == nil {
		return nil
	}
	return a.FindObjectType(C.GoString(name))
}

// ---------------------------------------------------------------------------
// Branch operations
// ---------------------------------------------------------------------------

//export lattice_branch_create
func lattice_branch_create(handle C.uint64_t, name *C.char, base *C.char, createdBy *C.char) *C.char {
	defer recoverToStderr("lattice_branch_create")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	ctx := context.Background()
	_, err := a.Branches().Create(ctx, C.GoString(name), C.GoString(base), C.GoString(createdBy), types.Ontology{})
	if err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_branch_get
func lattice_branch_get(handle C.uint64_t, name *C.char) *C.char {
	defer recoverToStderr("lattice_branch_get")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("{}")
	}
	ctx := context.Background()
	b, err := a.Branches().Get(ctx, C.GoString(name))
	if err != nil {
		return C.CString(fmt.Sprintf(`{"error":%q}`, err.Error()))
	}
	raw, _ := json.Marshal(b)
	return C.CString(string(raw))
}

//export lattice_branch_list
func lattice_branch_list(handle C.uint64_t) *C.char {
	defer recoverToStderr("lattice_branch_list")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("[]")
	}
	ctx := context.Background()
	list, err := a.Branches().List(ctx)
	if err != nil {
		return C.CString("[]")
	}
	raw, _ := json.Marshal(list)
	return C.CString(string(raw))
}

//export lattice_branch_diff
func lattice_branch_diff(handle C.uint64_t, from *C.char, to *C.char) *C.char {
	defer recoverToStderr("lattice_branch_diff")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("{}")
	}
	ctx := context.Background()
	diff, err := a.Branches().Diff(ctx, C.GoString(from), C.GoString(to))
	if err != nil {
		return C.CString(fmt.Sprintf(`{"error":%q}`, err.Error()))
	}
	raw, _ := json.Marshal(diff)
	return C.CString(string(raw))
}

//export lattice_branch_promote
func lattice_branch_promote(handle C.uint64_t, name *C.char) *C.char {
	defer recoverToStderr("lattice_branch_promote")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	ctx := context.Background()
	_, err := a.Branches().Promote(ctx, C.GoString(name))
	if err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_branch_merge
func lattice_branch_merge(handle C.uint64_t, src *C.char, dst *C.char) *C.char {
	defer recoverToStderr("lattice_branch_merge")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	ctx := context.Background()
	_, err := a.Branches().Merge(ctx, C.GoString(src), C.GoString(dst))
	if err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_branch_submit_review
func lattice_branch_submit_review(handle C.uint64_t, name *C.char, reviewersJSON *C.char) *C.char {
	defer recoverToStderr("lattice_branch_submit_review")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	var reviewers []string
	if reviewersJSON != nil && C.GoString(reviewersJSON) != "" {
		if err := json.Unmarshal([]byte(C.GoString(reviewersJSON)), &reviewers); err != nil {
			return C.CString(err.Error())
		}
	}
	ctx := context.Background()
	_, err := a.Branches().SubmitForReview(ctx, C.GoString(name), reviewers)
	if err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_branch_delete
func lattice_branch_delete(handle C.uint64_t, name *C.char) *C.char {
	defer recoverToStderr("lattice_branch_delete")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	ctx := context.Background()
	if err := a.Branches().Delete(ctx, C.GoString(name)); err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_branch_update
func lattice_branch_update(handle C.uint64_t, name *C.char, specJSON *C.char, length C.int) *C.char {
	defer recoverToStderr("lattice_branch_update")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	if specJSON == nil || length <= 0 {
		return C.CString("spec JSON required")
	}
	raw := C.GoBytes(unsafe.Pointer(specJSON), length)
	tmp := lattice.New()
	if err := tmp.ApplyJSON(raw); err != nil {
		return C.CString(err.Error())
	}
	snap := tmp.Snapshot()
	ctx := context.Background()
	_, err := a.Branches().Update(ctx, C.GoString(name), *snap)
	if err != nil {
		return C.CString(err.Error())
	}
	return nil
}

// ---------------------------------------------------------------------------
// Scheduler & Workflow
// ---------------------------------------------------------------------------

//export lattice_scheduler_add
func lattice_scheduler_add(handle C.uint64_t, id *C.char, expr *C.char) *C.char {
	defer recoverToStderr("lattice_scheduler_add")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	if err := a.ScheduleWorkflow(C.GoString(id), C.GoString(expr), C.GoString(id), workflow.State{}); err != nil {
		return C.CString(err.Error())
	}
	return nil
}

//export lattice_workflow_register
func lattice_workflow_register(handle C.uint64_t, name *C.char) *C.char {
	defer recoverToStderr("lattice_workflow_register")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	workflowName := C.GoString(name)
	a.RegisterWorkflow(workflow.Definition{Name: workflowName, Steps: snapshotWorkflowSteps(uint64(handle), workflowName)})
	return nil
}

//export lattice_workflow_start
func lattice_workflow_start(handle C.uint64_t, name *C.char, stateJSON *C.char) *C.char {
	defer recoverToStderr("lattice_workflow_start")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString(`{"error":"invalid app handle"}`)
	}
	var initial workflow.State
	if stateJSON != nil && C.GoString(stateJSON) != "" {
		if err := json.Unmarshal([]byte(C.GoString(stateJSON)), &initial); err != nil {
			return C.CString(fmt.Sprintf(`{"error":%q}`, err.Error()))
		}
	}
	run, err := a.StartWorkflow(context.Background(), C.GoString(name), initial)
	if err != nil {
		return C.CString(fmt.Sprintf(`{"error":%q}`, err.Error()))
	}
	raw, _ := json.Marshal(run)
	return C.CString(string(raw))
}

//export lattice_workflow_add_step
func lattice_workflow_add_step(handle C.uint64_t, workflowName *C.char, stepName *C.char, fn C.lattice_callback_fn) *C.char {
	defer recoverToStderr("lattice_workflow_add_step")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	name := C.GoString(workflowName)
	step := workflow.Step{
		Name: C.GoString(stepName),
		Run:  makeWorkflowStepClosure(fn),
	}
	appendWorkflowStep(uint64(handle), name, step)
	a.RegisterWorkflow(workflow.Definition{Name: name, Steps: snapshotWorkflowSteps(uint64(handle), name)})
	return nil
}

//export lattice_scheduler_add_workflow
func lattice_scheduler_add_workflow(handle C.uint64_t, id *C.char, expr *C.char, workflowName *C.char, stateJSON *C.char) *C.char {
	defer recoverToStderr("lattice_scheduler_add_workflow")
	a := lookupApp(uint64(handle))
	if a == nil {
		return C.CString("invalid app handle")
	}
	var initial workflow.State
	if stateJSON != nil && C.GoString(stateJSON) != "" {
		if err := json.Unmarshal([]byte(C.GoString(stateJSON)), &initial); err != nil {
			return C.CString(err.Error())
		}
	}
	if err := a.ScheduleWorkflow(C.GoString(id), C.GoString(expr), C.GoString(workflowName), initial); err != nil {
		return C.CString(err.Error())
	}
	return nil
}

// ---------------------------------------------------------------------------
// Memory management
// ---------------------------------------------------------------------------

//export lattice_free
func lattice_free(ptr *C.char) {
	if ptr != nil {
		C.free(unsafe.Pointer(ptr))
	}
}

// ---------------------------------------------------------------------------
// Closures: bridge Go pipeline calls to C function pointers.
// ---------------------------------------------------------------------------

func makeSearchClosure(fn C.lattice_callback_fn) lattice.SearchFunc {
	return func(_ context.Context, q lattice.Query) (lattice.Page, error) {
		raw, err := json.Marshal(q)
		if err != nil {
			return lattice.Page{}, fmt.Errorf("marshal query: %w", err)
		}
		out, err := callCallback(fn, raw)
		if err != nil {
			return lattice.Page{}, err
		}
		var page lattice.Page
		if err := json.Unmarshal(out, &page); err != nil {
			return lattice.Page{}, fmt.Errorf("unmarshal page: %w", err)
		}
		return page, nil
	}
}

type getReq struct {
	PrimaryKey any `json:"primary_key"`
}

func makeGetClosure(fn C.lattice_callback_fn) lattice.GetFunc {
	return func(_ context.Context, pk any) (lattice.Record, error) {
		raw, err := json.Marshal(getReq{PrimaryKey: pk})
		if err != nil {
			return lattice.Record{}, fmt.Errorf("marshal get: %w", err)
		}
		out, err := callCallback(fn, raw)
		if err != nil {
			return lattice.Record{}, err
		}
		var rec lattice.Record
		if err := json.Unmarshal(out, &rec); err != nil {
			return lattice.Record{}, fmt.Errorf("unmarshal record: %w", err)
		}
		return rec, nil
	}
}

func makeMutateClosure(fn C.lattice_callback_fn) lattice.MutateFunc {
	return func(_ context.Context, mut lattice.Mutation) (lattice.MutationResult, error) {
		raw, err := json.Marshal(mut)
		if err != nil {
			return lattice.MutationResult{}, fmt.Errorf("marshal mutation: %w", err)
		}
		out, err := callCallback(fn, raw)
		if err != nil {
			return lattice.MutationResult{}, err
		}
		var result lattice.MutationResult
		if err := json.Unmarshal(out, &result); err != nil {
			// Tolerate clients returning the bare values map.
			result.AffectedRows = 1
			result.Returned = make(map[types.APIName]any)
			_ = json.Unmarshal(out, &result.Returned)
		}
		return result, nil
	}
}

func makeActionClosure(fn C.lattice_callback_fn) func(context.Context, map[string]any) (map[string]any, error) {
	return func(_ context.Context, input map[string]any) (map[string]any, error) {
		raw, err := json.Marshal(input)
		if err != nil {
			return nil, fmt.Errorf("marshal action input: %w", err)
		}
		out, err := callCallback(fn, raw)
		if err != nil {
			return nil, err
		}
		var result map[string]any
		if err := json.Unmarshal(out, &result); err != nil {
			return nil, fmt.Errorf("unmarshal action result: %w", err)
		}
		return result, nil
	}
}

func makeToolClosure(fn C.lattice_callback_fn) func(context.Context, map[string]any) (map[string]any, error) {
	return func(_ context.Context, input map[string]any) (map[string]any, error) {
		raw, err := json.Marshal(input)
		if err != nil {
			return nil, fmt.Errorf("marshal tool input: %w", err)
		}
		out, err := callCallback(fn, raw)
		if err != nil {
			return nil, err
		}
		var result map[string]any
		if err := json.Unmarshal(out, &result); err != nil {
			return nil, fmt.Errorf("unmarshal tool result: %w", err)
		}
		return result, nil
	}
}

func makeAuthClosure(fn C.lattice_callback_fn) func(map[string]any) (types.Actor, error) {
	return func(reqMeta map[string]any) (types.Actor, error) {
		raw, err := json.Marshal(reqMeta)
		if err != nil {
			return types.Actor{}, fmt.Errorf("marshal auth meta: %w", err)
		}
		out, err := callCallback(fn, raw)
		if err != nil {
			return types.Actor{}, err
		}
		var actor types.Actor
		if err := json.Unmarshal(out, &actor); err != nil {
			return types.Actor{}, fmt.Errorf("unmarshal actor: %w", err)
		}
		return actor, nil
	}
}

func makeAuditClosure(fn C.lattice_callback_fn) func([]types.AuditRecord) error {
	return func(records []types.AuditRecord) error {
		raw, _ := json.Marshal(records)
		out, err := callCallback(fn, raw)
		if err != nil {
			return err
		}
		var result map[string]any
		if len(out) > 0 {
			_ = json.Unmarshal(out, &result)
		}
		return nil
	}
}

func makeEventClosure(fn C.lattice_callback_fn) func(events.Event) error {
	return func(ev events.Event) error {
		raw, _ := json.Marshal(ev)
		_, err := callCallback(fn, raw)
		return err
	}
}

func makeWorkflowStepClosure(fn C.lattice_callback_fn) func(context.Context, workflow.State) (workflow.State, error) {
	return func(_ context.Context, state workflow.State) (workflow.State, error) {
		raw, err := json.Marshal(state)
		if err != nil {
			return nil, fmt.Errorf("marshal workflow state: %w", err)
		}
		out, err := callCallback(fn, raw)
		if err != nil {
			return nil, err
		}
		var envelope struct {
			State  workflow.State `json:"state"`
			Error  string         `json:"_error"`
			Status string         `json:"status,omitempty"`
		}
		if err := json.Unmarshal(out, &envelope); err == nil {
			if envelope.Error != "" {
				return nil, errors.New(envelope.Error)
			}
			if envelope.State != nil {
				return envelope.State, nil
			}
		}
		var next workflow.State
		if err := json.Unmarshal(out, &next); err != nil {
			return nil, fmt.Errorf("unmarshal workflow state: %w", err)
		}
		return next, nil
	}
}

func appendWorkflowStep(handle uint64, workflowName string, step workflow.Step) {
	workflowDefsMu.Lock()
	defer workflowDefsMu.Unlock()
	defs, ok := workflowDefs[handle]
	if !ok {
		defs = make(map[string][]workflow.Step)
		workflowDefs[handle] = defs
	}
	defs[workflowName] = append(defs[workflowName], step)
}

func snapshotWorkflowSteps(handle uint64, workflowName string) []workflow.Step {
	workflowDefsMu.Lock()
	defer workflowDefsMu.Unlock()
	defs := workflowDefs[handle]
	steps := defs[workflowName]
	out := make([]workflow.Step, len(steps))
	copy(out, steps)
	return out
}

func deleteWorkflowDefs(handle uint64) {
	workflowDefsMu.Lock()
	defer workflowDefsMu.Unlock()
	delete(workflowDefs, handle)
}

// callCallback invokes fn with the JSON request and returns the raw JSON
// response. Handles the malloc/free dance with the binding.
func callCallback(fn C.lattice_callback_fn, req []byte) ([]byte, error) {
	cReq := C.CString(string(req))
	defer C.free(unsafe.Pointer(cReq))
	buf := C.lattice_call_callback(fn, cReq, C.int(len(req)))
	if buf.data == nil {
		return nil, fmt.Errorf("callback returned null buffer")
	}
	out := C.GoBytes(unsafe.Pointer(buf.data), buf.len)
	C.free(unsafe.Pointer(buf.data)) // binding allocated, we free
	return out, nil
}
