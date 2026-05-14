// Redact filters cannot reference redacted properties — otherwise a client
// could probe sensitive values via a WHERE clause. SanitizeFilter rejects any
// filter touching a redacted property.

package policy

import (
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// SanitizeFilter inspects f and returns an error if it references any
// property in redactions.
func SanitizeFilter(f types.Filter, redactions []types.APIName) error {
	if len(redactions) == 0 || f.IsZero() {
		return nil
	}
	set := make(map[string]struct{}, len(redactions))
	for _, r := range redactions {
		set[string(r)] = struct{}{}
	}
	var bad string
	f.Walk(func(node types.Filter) {
		if bad != "" {
			return
		}
		if node.Property == "" {
			return
		}
		if _, ok := set[node.Property]; ok {
			bad = node.Property
		}
	})
	if bad != "" {
		return fmt.Errorf("filter references redacted property %q", bad)
	}
	return nil
}

// SanitizeSort rejects sort specs that reference a redacted property.
func SanitizeSort(sort []types.SortSpec, redactions []types.APIName) error {
	if len(redactions) == 0 {
		return nil
	}
	set := make(map[types.APIName]struct{}, len(redactions))
	for _, r := range redactions {
		set[r] = struct{}{}
	}
	for _, s := range sort {
		if _, ok := set[s.Property]; ok {
			return fmt.Errorf("sort references redacted property %q", s.Property)
		}
	}
	return nil
}

// ApplyToRecord drops every redacted property from r and returns the result.
func ApplyToRecord(r types.Record, redactions []types.APIName) types.Record {
	if len(redactions) == 0 {
		return r
	}
	return r.Without(redactions...)
}

// ApplyToPage drops redacted properties from every record in p.
func ApplyToPage(p types.Page, redactions []types.APIName) types.Page {
	if len(redactions) == 0 {
		return p
	}
	out := p
	out.Records = make([]types.Record, len(p.Records))
	for i, rec := range p.Records {
		out.Records[i] = ApplyToRecord(rec, redactions)
	}
	return out
}
