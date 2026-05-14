// HeuristicMappingEngine proposes column mappings without calling an LLM.
// It uses name similarity, type compatibility, and historical mappings.

package upload

import (
	"fmt"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// HeuristicMappingEngine generates mapping proposals using deterministic rules.
type HeuristicMappingEngine struct{}

// NewHeuristicMappingEngine constructs a HeuristicMappingEngine.
func NewHeuristicMappingEngine() *HeuristicMappingEngine { return &HeuristicMappingEngine{} }

// MappingProposal is the result of heuristic matching.
type MappingProposal struct {
	Mappings        []types.ColumnMapping `json:"mappings"`
	UnmappedColumns []string              `json:"unmapped_columns"`
	MissingRequired []types.APIName       `json:"missing_required"`
	Conflicts       []string              `json:"conflicts"`
	Confidence      float64               `json:"confidence"`
}

// Propose generates a mapping from discovered columns to asset properties.
func (e *HeuristicMappingEngine) Propose(ds *types.DiscoveredSchema, asset types.Asset, historical []types.ColumnMapping) MappingProposal {
	var proposal MappingProposal
	matched := make(map[string]bool) // discovered column names
	propMatched := make(map[types.APIName]bool)

	// 1. Historical exact match
	for _, hist := range historical {
		col := findColumnByName(ds, hist.SourceColumn)
		if col != nil && !matched[col.Name] {
			proposal.Mappings = append(proposal.Mappings, types.ColumnMapping{
				SourceColumn:   col.Name,
				TargetProperty: hist.TargetProperty,
				Required:       hist.Required,
			})
			matched[col.Name] = true
			propMatched[hist.TargetProperty] = true
		}
	}

	// 2. Name exact match (case-insensitive)
	for i := range ds.Columns {
		col := &ds.Columns[i]
		if matched[col.Name] {
			continue
		}
		prop := findPropertyByName(asset, col.Name)
		if prop != nil && !propMatched[prop.APIName] {
			proposal.Mappings = append(proposal.Mappings, types.ColumnMapping{
				SourceColumn:   col.Name,
				TargetProperty: prop.APIName,
			})
			matched[col.Name] = true
			propMatched[prop.APIName] = true
		}
	}

	// 3. Name fuzzy match (normalize underscores, case)
	for i := range ds.Columns {
		col := &ds.Columns[i]
		if matched[col.Name] {
			continue
		}
		normalizedCol := normalizeName(col.Name)
		for _, prop := range asset.Properties {
			if propMatched[prop.APIName] {
				continue
			}
			if normalizeName(string(prop.APIName)) == normalizedCol {
				proposal.Mappings = append(proposal.Mappings, types.ColumnMapping{
					SourceColumn:   col.Name,
					TargetProperty: prop.APIName,
				})
				matched[col.Name] = true
				propMatched[prop.APIName] = true
				break
			}
		}
	}

	// 4. Type-compatible match for remaining columns
	for i := range ds.Columns {
		col := &ds.Columns[i]
		if matched[col.Name] {
			continue
		}
		for _, prop := range asset.Properties {
			if propMatched[prop.APIName] {
				continue
			}
			if typeCompatible(col.InferredType, prop.DataType) {
				proposal.Mappings = append(proposal.Mappings, types.ColumnMapping{
					SourceColumn:   col.Name,
					TargetProperty: prop.APIName,
				})
				matched[col.Name] = true
				propMatched[prop.APIName] = true
				break
			}
		}
	}

	// Collect unmapped columns
	for i := range ds.Columns {
		if !matched[ds.Columns[i].Name] {
			proposal.UnmappedColumns = append(proposal.UnmappedColumns, ds.Columns[i].Name)
		}
	}

	// Collect missing required properties
	for _, prop := range asset.Properties {
		if !prop.Nullable && !propMatched[prop.APIName] {
			proposal.MissingRequired = append(proposal.MissingRequired, prop.APIName)
		}
	}

	// Compute confidence
	if len(ds.Columns) > 0 {
		proposal.Confidence = float64(len(proposal.Mappings)) / float64(len(ds.Columns))
	}

	return proposal
}

// ValidateMapping checks whether a confirmed mapping satisfies asset requirements.
func (e *HeuristicMappingEngine) ValidateMapping(mappings []types.ColumnMapping, asset types.Asset, policy string) (warnings []string, blocking []string) {
	propMapped := make(map[types.APIName]bool)
	for _, m := range mappings {
		propMapped[m.TargetProperty] = true
	}
	for _, prop := range asset.Properties {
		if !prop.Nullable && !propMapped[prop.APIName] {
			blocking = append(blocking, fmt.Sprintf("required property %q is not mapped", prop.APIName))
		}
	}
	return warnings, blocking
}

func findColumnByName(ds *types.DiscoveredSchema, name string) *types.DiscoveredColumn {
	for i := range ds.Columns {
		if strings.EqualFold(ds.Columns[i].Name, name) {
			return &ds.Columns[i]
		}
	}
	return nil
}

func findPropertyByName(asset types.Asset, name string) *types.Property {
	for i := range asset.Properties {
		if strings.EqualFold(string(asset.Properties[i].APIName), name) {
			return &asset.Properties[i]
		}
		if asset.Properties[i].SourceColumn != "" && strings.EqualFold(asset.Properties[i].SourceColumn, name) {
			return &asset.Properties[i]
		}
	}
	return nil
}

func normalizeName(name string) string {
	return strings.ToLower(strings.ReplaceAll(name, "_", ""))
}

func typeCompatible(source, target types.DataType) bool {
	if source == target {
		return true
	}
	// Allow numeric widening
	if source.IsNumeric() && target.IsNumeric() {
		return true
	}
	// Allow string to anything (coercion)
	if source == types.DataTypeString {
		return true
	}
	return false
}
