package ontology

import (
	"encoding/json"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func TestSerializeDocument_UsesJSONFieldNamesAndRoundTrips(t *testing.T) {
	t.Parallel()

	ont := &types.Ontology{
		Workspace: types.Workspace{
			APIName:     "default",
			DisplayName: "Default",
		},
		ObjectTypes: []types.ObjectType{
			{
				APIName:    "Trip",
				PrimaryKey: "id",
				Source: types.SourceConfig{
					DatasourceAPIName: "warehouse",
					Table:             "trips",
				},
				Properties: []types.Property{
					{APIName: "id", DataType: types.DataTypeUUID},
					{
						APIName:      "status",
						DataType:     types.DataTypeString,
						DefaultValue: "planned",
						Metadata:     map[string]any{"owner": "ops"},
					},
				},
			},
		},
		Assets: []types.Asset{
			{
				APIName:      "TripEvents",
				DisplayName:  "Trip events",
				Metadata:     map[string]any{"owner": "ops"},
				Tags:         []string{"gold"},
				Properties:   []types.Property{{APIName: "trip_id", DataType: types.DataTypeUUID}},
				Dependencies: []types.AssetDependency{{Kind: "object_type", Target: "Trip"}},
				Sink:         types.AssetSink{DatasourceAPIName: "warehouse", Table: "trip_events"},
			},
		},
	}

	raw, err := SerializeDocument(ont)
	if err != nil {
		t.Fatalf("SerializeDocument: %v", err)
	}

	var exported map[string]any
	if err := json.Unmarshal(raw, &exported); err != nil {
		t.Fatalf("unmarshal exported JSON: %v", err)
	}
	if _, ok := exported["api_version"]; !ok {
		t.Fatalf("expected api_version key, got %s", string(raw))
	}
	if _, ok := exported["APIVersion"]; ok {
		t.Fatalf("unexpected CamelCase key in export: %s", string(raw))
	}

	doc, err := ParseDocument(raw)
	if err != nil {
		t.Fatalf("ParseDocument: %v", err)
	}
	if got := doc.Workspace.APIName; got != "default" {
		t.Fatalf("workspace api_name mismatch: %q", got)
	}
	if got := doc.Assets[0].Metadata["owner"]; got != "ops" {
		t.Fatalf("asset metadata mismatch: %#v", doc.Assets[0].Metadata)
	}
}
