// Convert maps between upload-level and backend-level column mapping types.

package upload

import (
	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// ToBackendColumnMapping converts the canonical lattice mapping to the
// backend adapter contract.
func ToBackendColumnMapping(cm []types.ColumnMapping) backend.ColumnMapping {
	entries := make([]backend.ColumnMappingEntry, 0, len(cm))
	for _, m := range cm {
		var transform string
		if len(m.Transforms) > 0 {
			// If any transform carries a CEL expression, pass the first one.
			// Full transform chains are adapter-specific and live outside Lattice.
			if cel, ok := m.Transforms[0].Config["cel"].(string); ok {
				transform = cel
			}
		}
		entries = append(entries, backend.ColumnMappingEntry{
			SourceColumn:   m.SourceColumn,
			TargetProperty: m.TargetProperty,
			Transform:      transform,
		})
	}
	return backend.ColumnMapping{Entries: entries}
}
