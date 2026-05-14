// Pipeline is the read-side composition root. Every Get/Search/Aggregate/
// Traverse request runs the same staged execution: resolve actor, look up
// the ontology entity, evaluate policy, build the adapter request, execute,
// hydrate + redact, audit.

package query

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/audit"
	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/policy"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Deps bundles the dependencies required by the pipeline. The same instance
// is shared across all goroutines.
type Deps struct {
	Ontology *ontology.Registry
	Policies PolicyResolver
	Adapters *backend.Registry
	Audit    *audit.Writer
	Now      func() time.Time
}

// PolicyResolver returns the per-snapshot evaluator. The default
// implementation memoizes by snapshot pointer.
type PolicyResolver interface {
	For(snap *types.Ontology) (*policy.Evaluator, error)
}

// Pipeline is the read-side orchestrator.
type Pipeline struct{ deps Deps }

// NewPipeline constructs a Pipeline.
func NewPipeline(d Deps) *Pipeline {
	if d.Now == nil {
		d.Now = time.Now
	}
	return &Pipeline{deps: d}
}

// GetRequest is the input to Pipeline.Get.
type GetRequest struct {
	Actor       types.Actor
	WorkspaceID types.WorkspaceID
	ObjectType  types.APIName
	PrimaryKey  any
	RequestID   string
	TraceID     string
}

// Get returns a single object by primary key.
func (p *Pipeline) Get(ctx context.Context, req GetRequest) (types.Record, error) {
	start := p.deps.Now()
	snap, ot, err := p.lookupObject(ctx, req.WorkspaceID, req.ObjectType)
	if err != nil {
		return types.Record{}, err
	}
	dec, err := p.evaluate(snap, policy.Request{
		Actor: req.Actor, Operation: types.OperationRead,
		ResourceKind: types.KindObjectType, ResourceName: req.ObjectType,
	})
	if err != nil {
		return types.Record{}, err
	}
	if !dec.Allow {
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationRead, "", 0, start, "policy_denied")
		return types.Record{}, errs.New(errs.CodePolicyDenied, dec.Reason)
	}
	conn, ds, err := p.openAdapter(ctx, snap, ot)
	if err != nil {
		return types.Record{}, err
	}
	getter, err := backend.AsGetter(conn)
	if err != nil {
		return types.Record{}, errs.Wrap(err, errs.CodeAdapter, "capability")
	}
	rec, err := getter.Get(ctx, ot.Source, ot, req.PrimaryKey, dec.Filter)
	if err != nil {
		if errors.Is(err, backend.ErrNotFound) {
			p.audit(ctx, req.WorkspaceID, req, dec, types.OperationRead, "", 0, start, "not_found")
			return types.Record{}, errs.New(errs.CodeNotFound, "object not found")
		}
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationRead, "", 0, start, "adapter_error")
		return types.Record{}, errs.Wrap(err, errs.CodeAdapter, "adapter")
	}
	rec = policy.ApplyToRecord(rec, dec.Redactions)
	_ = ds
	p.audit(ctx, req.WorkspaceID, req, dec, types.OperationRead, fmt.Sprint(req.PrimaryKey), 1, start, "")
	return rec, nil
}

// SearchRequest is the input to Pipeline.Search.
type SearchRequest struct {
	Actor       types.Actor
	WorkspaceID types.WorkspaceID
	ObjectType  types.APIName
	Spec        types.QuerySpec
	RequestID   string
	TraceID     string
}

// Search executes a multi-row query.
func (p *Pipeline) Search(ctx context.Context, req SearchRequest) (types.Page, error) {
	start := p.deps.Now()
	snap, ot, err := p.lookupObject(ctx, req.WorkspaceID, req.ObjectType)
	if err != nil {
		return types.Page{}, err
	}
	dec, err := p.evaluate(snap, policy.Request{
		Actor: req.Actor, Operation: types.OperationSearch,
		ResourceKind: types.KindObjectType, ResourceName: req.ObjectType,
	})
	if err != nil {
		return types.Page{}, err
	}
	if !dec.Allow {
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationSearch, "", 0, start, "policy_denied")
		return types.Page{}, errs.New(errs.CodePolicyDenied, dec.Reason)
	}
	if err := policy.SanitizeFilter(req.Spec.Filter, dec.Redactions); err != nil {
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationSearch, "", 0, start, "validation_error")
		return types.Page{}, errs.Wrap(err, errs.CodeValidation, "redacted property in filter")
	}
	if err := policy.SanitizeSort(req.Spec.Sort, dec.Redactions); err != nil {
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationSearch, "", 0, start, "validation_error")
		return types.Page{}, errs.Wrap(err, errs.CodeValidation, "redacted property in sort")
	}
	if err := enforceMaxRows(snap.Workspace.Settings, &req.Spec.Page); err != nil {
		return types.Page{}, errs.Wrap(err, errs.CodeValidation, "page limit")
	}
	conn, _, err := p.openAdapter(ctx, snap, ot)
	if err != nil {
		return types.Page{}, err
	}
	searcher, err := backend.AsSearcher(conn)
	if err != nil {
		return types.Page{}, errs.Wrap(err, errs.CodeAdapter, "capability")
	}
	page, err := searcher.Search(ctx, ot.Source, ot, req.Spec, dec.Filter)
	if err != nil {
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationSearch, "", 0, start, "adapter_error")
		return types.Page{}, errs.Wrap(err, errs.CodeAdapter, "adapter")
	}
	page = policy.ApplyToPage(page, dec.Redactions)
	p.audit(ctx, req.WorkspaceID, req, dec, types.OperationSearch, "", int64(len(page.Records)), start, "")
	return page, nil
}

// AggregateRequest is the input to Pipeline.Aggregate.
type AggregateRequest struct {
	Actor       types.Actor
	WorkspaceID types.WorkspaceID
	ObjectType  types.APIName
	Spec        types.AggregateSpec
	RequestID   string
	TraceID     string
}

// Aggregate executes a grouped aggregation.
func (p *Pipeline) Aggregate(ctx context.Context, req AggregateRequest) (types.AggregateResult, error) {
	start := p.deps.Now()
	snap, ot, err := p.lookupObject(ctx, req.WorkspaceID, req.ObjectType)
	if err != nil {
		return types.AggregateResult{}, err
	}
	dec, err := p.evaluate(snap, policy.Request{
		Actor: req.Actor, Operation: types.OperationAggregate,
		ResourceKind: types.KindObjectType, ResourceName: req.ObjectType,
	})
	if err != nil {
		return types.AggregateResult{}, err
	}
	if !dec.Allow {
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationAggregate, "", 0, start, "policy_denied")
		return types.AggregateResult{}, errs.New(errs.CodePolicyDenied, dec.Reason)
	}
	conn, _, err := p.openAdapter(ctx, snap, ot)
	if err != nil {
		return types.AggregateResult{}, err
	}
	aggregator, err := backend.AsAggregator(conn)
	if err != nil {
		return types.AggregateResult{}, errs.Wrap(err, errs.CodeAdapter, "capability")
	}
	res, err := aggregator.Aggregate(ctx, ot.Source, ot, req.Spec, dec.Filter)
	if err != nil {
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationAggregate, "", 0, start, "adapter_error")
		return types.AggregateResult{}, errs.Wrap(err, errs.CodeAdapter, "adapter")
	}
	p.audit(ctx, req.WorkspaceID, req, dec, types.OperationAggregate, "", int64(len(res.Groups)), start, "")
	return res, nil
}

// ExplainSearch asks the adapter for an execution plan when it supports the
// optional SearchExplainer capability.
func (p *Pipeline) ExplainSearch(ctx context.Context, req SearchRequest) (types.QueryPlan, error) {
	snap, ot, err := p.lookupObject(ctx, req.WorkspaceID, req.ObjectType)
	if err != nil {
		return types.QueryPlan{}, err
	}
	dec, err := p.evaluate(snap, policy.Request{
		Actor: req.Actor, Operation: types.OperationSearch,
		ResourceKind: types.KindObjectType, ResourceName: req.ObjectType,
	})
	if err != nil {
		return types.QueryPlan{}, err
	}
	if !dec.Allow {
		return types.QueryPlan{}, errs.New(errs.CodePolicyDenied, dec.Reason)
	}
	conn, _, err := p.openAdapter(ctx, snap, ot)
	if err != nil {
		return types.QueryPlan{}, err
	}
	explainer, ok := conn.(backend.SearchExplainer)
	if !ok {
		return types.QueryPlan{}, errs.New(errs.CodeAdapter, "adapter does not support search explain")
	}
	plan, err := explainer.ExplainSearch(ctx, ot.Source, ot, req.Spec, dec.Filter)
	if err != nil {
		return types.QueryPlan{}, errs.Wrap(err, errs.CodeAdapter, "adapter explain")
	}
	return plan, nil
}

// ExplainAggregate asks the adapter for an execution plan when it supports
// the optional AggregateExplainer capability.
func (p *Pipeline) ExplainAggregate(ctx context.Context, req AggregateRequest) (types.QueryPlan, error) {
	snap, ot, err := p.lookupObject(ctx, req.WorkspaceID, req.ObjectType)
	if err != nil {
		return types.QueryPlan{}, err
	}
	dec, err := p.evaluate(snap, policy.Request{
		Actor: req.Actor, Operation: types.OperationAggregate,
		ResourceKind: types.KindObjectType, ResourceName: req.ObjectType,
	})
	if err != nil {
		return types.QueryPlan{}, err
	}
	if !dec.Allow {
		return types.QueryPlan{}, errs.New(errs.CodePolicyDenied, dec.Reason)
	}
	conn, _, err := p.openAdapter(ctx, snap, ot)
	if err != nil {
		return types.QueryPlan{}, err
	}
	explainer, ok := conn.(backend.AggregateExplainer)
	if !ok {
		return types.QueryPlan{}, errs.New(errs.CodeAdapter, "adapter does not support aggregate explain")
	}
	plan, err := explainer.ExplainAggregate(ctx, ot.Source, ot, req.Spec, dec.Filter)
	if err != nil {
		return types.QueryPlan{}, errs.Wrap(err, errs.CodeAdapter, "adapter explain")
	}
	return plan, nil
}

// TraverseRequest is the input to Pipeline.Traverse.
type TraverseRequest struct {
	Actor       types.Actor
	WorkspaceID types.WorkspaceID
	From        types.APIName // source object type
	LinkType    types.APIName
	SourceKey   any
	Spec        types.QuerySpec
	RequestID   string
	TraceID     string
}

// Traverse follows a link from a source row.
func (p *Pipeline) Traverse(ctx context.Context, req TraverseRequest) (types.Page, error) {
	start := p.deps.Now()
	snap, err := p.deps.Ontology.Snapshot(ctx, req.WorkspaceID)
	if err != nil {
		return types.Page{}, errs.Wrap(err, errs.CodeInternal, "load ontology")
	}
	lt, ok := snap.LinkTypeByName(req.LinkType)
	if !ok {
		return types.Page{}, errs.New(errs.CodeNotFound, "link type not found")
	}
	target, ok := snap.ObjectTypeByName(lt.ToObjectType)
	if !ok {
		return types.Page{}, errs.New(errs.CodeNotFound, "target object type missing")
	}
	dec, err := p.evaluate(snap, policy.Request{
		Actor: req.Actor, Operation: types.OperationTraverse,
		ResourceKind: types.KindObjectType, ResourceName: target.APIName,
	})
	if err != nil {
		return types.Page{}, err
	}
	if !dec.Allow {
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationTraverse, fmt.Sprint(req.SourceKey), 0, start, "policy_denied")
		return types.Page{}, errs.New(errs.CodePolicyDenied, dec.Reason)
	}
	conn, _, err := p.openAdapter(ctx, snap, target)
	if err != nil {
		return types.Page{}, err
	}
	traverser, err := backend.AsTraverser(conn)
	if err != nil {
		return types.Page{}, errs.Wrap(err, errs.CodeAdapter, "capability")
	}
	page, err := traverser.Traverse(ctx, target.Source, lt, target, []any{req.SourceKey}, req.Spec, dec.Filter)
	if err != nil {
		p.audit(ctx, req.WorkspaceID, req, dec, types.OperationTraverse, fmt.Sprint(req.SourceKey), 0, start, "adapter_error")
		return types.Page{}, errs.Wrap(err, errs.CodeAdapter, "adapter")
	}
	page = policy.ApplyToPage(page, dec.Redactions)
	p.audit(ctx, req.WorkspaceID, req, dec, types.OperationTraverse, fmt.Sprint(req.SourceKey), int64(len(page.Records)), start, "")
	return page, nil
}

// ExplainTraverse asks the adapter for an execution plan when it supports the
// optional TraverseExplainer capability.
func (p *Pipeline) ExplainTraverse(ctx context.Context, req TraverseRequest) (types.QueryPlan, error) {
	snap, err := p.deps.Ontology.Snapshot(ctx, req.WorkspaceID)
	if err != nil {
		return types.QueryPlan{}, errs.Wrap(err, errs.CodeInternal, "load ontology")
	}
	lt, ok := snap.LinkTypeByName(req.LinkType)
	if !ok {
		return types.QueryPlan{}, errs.New(errs.CodeNotFound, "link type not found")
	}
	target, ok := snap.ObjectTypeByName(lt.ToObjectType)
	if !ok {
		return types.QueryPlan{}, errs.New(errs.CodeNotFound, "target object type missing")
	}
	dec, err := p.evaluate(snap, policy.Request{
		Actor: req.Actor, Operation: types.OperationTraverse,
		ResourceKind: types.KindObjectType, ResourceName: target.APIName,
	})
	if err != nil {
		return types.QueryPlan{}, err
	}
	if !dec.Allow {
		return types.QueryPlan{}, errs.New(errs.CodePolicyDenied, dec.Reason)
	}
	conn, _, err := p.openAdapter(ctx, snap, target)
	if err != nil {
		return types.QueryPlan{}, err
	}
	explainer, ok := conn.(backend.TraverseExplainer)
	if !ok {
		return types.QueryPlan{}, errs.New(errs.CodeAdapter, "adapter does not support traverse explain")
	}
	plan, err := explainer.ExplainTraverse(ctx, target.Source, lt, target, []any{req.SourceKey}, req.Spec, dec.Filter)
	if err != nil {
		return types.QueryPlan{}, errs.Wrap(err, errs.CodeAdapter, "adapter explain")
	}
	return plan, nil
}

// lookupObject loads the snapshot + ObjectType in one go.
func (p *Pipeline) lookupObject(ctx context.Context, ws types.WorkspaceID, name types.APIName) (*types.Ontology, types.ObjectType, error) {
	snap, err := p.deps.Ontology.Snapshot(ctx, ws)
	if err != nil {
		return nil, types.ObjectType{}, errs.Wrap(err, errs.CodeInternal, "load ontology")
	}
	ot, ok := snap.ObjectTypeByName(name)
	if !ok {
		return nil, types.ObjectType{}, errs.Newf(errs.CodeNotFound, "object type %q not found", name)
	}
	return snap, ot, nil
}

// evaluate is a thin wrapper around the policy evaluator that returns errs.
func (p *Pipeline) evaluate(snap *types.Ontology, req policy.Request) (policy.Decision, error) {
	eval, err := p.deps.Policies.For(snap)
	if err != nil {
		return policy.Decision{}, errs.Wrap(err, errs.CodeInternal, "policy")
	}
	return eval.Evaluate(req), nil
}

// openAdapter resolves the datasource backing ot and returns a Connection.
func (p *Pipeline) openAdapter(ctx context.Context, snap *types.Ontology, ot types.ObjectType) (backend.Connection, types.Datasource, error) {
	ds, ok := snap.DatasourceByName(ot.Source.DatasourceAPIName)
	if !ok {
		return nil, types.Datasource{}, errs.Newf(errs.CodeInternal, "datasource %q for object type %q not found", ot.Source.DatasourceAPIName, ot.APIName)
	}
	conn, err := p.deps.Adapters.Acquire(ctx, ds)
	if err != nil {
		return nil, types.Datasource{}, errs.Wrap(err, errs.CodeAdapter, "acquire adapter")
	}
	return conn, ds, nil
}

func enforceMaxRows(s types.WorkspaceSettings, page *types.PageSpec) error {
	if s.MaxRowsPerQuery == 0 {
		return nil
	}
	limit := page.Limit
	if limit <= 0 {
		limit = s.DefaultPageSize
		if limit == 0 {
			limit = 50
		}
	}
	if int64(limit) > s.MaxRowsPerQuery {
		return fmt.Errorf("page.limit %d exceeds workspace max_rows_per_query %d", limit, s.MaxRowsPerQuery)
	}
	page.Limit = limit
	return nil
}

// audit emits an audit record asynchronously.
func (p *Pipeline) audit(ctx context.Context, ws types.WorkspaceID, req any, dec policy.Decision, op types.Operation, subject string, count int64, start time.Time, errCode string) {
	rec := types.AuditRecord{
		WorkspaceID:        ws,
		OccurredAt:         start.UTC(),
		Operation:          op,
		ResourceKind:       string(types.KindObjectType),
		PolicyDecision:     decisionToAudit(dec),
		MatchedRules:       dec.MatchedRules,
		RedactedProperties: dec.Redactions,
		ResultCount:        count,
		DurationMS:         time.Since(start).Milliseconds(),
		ErrorCode:          errCode,
		SubjectKey:         subject,
	}
	if r, ok := req.(interface{ getActor() types.Actor }); ok {
		a := r.getActor()
		rec.ActorUserID = a.UserID
		rec.ActorRoles = append([]string(nil), a.Roles...)
	}
	if r, ok := req.(interface{ getRequestID() string }); ok {
		rec.RequestID = r.getRequestID()
	}
	if r, ok := req.(interface{ getResourceName() types.APIName }); ok {
		rec.ResourceAPIName = r.getResourceName()
	}
	_ = p.deps.Audit.Write(ctx, rec)
}

func decisionToAudit(d policy.Decision) types.AuditDecision {
	if d.Allow {
		return types.AuditDecisionAllow
	}
	return types.AuditDecisionDeny
}

// getActor adapters so the audit helper can extract fields without a switch.
func (r GetRequest) getActor() types.Actor             { return r.Actor }
func (r GetRequest) getRequestID() string              { return r.RequestID }
func (r GetRequest) getResourceName() types.APIName    { return r.ObjectType }
func (r SearchRequest) getActor() types.Actor          { return r.Actor }
func (r SearchRequest) getRequestID() string           { return r.RequestID }
func (r SearchRequest) getResourceName() types.APIName { return r.ObjectType }
func (r AggregateRequest) getActor() types.Actor       { return r.Actor }
func (r AggregateRequest) getRequestID() string        { return r.RequestID }
func (r AggregateRequest) getResourceName() types.APIName {
	return r.ObjectType
}
func (r TraverseRequest) getActor() types.Actor          { return r.Actor }
func (r TraverseRequest) getRequestID() string           { return r.RequestID }
func (r TraverseRequest) getResourceName() types.APIName { return r.LinkType }
