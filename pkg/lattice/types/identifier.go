// APIName is the stable, user-facing identifier used throughout the ontology
// (object types, property names, link types, action types, roles).

package types

import (
	"errors"
	"fmt"
	"regexp"
)

// APIName is a stable identifier used as the user-facing name of an entity.
// Naming rules:
//
//   - Starts with an ASCII letter.
//   - Contains only ASCII letters, digits, underscore, and a single dot
//     separator (used for qualified names like "Order.lineItems").
//
// The single-dot separator is allowed because link type api_names follow the
// SourceType.relationshipName convention (see docs/06-domain-model/04-link-type.md).
type APIName string

var apiNameRE = regexp.MustCompile(`^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)?$`)

// String implements fmt.Stringer.
func (n APIName) String() string { return string(n) }

// Validate reports whether the name conforms to the APIName grammar.
func (n APIName) Validate() error {
	if n == "" {
		return errors.New("api_name must not be empty")
	}
	if !apiNameRE.MatchString(string(n)) {
		return fmt.Errorf("api_name %q does not match grammar [A-Za-z][A-Za-z0-9_]*(\\.[A-Za-z][A-Za-z0-9_]*)?", n)
	}
	return nil
}
