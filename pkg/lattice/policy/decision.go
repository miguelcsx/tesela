// Decision is the structured result of an evaluator call. It is the only
// thing the query/action/agent pipelines need.

package policy

import "github.com/miguelcsx/lattice/pkg/lattice/types"

// Decision is the outcome of evaluating policy for a single request.
type Decision struct {
	Allow        bool            `json:"allow"`
	Filter       types.Filter    `json:"filter,omitempty"`
	Redactions   []types.APIName `json:"redactions,omitempty"`
	MatchedRules []types.APIName `json:"matched_rules,omitempty"`
	Reason       string          `json:"reason,omitempty"`
}

// IsAllowed is a convenience for clients that only need the allow flag.
func (d Decision) IsAllowed() bool { return d.Allow }

// Request is the input to Evaluator.Evaluate. The kind/name pair identifies
// the resource (object_type or action_type api_name), and subject is the
// optional pre-resolved subject row (used by action evaluations).
type Request struct {
	Actor        types.Actor
	Operation    types.Operation
	ResourceKind types.Kind
	ResourceName types.APIName
	Subject      types.Record
	Input        map[string]any
}
