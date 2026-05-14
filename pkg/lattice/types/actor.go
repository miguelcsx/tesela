// Actor is the principal of a request — assembled from a validated identity
// token at the API edge and propagated through every pipeline stage.

package types

// Actor is the principal performing an operation.
//
// UserID is the stable identifier of the human or service. WorkspaceID scopes
// every authorization check to a single tenant. Roles and Claims drive policy
// evaluation: roles are checked against PolicyRule subjects, and Claims feed
// into CEL custom_expression evaluation.
type Actor struct {
	UserID      string         `json:"user_id"`
	WorkspaceID string         `json:"workspace_id"`
	Roles       []string       `json:"roles,omitempty"`
	Claims      map[string]any `json:"claims,omitempty"`

	// Markings the actor is cleared to see. A request fails open (deny) when
	// touching data tagged with a marking the actor lacks. Mirrors Foundry's
	// security marking model.
	Markings []string `json:"markings,omitempty"`
	// Purpose is the legal/contractual reason for this access (e.g. "audit",
	// "billing", "fraud-investigation"). Policy rules may require purpose.
	Purpose string `json:"purpose,omitempty"`
}

// HasMarking reports whether the actor is cleared for the given marking.
func (a Actor) HasMarking(m string) bool {
	for _, x := range a.Markings {
		if x == m {
			return true
		}
	}
	return false
}

// HasRole reports whether the actor carries the given role string. Comparison
// is case-sensitive and exact.
func (a Actor) HasRole(role string) bool {
	for _, r := range a.Roles {
		if r == role {
			return true
		}
	}
	return false
}

// Claim returns the claim value at key, with ok=false when absent.
func (a Actor) Claim(key string) (any, bool) {
	if a.Claims == nil {
		return nil, false
	}
	v, ok := a.Claims[key]
	return v, ok
}

// IsAuthenticated reports whether an identity has been resolved.
// The zero-value Actor (anonymous) returns false.
func (a Actor) IsAuthenticated() bool { return a.UserID != "" }
