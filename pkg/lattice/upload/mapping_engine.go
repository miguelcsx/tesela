// MappingEngine drives AI-proposed column mappings for uploads.

package upload

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/modelproviders"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// MappingEngine builds prompts and parses LLM responses into column mappings.
type MappingEngine struct {
	Provider    modelproviders.Provider
	ModelConfig types.ModelConfig
}

// NewMappingEngine constructs an engine with sensible defaults.
func NewMappingEngine(p modelproviders.Provider, cfg types.ModelConfig) *MappingEngine {
	return &MappingEngine{Provider: p, ModelConfig: cfg}
}

// ProposeMapping asks the model to map discovered columns to asset properties.
func (e *MappingEngine) ProposeMapping(
	ctx context.Context,
	u types.Upload,
	asset types.Asset,
) (types.Upload, error) {
	if e.Provider == nil {
		return u, fmt.Errorf("mapping engine: no provider configured")
	}
	prompt := buildMappingPrompt(asset, u.DiscoveredSchema, asset.SavedColumnMapping)
	resp, err := e.Provider.Call(ctx, modelproviders.CallRequest{
		Model:       e.ModelConfig.Model,
		Messages:    []modelproviders.Message{{Role: "user", Content: prompt}},
		Temperature: e.ModelConfig.Temperature,
		MaxTokens:   e.ModelConfig.MaxTokens,
	})
	if err != nil {
		return u, fmt.Errorf("mapping engine call: %w", err)
	}
	parsed, err := parseMappingResponse(resp.Message.Content)
	if err != nil {
		return u, fmt.Errorf("mapping engine parse: %w", err)
	}
	now := time.Now().UTC()
	u.ProposedColumnMapping = parsed.Mappings
	u.MappingConfidence = parsed.Confidence
	u.MappingProposedAt = &now
	u.MappingModelConfig = &e.ModelConfig
	return u, nil
}

type parsedMapping struct {
	Mappings   []types.ColumnMapping
	Confidence float64
}

func buildMappingPrompt(
	asset types.Asset,
	ds *types.DiscoveredSchema,
	historical []types.ColumnMapping,
) string {
	propLines := ""
	for _, p := range asset.Properties {
		propLines += fmt.Sprintf("- %s (%s)\n", p.APIName, p.DataType)
	}
	colLines := ""
	if ds != nil {
		for _, c := range ds.Columns {
			colLines += fmt.Sprintf("- %s (%s) samples: %v\n", c.Name, c.InferredType, c.SampleValues)
		}
	}
	histLines := ""
	if len(historical) > 0 {
		for _, m := range historical {
			histLines += fmt.Sprintf("- %s → %s\n", m.SourceColumn, m.TargetProperty)
		}
	} else {
		histLines = "None\n"
	}
	return fmt.Sprintf(`Target asset properties:
%s
Discovered columns:
%s
Historical mappings:
%s

Propose a mapping from each discovered column to a target property.
Return JSON with:
- "mappings": array of { "source_column": string, "target_property": string, "required": bool, "value_mapping": map[string]string }
- "confidence": float 0.0-1.0
- "reasoning": string`, propLines, colLines, histLines)
}

func parseMappingResponse(content string) (parsedMapping, error) {
	// Extract JSON block if wrapped in markdown fences.
	var raw string
	if i := jsonStartIndex(content); i >= 0 {
		j := jsonEndIndex(content, i)
		if j > i {
			raw = content[i : j+1]
		}
	}
	if raw == "" {
		raw = content
	}
	var out struct {
		Mappings   []types.ColumnMapping `json:"mappings"`
		Confidence float64               `json:"confidence"`
		Reasoning  string                `json:"reasoning"`
	}
	if err := json.Unmarshal([]byte(raw), &out); err != nil {
		return parsedMapping{}, fmt.Errorf("unmarshal mapping response: %w", err)
	}
	return parsedMapping{Mappings: out.Mappings, Confidence: out.Confidence}, nil
}

func jsonStartIndex(s string) int {
	for i, r := range s {
		if r == '{' {
			return i
		}
	}
	return -1
}

func jsonEndIndex(s string, start int) int {
	depth := 0
	for i := start; i < len(s); i++ {
		switch s[i] {
		case '{':
			depth++
		case '}':
			depth--
			if depth == 0 {
				return i
			}
		}
	}
	return -1
}
