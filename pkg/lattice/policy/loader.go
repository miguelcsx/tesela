// Loader compiles a *types.Ontology snapshot into the in-memory data
// structures the evaluator uses: rules indexed by (kind, name, op), per-role
// transitive closures of inheritance, and pre-compiled CEL programs.

package policy

import (
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// loadedRule is the runtime representation of a types.PolicyRule.
type loadedRule struct {
	source     types.PolicyRule
	conditions []conditionEval
}

// conditionEval pairs a parsed condition with its compiled evaluator.
type conditionEval struct {
	kind    types.ConditionKind
	expr    string
	program celProgram
}

// celProgram is the cached compiled CEL closure (shape of cel.Program but
// kept narrow to make the evaluator package easier to test).
type celProgram interface {
	eval(actor map[string]any, resource map[string]any, input map[string]any) (any, error)
}

// loadedRules carries the index used by the evaluator.
type loadedRules struct {
	all       []loadedRule
	roles     map[types.APIName]map[types.APIName]struct{} // role -> transitive closure
	progCache *programCache
}

// load compiles the ontology into a loadedRules bundle.
func load(o *types.Ontology) (*loadedRules, error) {
	out := &loadedRules{
		all:       make([]loadedRule, 0, len(o.PolicyRules)),
		roles:     buildRoleClosures(o.Roles),
		progCache: newProgramCache(),
	}
	for _, pr := range o.PolicyRules {
		conds := make([]conditionEval, 0, len(pr.Conditions))
		for _, c := range pr.Conditions {
			if c.Kind != types.ConditionKindCEL {
				continue
			}
			prg, err := out.progCache.get(c.Expression)
			if err != nil {
				return nil, fmt.Errorf("policy_rule %q condition: %w", pr.APIName, err)
			}
			conds = append(conds, conditionEval{
				kind:    c.Kind,
				expr:    c.Expression,
				program: celWrapper{prg: prg},
			})
		}
		out.all = append(out.all, loadedRule{source: pr, conditions: conds})
	}
	return out, nil
}

// buildRoleClosures returns role -> set of all roles inherited (including
// itself). Cycles are not possible because the validator rejects them.
func buildRoleClosures(roles []types.Role) map[types.APIName]map[types.APIName]struct{} {
	idx := make(map[types.APIName]types.Role, len(roles))
	for _, r := range roles {
		idx[r.APIName] = r
	}
	out := make(map[types.APIName]map[types.APIName]struct{}, len(roles))
	for _, r := range roles {
		closure := make(map[types.APIName]struct{})
		walkRoleClosure(r.APIName, idx, closure)
		out[r.APIName] = closure
	}
	return out
}

func walkRoleClosure(name types.APIName, idx map[types.APIName]types.Role, out map[types.APIName]struct{}) {
	if _, ok := out[name]; ok {
		return
	}
	out[name] = struct{}{}
	r, ok := idx[name]
	if !ok {
		return
	}
	for _, parent := range r.Inherits {
		walkRoleClosure(parent, idx, out)
	}
}
