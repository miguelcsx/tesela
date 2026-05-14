// Pipeline is the action runtime composition root. Execute runs the full
// 10-stage flow declarative dispatch from API or worker.

package actions

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/audit"
	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/policy"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Deps bundles the dependencies the pipeline needs.
type Deps struct {
	Store      *storage.Store
	Ontology   *ontology.Registry
	Policies   PolicyResolver
	Adapters   *backend.Registry
	Dispatcher *Dispatcher
	Audit      *audit.Writer
	Now        func() time.Time
}

// PolicyResolver mirrors query.PolicyResolver — duplicated here to avoid an
// internal/query → internal/actions cycle.
type PolicyResolver interface {
	For(snap *types.Ontology) (*policy.Evaluator, error)
}

// Pipeline is the action runtime entry point.
type Pipeline struct {
	deps    Deps
	schemas *schemaCache
}

// NewPipeline builds a Pipeline.
func NewPipeline(d Deps) *Pipeline {
	if d.Now == nil {
		d.Now = time.Now
	}
	return &Pipeline{deps: d, schemas: newSchemaCache()}
}

// ExecuteRequest is the input to Execute.
type ExecuteRequest struct {
	Actor          types.Actor
	WorkspaceID    types.WorkspaceID
	ActionTypeName types.APIName
	Input          map[string]any
	IdempotencyKey string
	RequestID      string
	TraceID        string
	SubjectKey     string // optional; pre-resolved primary key string
}

// ExecuteResult is what the API returns to the client.
type ExecuteResult struct {
	RunID  types.ActionRunID `json:"run_id"`
	Status types.RunStatus   `json:"status"`
	Output json.RawMessage   `json:"output,omitempty"`
}

// Execute runs the action synchronously. Async dispatch is the caller's
// responsibility (it calls into a worker handler that re-enters this method).
func (p *Pipeline) Execute(ctx context.Context, req ExecuteRequest) (ExecuteResult, error) {
	start := p.deps.Now()
	ws, snap, at, err := p.lookup(ctx, req.WorkspaceID, req.ActionTypeName)
	if err != nil {
		return ExecuteResult{}, err
	}
	if err := p.validate(at, req.Input); err != nil {
		return ExecuteResult{}, err
	}
	subjectType, subjectRecord, srcConfig, ds, err := p.resolveSubject(ctx, snap, at, req)
	if err != nil {
		return ExecuteResult{}, err
	}
	dec, err := p.evaluate(snap, req, subjectRecord)
	if err != nil {
		return ExecuteResult{}, err
	}
	if !dec.Allow {
		return ExecuteResult{}, errs.New(errs.CodePolicyDenied, dec.Reason)
	}
	if existing, ok, err := p.checkIdempotency(ctx, req); err != nil {
		return ExecuteResult{}, err
	} else if ok {
		return existing, nil
	}
	run, err := p.createRun(ctx, ws, at, req)
	if err != nil {
		return ExecuteResult{}, err
	}
	conn, err := p.acquireConnection(ctx, at, ds)
	if err != nil {
		return p.fail(ctx, ws, run, err, start, dec)
	}
	out, dispatchErr := p.deps.Dispatcher.Dispatch(ctx, DispatchEvent{
		Workspace:      ws,
		ActionType:     at,
		SubjectType:    subjectType,
		Subject:        subjectRecord,
		SourceConfig:   srcConfig,
		Input:          req.Input,
		Actor:          req.Actor,
		IdempotencyKey: req.IdempotencyKey,
		Datasource:     ds,
		Connection:     conn,
	})
	if dispatchErr != nil {
		return p.fail(ctx, ws, run, dispatchErr, start, dec)
	}
	run, err = p.complete(ctx, run, out)
	if err != nil {
		return ExecuteResult{}, err
	}
	p.audit(ctx, ws.ID, req, dec, types.OperationExecute, "", 1, start, "")
	return ExecuteResult{RunID: run.ID, Status: run.Status, Output: out.Output}, nil
}

func (p *Pipeline) lookup(ctx context.Context, wsID types.WorkspaceID, name types.APIName) (types.Workspace, *types.Ontology, types.ActionType, error) {
	snap, err := p.deps.Ontology.Snapshot(ctx, wsID)
	if err != nil {
		return types.Workspace{}, nil, types.ActionType{}, errs.Wrap(err, errs.CodeInternal, "ontology")
	}
	at, ok := snap.ActionTypeByName(name)
	if !ok {
		return types.Workspace{}, nil, types.ActionType{}, errs.Newf(errs.CodeNotFound, "action type %q not found", name)
	}
	return snap.Workspace, snap, at, nil
}

func (p *Pipeline) validate(at types.ActionType, input map[string]any) error {
	if err := p.schemas.validate(string(at.APIName), at.InputSchema, input); err != nil {
		return errs.Wrap(err, errs.CodeValidation, "input")
	}
	return nil
}

func (p *Pipeline) resolveSubject(ctx context.Context, snap *types.Ontology, at types.ActionType, req ExecuteRequest) (*types.ObjectType, *types.Record, types.SourceConfig, types.Datasource, error) {
	if at.Subject == "" {
		return nil, nil, types.SourceConfig{}, types.Datasource{}, nil
	}
	ot, ok := snap.ObjectTypeByName(at.Subject)
	if !ok {
		return nil, nil, types.SourceConfig{}, types.Datasource{}, errs.Newf(errs.CodeInternal, "subject %q not found", at.Subject)
	}
	ds, ok := snap.DatasourceByName(ot.Source.DatasourceAPIName)
	if !ok {
		return nil, nil, types.SourceConfig{}, types.Datasource{}, errs.Newf(errs.CodeInternal, "datasource %q not found", ot.Source.DatasourceAPIName)
	}
	if req.SubjectKey == "" {
		// Some actions (Create) have a subject type but no subject record yet.
		return &ot, nil, ot.Source, ds, nil
	}
	conn, err := p.deps.Adapters.Acquire(ctx, ds)
	if err != nil {
		return nil, nil, types.SourceConfig{}, types.Datasource{}, errs.Wrap(err, errs.CodeAdapter, "adapter")
	}
	getter, err := backend.AsGetter(conn)
	if err != nil {
		return nil, nil, types.SourceConfig{}, types.Datasource{}, errs.Wrap(err, errs.CodeAdapter, "capability")
	}
	rec, err := getter.Get(ctx, ot.Source, ot, req.SubjectKey, types.Filter{})
	if err != nil {
		if errors.Is(err, backend.ErrNotFound) {
			return nil, nil, types.SourceConfig{}, types.Datasource{}, errs.Newf(errs.CodeNotFound, "subject %s not found", req.SubjectKey)
		}
		return nil, nil, types.SourceConfig{}, types.Datasource{}, errs.Wrap(err, errs.CodeAdapter, "load subject")
	}
	return &ot, &rec, ot.Source, ds, nil
}

func (p *Pipeline) evaluate(snap *types.Ontology, req ExecuteRequest, subject *types.Record) (policy.Decision, error) {
	eval, err := p.deps.Policies.For(snap)
	if err != nil {
		return policy.Decision{}, errs.Wrap(err, errs.CodeInternal, "policy")
	}
	preq := policy.Request{
		Actor:        req.Actor,
		Operation:    types.OperationExecute,
		ResourceKind: types.KindActionType,
		ResourceName: req.ActionTypeName,
		Input:        req.Input,
	}
	if subject != nil {
		preq.Subject = *subject
	}
	return eval.Evaluate(preq), nil
}

func (p *Pipeline) checkIdempotency(ctx context.Context, req ExecuteRequest) (ExecuteResult, bool, error) {
	if req.IdempotencyKey == "" {
		return ExecuteResult{}, false, nil
	}
	existing, err := p.deps.Store.ActionRuns().GetByIdempotencyKey(ctx, req.WorkspaceID, req.IdempotencyKey)
	if err != nil {
		if errors.Is(err, storage.ErrNotFound) {
			return ExecuteResult{}, false, nil
		}
		return ExecuteResult{}, false, errs.Wrap(err, errs.CodeInternal, "idempotency lookup")
	}
	return ExecuteResult{RunID: existing.ID, Status: existing.Status, Output: existing.Output}, true, nil
}

func (p *Pipeline) createRun(ctx context.Context, ws types.Workspace, at types.ActionType, req ExecuteRequest) (types.ActionRun, error) {
	input, _ := json.Marshal(req.Input)
	ar := types.ActionRun{
		ID:             types.ActionRunID(ids.NewULID()),
		WorkspaceID:    ws.ID,
		ActionType:     at.APIName,
		IdempotencyKey: req.IdempotencyKey,
		Subject:        req.SubjectKey,
		ActorUserID:    req.Actor.UserID,
		ActorRoles:     append([]string(nil), req.Actor.Roles...),
		Input:          input,
		Status:         types.RunStatusRunning,
	}
	created, err := p.deps.Store.ActionRuns().Create(ctx, ar)
	if err != nil {
		if errors.Is(err, storage.ErrConflict) && req.IdempotencyKey != "" {
			existing, lookupErr := p.deps.Store.ActionRuns().GetByIdempotencyKey(ctx, ws.ID, req.IdempotencyKey)
			if lookupErr == nil {
				return existing, nil
			}
		}
		return types.ActionRun{}, errs.Wrap(err, errs.CodeInternal, "create run")
	}
	return created, nil
}

func (p *Pipeline) acquireConnection(ctx context.Context, at types.ActionType, ds types.Datasource) (backend.Connection, error) {
	if at.Handler.Kind != types.HandlerKindCRUDCreate &&
		at.Handler.Kind != types.HandlerKindCRUDUpdate &&
		at.Handler.Kind != types.HandlerKindCRUDDelete {
		return nil, nil
	}
	conn, err := p.deps.Adapters.Acquire(ctx, ds)
	if err != nil {
		return nil, errs.Wrap(err, errs.CodeAdapter, "adapter")
	}
	return conn, nil
}

func (p *Pipeline) complete(ctx context.Context, run types.ActionRun, out DispatchResult) (types.ActionRun, error) {
	now := p.deps.Now().UTC()
	run.Status = types.RunStatusDone
	run.Output = out.Output
	run.FinishedAt = &now
	updated, err := p.deps.Store.ActionRuns().Update(ctx, run)
	if err != nil {
		return run, errs.Wrap(err, errs.CodeInternal, "update run")
	}
	return updated, nil
}

func (p *Pipeline) fail(ctx context.Context, ws types.Workspace, run types.ActionRun, dispatchErr error, start time.Time, dec policy.Decision) (ExecuteResult, error) {
	now := p.deps.Now().UTC()
	run.Status = types.RunStatusFailed
	run.ErrorCode = "internal_error"
	if le, ok := errs.As(dispatchErr); ok {
		run.ErrorCode = string(le.Code)
		run.ErrorMessage = le.Message
	} else {
		run.ErrorMessage = dispatchErr.Error()
	}
	run.FinishedAt = &now
	if _, upErr := p.deps.Store.ActionRuns().Update(ctx, run); upErr != nil {
		// Best-effort: dispatch error already happened; log via slog default.
	}
	p.audit(ctx, ws.ID, ExecuteRequest{Actor: types.Actor{UserID: run.ActorUserID}, ActionTypeName: run.ActionType}, dec, types.OperationExecute, "", 0, start, run.ErrorCode)
	return ExecuteResult{RunID: run.ID, Status: run.Status}, errs.Wrap(dispatchErr, errs.CodeInternal, "action dispatch")
}

func (p *Pipeline) audit(ctx context.Context, ws types.WorkspaceID, req ExecuteRequest, dec policy.Decision, op types.Operation, subject string, count int64, start time.Time, errCode string) {
	rec := types.AuditRecord{
		WorkspaceID:        ws,
		OccurredAt:         start.UTC(),
		Operation:          op,
		ResourceKind:       string(types.KindActionType),
		ResourceAPIName:    req.ActionTypeName,
		PolicyDecision:     decisionToAudit(dec),
		MatchedRules:       dec.MatchedRules,
		RedactedProperties: dec.Redactions,
		ActorUserID:        req.Actor.UserID,
		ActorRoles:         append([]string(nil), req.Actor.Roles...),
		ResultCount:        count,
		DurationMS:         time.Since(start).Milliseconds(),
		ErrorCode:          errCode,
		SubjectKey:         subject,
		RequestID:          req.RequestID,
	}
	_ = p.deps.Audit.Write(ctx, rec)
	_ = fmt.Sprintf // keep fmt usage stable
}

func decisionToAudit(d policy.Decision) types.AuditDecision {
	if d.Allow {
		return types.AuditDecisionAllow
	}
	return types.AuditDecisionDeny
}
