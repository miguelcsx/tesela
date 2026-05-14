// Evaluator answers "is this request allowed?" for a single ontology
// snapshot. It is constructed once per snapshot via NewEvaluator and used
// concurrently across requests.

package policy

import (
	"sort"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Evaluator is the per-snapshot policy decision maker.
type Evaluator struct {
	rules *loadedRules
}

// NewEvaluator builds an Evaluator from the supplied snapshot.
func NewEvaluator(o *types.Ontology) (*Evaluator, error) {
	loaded, err := load(o)
	if err != nil {
		return nil, err
	}
	return &Evaluator{rules: loaded}, nil
}

// Evaluate runs the policy decision for req. The result includes:
//   - Allow: whether at least one allow rule matched and no deny matched.
//   - Filter: combined row filter from every matching allow rule.
//   - Redactions: union of redacted properties across matching rules.
//   - MatchedRules: every rule that contributed to the decision.
func (e *Evaluator) Evaluate(req Request) Decision {
	matches := e.matchingRules(req)
	if len(matches) == 0 {
		return Decision{Reason: "no rule matched"}
	}
	dec := Decision{}
	allowFilters := make([]types.Filter, 0, len(matches))
	deniedBy := types.APIName("")
	for _, m := range matches {
		dec.MatchedRules = append(dec.MatchedRules, m.source.APIName)
		if m.source.Effect == types.PolicyEffectDeny {
			deniedBy = m.source.APIName
			continue
		}
		dec.Allow = true
		if !m.source.RowFilter.IsZero() {
			allowFilters = append(allowFilters, m.source.RowFilter)
		}
		dec.Redactions = append(dec.Redactions, m.source.Redactions...)
	}
	if deniedBy != "" {
		dec.Allow = false
		dec.Reason = "denied by " + string(deniedBy)
		return dec
	}
	if !dec.Allow {
		dec.Reason = "no allow rule matched"
		return dec
	}
	dec.Filter = combineFilters(allowFilters)
	dec.Redactions = uniqAPINames(dec.Redactions)
	return dec
}

// matchingRules returns every rule whose static parameters (role, operation,
// resource scope) and dynamic conditions (CEL) match req.
func (e *Evaluator) matchingRules(req Request) []loadedRule {
	out := make([]loadedRule, 0, 4)
	actorRoles := actorRoleSet(req.Actor, e.rules.roles)
	actorMap := actorContext(req.Actor)
	resourceMap := resourceContext(req)

	for _, r := range e.rules.all {
		if !ruleAppliesToRole(r.source, actorRoles) {
			continue
		}
		if !ruleAppliesToOperation(r.source, req.Operation) {
			continue
		}
		if !ruleAppliesToResource(r.source, req) {
			continue
		}
		if !evalConditions(r.conditions, actorMap, resourceMap, req.Input) {
			continue
		}
		out = append(out, r)
	}
	sort.Slice(out, func(i, j int) bool {
		return out[i].source.Priority > out[j].source.Priority
	})
	return out
}

// actorRoleSet expands an actor's roles via the inheritance closure.
func actorRoleSet(a types.Actor, closures map[types.APIName]map[types.APIName]struct{}) map[types.APIName]struct{} {
	out := make(map[types.APIName]struct{}, 4)
	for _, r := range a.Roles {
		out[types.APIName(r)] = struct{}{}
		for n := range closures[types.APIName(r)] {
			out[n] = struct{}{}
		}
	}
	return out
}

func ruleAppliesToRole(r types.PolicyRule, actorRoles map[types.APIName]struct{}) bool {
	if len(r.Roles) == 0 {
		return true
	}
	for _, role := range r.Roles {
		if _, ok := actorRoles[role]; ok {
			return true
		}
	}
	return false
}

func ruleAppliesToOperation(r types.PolicyRule, op types.Operation) bool {
	for _, ro := range r.Operations {
		if ro == op {
			return true
		}
	}
	return false
}

func ruleAppliesToResource(r types.PolicyRule, req Request) bool {
	switch req.ResourceKind {
	case types.KindObjectType:
		if r.ObjectType == "" {
			return r.ActionType == ""
		}
		return r.ObjectType == req.ResourceName
	case types.KindActionType:
		if r.ActionType == "" {
			return r.ObjectType == ""
		}
		return r.ActionType == req.ResourceName
	default:
		return r.ObjectType == "" && r.ActionType == ""
	}
}

func evalConditions(conds []conditionEval, actor, resource map[string]any, input map[string]any) bool {
	for _, c := range conds {
		v, err := c.program.eval(actor, resource, input)
		if err != nil {
			return false
		}
		b, ok := v.(bool)
		if !ok || !b {
			return false
		}
	}
	return true
}

func actorContext(a types.Actor) map[string]any {
	roles := make([]any, 0, len(a.Roles))
	for _, r := range a.Roles {
		roles = append(roles, r)
	}
	return map[string]any{
		"user_id":      a.UserID,
		"workspace_id": string(a.WorkspaceID),
		"roles":        roles,
		"claims":       a.Claims,
	}
}

func resourceContext(req Request) map[string]any {
	out := map[string]any{
		"kind":     string(req.ResourceKind),
		"api_name": string(req.ResourceName),
	}
	if req.Subject.Values != nil {
		props := make(map[string]any, len(req.Subject.Values))
		for k, v := range req.Subject.Values {
			props[string(k)] = v
		}
		out["values"] = props
	}
	return out
}

func combineFilters(in []types.Filter) types.Filter {
	if len(in) == 0 {
		return types.Filter{}
	}
	if len(in) == 1 {
		return in[0]
	}
	return types.OrFilters(in...)
}

func uniqAPINames(in []types.APIName) []types.APIName {
	if len(in) == 0 {
		return nil
	}
	seen := make(map[types.APIName]struct{}, len(in))
	out := in[:0]
	for _, n := range in {
		if _, ok := seen[n]; ok {
			continue
		}
		seen[n] = struct{}{}
		out = append(out, n)
	}
	return out
}
