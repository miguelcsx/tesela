package upload

import (
	"context"
	"encoding/json"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/modelproviders"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type mockProvider struct {
	response string
}

func (m *mockProvider) Name() string { return "mock" }

func (m *mockProvider) Call(_ context.Context, _ modelproviders.CallRequest) (modelproviders.CallResponse, error) {
	return modelproviders.CallResponse{
		Message: modelproviders.Message{Role: "assistant", Content: m.response},
	}, nil
}

func TestProposeMapping(t *testing.T) {
	resp := map[string]any{
		"mappings": []map[string]any{
			{"source_column": "col_a", "target_property": "origin", "required": true},
		},
		"confidence": 0.92,
		"reasoning":  "direct match",
	}
	raw, _ := json.Marshal(resp)
	engine := NewMappingEngine(
		&mockProvider{response: string(raw)},
		types.ModelConfig{Provider: "mock", Model: "m"},
	)

	u := types.Upload{
		DiscoveredSchema: &types.DiscoveredSchema{
			Format: "csv",
			Columns: []types.DiscoveredColumn{
				{Name: "col_a", InferredType: types.DataTypeString, SampleValues: []string{"NYC"}},
			},
		},
	}
	asset := types.Asset{
		APIName: "trips",
		Properties: []types.Property{
			{APIName: "origin", DataType: types.DataTypeString},
		},
	}

	out, err := engine.ProposeMapping(context.Background(), u, asset)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(out.ProposedColumnMapping) != 1 {
		t.Fatalf("expected 1 mapping, got %d", len(out.ProposedColumnMapping))
	}
	if out.ProposedColumnMapping[0].TargetProperty != "origin" {
		t.Fatalf("expected target origin, got %s", out.ProposedColumnMapping[0].TargetProperty)
	}
	if out.MappingConfidence != 0.92 {
		t.Fatalf("expected confidence 0.92, got %f", out.MappingConfidence)
	}
	if out.MappingProposedAt == nil {
		t.Fatal("expected MappingProposedAt to be set")
	}
}
