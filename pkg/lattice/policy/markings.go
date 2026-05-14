// Marking-based property redaction. Properties may declare Markings; a
// request whose Actor lacks any of those markings gets the property
// redacted from the response. This composes with the rule-based redactions
// already produced by Evaluate — the union is what the handler applies.

package policy

import (
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// MarkingRedactions returns the set of property names that must be
// redacted from results because the actor lacks the required markings.
//
// Composes with Decision.Redactions: handlers should append these to that
// slice before calling ApplyToPage / ApplyToRecord.
func MarkingRedactions(actor types.Actor, ot types.ObjectType) []types.APIName {
	if len(ot.Properties) == 0 {
		return nil
	}
	cleared := actorMarkings(actor)
	var out []types.APIName
	for _, p := range ot.Properties {
		if len(p.Markings) == 0 {
			continue
		}
		ok := true
		for _, m := range p.Markings {
			if _, has := cleared[m]; !has {
				ok = false
				break
			}
		}
		if !ok {
			out = append(out, p.APIName)
		}
	}
	return out
}

// PurposeRequired returns the policy-required purpose for the operation
// (empty string if any purpose is acceptable). Today it returns "" — the
// hook is left in place so future versions can read a workspace policy
// without changing call sites.
func PurposeRequired(_ types.PolicyRule) string { return "" }

func actorMarkings(a types.Actor) map[string]struct{} {
	out := make(map[string]struct{}, len(a.Markings))
	for _, m := range a.Markings {
		out[m] = struct{}{}
	}
	return out
}
