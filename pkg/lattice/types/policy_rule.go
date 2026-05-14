// PolicyRule expresses what roles can do with which resources, under what
// conditions, with optional property-level redactions.

package types

import "time"

// PolicyRuleID is the canonical handle for a policy rule.
type PolicyRuleID string

// PolicyEffect declares whether a matching rule grants or denies access.
type PolicyEffect string

const (
	PolicyEffectAllow PolicyEffect = "allow"
	PolicyEffectDeny  PolicyEffect = "deny"
)

// PolicyRule is a single rule in the policy registry.
//
// Roles is the OR-set of role api_names the rule applies to. Operations is
// the OR-set of operations covered. ObjectType identifies the resource scope
// (an API name; an empty value means "all object types"). RowFilter is an
// optional structured predicate appended to every query the rule allows.
// Conditions hold attribute-based predicates evaluated by the policy engine
// (CEL expressions, ownership checks, time windows). Redactions remove the
// listed properties from query responses for actors that match this rule.
type PolicyRule struct {
	ID          PolicyRuleID `json:"id"`
	WorkspaceID WorkspaceID  `json:"workspace_id"`
	APIName     APIName      `json:"api_name"`
	DisplayName string       `json:"display_name,omitempty"`
	Description string       `json:"description,omitempty"`
	Effect      PolicyEffect `json:"effect"`
	Roles       []APIName    `json:"roles,omitempty"`
	Operations  []Operation  `json:"operations"`
	ObjectType  APIName      `json:"object_type,omitempty"`
	ActionType  APIName      `json:"action_type,omitempty"`
	RowFilter   Filter       `json:"row_filter,omitempty"`
	Conditions  []Condition  `json:"conditions,omitempty"`
	Redactions  []APIName    `json:"redactions,omitempty"`
	Priority    int          `json:"priority,omitempty"`
	CreatedAt   time.Time    `json:"created_at"`
	UpdatedAt   time.Time    `json:"updated_at"`
}

// Condition is an attribute-based predicate evaluated at request time.
//
// Currently the policy engine supports CEL expressions exclusively; future
// shorthand kinds (ownership, time_window, data_window) compile down to CEL
// and are validated at apply time.
type Condition struct {
	Kind       ConditionKind `json:"kind"`
	Expression string        `json:"expression,omitempty"`
}

// ConditionKind selects how a Condition is evaluated.
type ConditionKind string

const (
	ConditionKindCEL        ConditionKind = "cel"
	ConditionKindOwnership  ConditionKind = "ownership"
	ConditionKindTimeWindow ConditionKind = "time_window"
)
