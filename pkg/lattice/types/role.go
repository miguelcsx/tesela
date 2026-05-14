// Role names a principal category. Roles can inherit from other roles,
// forming a transitive hierarchy that the policy loader resolves at apply time.

package types

import "time"

// RoleID is the canonical handle for a role.
type RoleID string

// Role is a named principal category.
type Role struct {
	ID          RoleID      `json:"id"`
	WorkspaceID WorkspaceID `json:"workspace_id"`
	APIName     APIName     `json:"api_name"`
	DisplayName string      `json:"display_name,omitempty"`
	Description string      `json:"description,omitempty"`
	// Inherits lists role api_names this role inherits permissions from.
	// Cycles are rejected at apply time by the ontology validator.
	Inherits  []APIName `json:"inherits,omitempty"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}
