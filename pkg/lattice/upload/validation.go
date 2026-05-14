// ValidationEngine runs pre-load quality checks against the DiscoveredSchema.

package upload

import (
	"encoding/json"
	"fmt"
	"regexp"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// ValidationIssue is a single quality rule violation observed during
// pre-load or post-load validation.
type ValidationIssue struct {
	RuleAPIName types.APIName             `json:"rule_api_name"`
	Kind        types.QualityRuleKind     `json:"kind"`
	Property    types.APIName             `json:"property"`
	Severity    types.QualityRuleSeverity `json:"severity"`
	Message     string                    `json:"message"`
}

// ValidationResult aggregates issues from a validation run.
type ValidationResult struct {
	Errors   []ValidationIssue `json:"errors"`
	Warnings []ValidationIssue `json:"warnings"`
}

// HasErrors reports whether any blocking issues were found.
func (r ValidationResult) HasErrors() bool { return len(r.Errors) > 0 }

// ValidationEngine evaluates quality rules against discovered schema.
type ValidationEngine struct{}

// NewValidationEngine constructs a ValidationEngine.
func NewValidationEngine() *ValidationEngine { return &ValidationEngine{} }

// Validate runs every quality rule in asset against the discovered schema.
func (e *ValidationEngine) Validate(u types.Upload, asset types.Asset) ValidationResult {
	var result ValidationResult
	if u.DiscoveredSchema == nil {
		return result
	}
	for _, rule := range asset.QualityRules {
		issues := e.evaluateRule(rule, u.DiscoveredSchema)
		for _, issue := range issues {
			if issue.Severity == types.QualityRuleSeverityError {
				result.Errors = append(result.Errors, issue)
			} else {
				result.Warnings = append(result.Warnings, issue)
			}
		}
	}
	return result
}

func (e *ValidationEngine) evaluateRule(rule types.QualityRule, ds *types.DiscoveredSchema) []ValidationIssue {
	switch rule.Kind {
	case types.QualityRuleKindNotNull:
		return e.checkNotNull(rule, ds)
	case types.QualityRuleKindUnique:
		return e.checkUnique(rule, ds)
	case types.QualityRuleKindRegex:
		return e.checkRegex(rule, ds)
	case types.QualityRuleKindAllowedValues:
		return e.checkAllowedValues(rule, ds)
	case types.QualityRuleKindRange:
		return e.checkRange(rule, ds)
	case types.QualityRuleKindCustomCEL:
		return nil // Phase 2: needs CEL evaluation context
	default:
		return []ValidationIssue{{
			RuleAPIName: rule.APIName,
			Kind:        rule.Kind,
			Property:    rule.Property,
			Severity:    rule.Severity,
			Message:     fmt.Sprintf("unknown quality rule kind %q", rule.Kind),
		}}
	}
}

func (e *ValidationEngine) checkNotNull(rule types.QualityRule, ds *types.DiscoveredSchema) []ValidationIssue {
	col := findDiscoveredColumn(ds, rule.Property)
	if col == nil {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: rule.Severity,
			Message:  fmt.Sprintf("column for property %q not found in discovered schema", rule.Property),
		}}
	}
	if col.NullRate > 0 {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: rule.Severity,
			Message:  fmt.Sprintf("null rate %.2f%% violates not_null rule", col.NullRate*100),
		}}
	}
	return nil
}

func (e *ValidationEngine) checkUnique(rule types.QualityRule, ds *types.DiscoveredSchema) []ValidationIssue {
	col := findDiscoveredColumn(ds, rule.Property)
	if col == nil {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: rule.Severity,
			Message:  fmt.Sprintf("column for property %q not found in discovered schema", rule.Property),
		}}
	}
	if col.UniqueRate < 1.0 {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: rule.Severity,
			Message:  fmt.Sprintf("unique rate %.2f%% violates unique rule", col.UniqueRate*100),
		}}
	}
	return nil
}

func (e *ValidationEngine) checkRegex(rule types.QualityRule, ds *types.DiscoveredSchema) []ValidationIssue {
	col := findDiscoveredColumn(ds, rule.Property)
	if col == nil {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: rule.Severity,
			Message:  fmt.Sprintf("column for property %q not found in discovered schema", rule.Property),
		}}
	}
	var args struct{ Pattern string `json:"pattern"` }
	if err := json.Unmarshal(rule.Args, &args); err != nil || args.Pattern == "" {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: types.QualityRuleSeverityError,
			Message:  fmt.Sprintf("regex rule %q missing pattern argument", rule.APIName),
		}}
	}
	re, err := regexp.Compile(args.Pattern)
	if err != nil {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: types.QualityRuleSeverityError,
			Message:  fmt.Sprintf("invalid regex pattern %q: %v", args.Pattern, err),
		}}
	}
	for _, v := range col.SampleValues {
		if !re.MatchString(v) {
			return []ValidationIssue{{
				RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
				Severity: rule.Severity,
				Message:  fmt.Sprintf("sample value %q does not match pattern %q", v, args.Pattern),
			}}
		}
	}
	return nil
}

func (e *ValidationEngine) checkAllowedValues(rule types.QualityRule, ds *types.DiscoveredSchema) []ValidationIssue {
	col := findDiscoveredColumn(ds, rule.Property)
	if col == nil {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: rule.Severity,
			Message:  fmt.Sprintf("column for property %q not found in discovered schema", rule.Property),
		}}
	}
	var args struct{ Values []string `json:"values"` }
	if err := json.Unmarshal(rule.Args, &args); err != nil || len(args.Values) == 0 {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: types.QualityRuleSeverityError,
			Message:  fmt.Sprintf("allowed_values rule %q missing values argument", rule.APIName),
		}}
	}
	allowed := make(map[string]struct{}, len(args.Values))
	for _, v := range args.Values {
		allowed[v] = struct{}{}
	}
	for _, v := range col.SampleValues {
		if _, ok := allowed[v]; !ok {
			return []ValidationIssue{{
				RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
				Severity: rule.Severity,
				Message:  fmt.Sprintf("sample value %q not in allowed set", v),
			}}
		}
	}
	return nil
}

func (e *ValidationEngine) checkRange(rule types.QualityRule, ds *types.DiscoveredSchema) []ValidationIssue {
	col := findDiscoveredColumn(ds, rule.Property)
	if col == nil {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: rule.Severity,
			Message:  fmt.Sprintf("column for property %q not found in discovered schema", rule.Property),
		}}
	}
	var args struct {
		Min *float64 `json:"min,omitempty"`
		Max *float64 `json:"max,omitempty"`
	}
	if err := json.Unmarshal(rule.Args, &args); err != nil {
		return []ValidationIssue{{
			RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
			Severity: types.QualityRuleSeverityError,
			Message:  fmt.Sprintf("range rule %q has invalid arguments: %v", rule.APIName, err),
		}}
	}
	if args.Min == nil && args.Max == nil {
		return nil
	}
	for _, v := range col.SampleValues {
		var f float64
		if _, err := fmt.Sscanf(v, "%f", &f); err != nil {
			continue // non-numeric values are skipped in range checks on sample
		}
		if args.Min != nil && f < *args.Min {
			return []ValidationIssue{{
				RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
				Severity: rule.Severity,
				Message:  fmt.Sprintf("sample value %q is below minimum %v", v, *args.Min),
			}}
		}
		if args.Max != nil && f > *args.Max {
			return []ValidationIssue{{
				RuleAPIName: rule.APIName, Kind: rule.Kind, Property: rule.Property,
				Severity: rule.Severity,
				Message:  fmt.Sprintf("sample value %q is above maximum %v", v, *args.Max),
			}}
		}
	}
	return nil
}

func findDiscoveredColumn(ds *types.DiscoveredSchema, property types.APIName) *types.DiscoveredColumn {
	name := string(property)
	for i := range ds.Columns {
		if ds.Columns[i].Name == name {
			return &ds.Columns[i]
		}
	}
	return nil
}
