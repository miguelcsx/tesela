// PostValidationEngine validates an upload after bulk loading has occurred.

package upload

import (
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// PostValidationEngine performs sanity checks after data has been loaded.
type PostValidationEngine struct{}

// NewPostValidationEngine constructs a PostValidationEngine.
func NewPostValidationEngine() *PostValidationEngine { return &PostValidationEngine{} }

// Validate checks the bulk load result and asset expectations.
func (e *PostValidationEngine) Validate(u types.Upload, asset types.Asset, result backend.BulkLoadResult) ValidationResult {
	var res ValidationResult
	if result.RowsLoaded == 0 {
		res.Errors = append(res.Errors, ValidationIssue{
			RuleAPIName: "post_load",
			Kind:        types.QualityRuleKindCustomCEL,
			Property:    "",
			Severity:    types.QualityRuleSeverityError,
			Message:     "bulk load produced zero rows",
		})
	}
	if meta, ok := asset.Metadata["expected_min_rows"]; ok {
		var minRows int64
		switch v := meta.(type) {
		case int:
			minRows = int64(v)
		case int64:
			minRows = v
		case float64:
			minRows = int64(v)
		}
		if minRows > 0 && result.RowsLoaded < minRows {
			res.Errors = append(res.Errors, ValidationIssue{
				RuleAPIName: "expected_min_rows",
				Kind:        types.QualityRuleKindCustomCEL,
				Property:    "",
				Severity:    types.QualityRuleSeverityError,
				Message:     fmt.Sprintf("loaded %d rows, expected at least %d", result.RowsLoaded, minRows),
			})
		}
	}
	return res
}
