package lattice

import (
	"encoding/json"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func TestApplyJSON_RichSpec(t *testing.T) {
	t.Parallel()

	spec := map[string]any{
		"object_types": []map[string]any{
			{
				"api_name": "Trip",
				"properties": []map[string]any{
					{"api_name": "id", "data_type": "uuid", "primary_key": true},
					{"api_name": "origin", "data_type": "string", "indexed": true},
				},
			},
		},
		"action_types": []map[string]any{
			{
				"api_name":                 "book_trip",
				"display_name":             "Book trip",
				"description":              "Create a trip booking",
				"subject":                  "Trip",
				"permission_key":           "trip:book",
				"input_schema":             map[string]any{"trip_id": map[string]any{"type": "string"}},
				"output_schema":            map[string]any{"status": map[string]any{"type": "string"}},
				"idempotency_key_template": "{{ input.trip_id }}",
				"handler_kind":             "composite",
				"composite_steps":          []map[string]any{{"name": "charge", "action_ref": "payments.charge", "on_failure": "abort"}},
				"execution_mode":           "sync",
			},
		},
		"custom_tools": []map[string]any{
			{
				"api_name":      "lookup_fare",
				"display_name":  "Lookup fare",
				"description":   "Estimate fares",
				"kind":          "composite",
				"input_schema":  map[string]any{"origin": map[string]any{"type": "string"}},
				"output_schema": map[string]any{"fare": map[string]any{"type": "number"}},
				"composite": map[string]any{
					"steps": []map[string]any{{"name": "query", "action_ref": "fares.lookup", "on_failure": "abort"}},
				},
			},
		},
		"agents": []map[string]any{
			{
				"api_name":                     "planner",
				"display_name":                 "Planner",
				"description":                  "Plans trips",
				"system_prompt":                "Plan trips carefully",
				"model":                        map[string]any{"provider": "anthropic", "model": "claude", "temperature": 0.1, "max_tokens": 256},
				"from_object_types":            []string{"Trip"},
				"custom_tools":                 []string{"lookup_fare"},
				"context_sources":              []map[string]any{{"name": "memory", "kind": "memory", "max_items": 5}},
				"memory":                       map[string]any{"enabled": true, "namespace": "planner", "include_in_prompt": true},
				"planning":                     map[string]any{"enabled": true, "mode": "explicit", "persist_plan": true},
				"compaction":                   map[string]any{"enabled": true, "trigger_tokens": 4000},
				"subagents":                    map[string]any{"enabled": true, "agent_refs": []string{"verifier"}},
				"communication":                map[string]any{"channels": []map[string]any{{"name": "handoff", "kind": "mailbox"}}},
				"allowed_roles":                []string{"analyst"},
				"require_approval_for_actions": true,
			},
		},
		"assets": []map[string]any{
			{
				"api_name":      "TripEvents",
				"display_name":  "Trip events",
				"description":   "Normalized trip events",
				"metadata":      map[string]any{"owner": "ops"},
				"tags":          []string{"gold"},
				"properties":    []map[string]any{{"api_name": "trip_id", "data_type": "uuid"}, {"api_name": "event_type", "data_type": "string"}},
				"quality_rules": []map[string]any{{"api_name": "trip_id_present", "kind": "not_null", "property": "trip_id", "severity": "error"}},
				"dependencies":  []map[string]any{{"kind": "object_type", "target": "Trip"}},
				"sink":          map[string]any{"datasource": "warehouse", "table": "trip_events"},
				"saved_column_mapping": []map[string]any{
					{"source_column": "trip_id", "target_property": "trip_id", "required": true},
				},
				"unmapped_column_policy": "error",
			},
		},
	}
	raw, err := json.Marshal(spec)
	if err != nil {
		t.Fatalf("marshal spec: %v", err)
	}

	app := New()
	if err := app.ApplyJSON(raw); err != nil {
		t.Fatalf("ApplyJSON: %v", err)
	}

	snap := app.snapshot()
	if got := len(snap.Assets); got != 1 {
		t.Fatalf("expected 1 asset, got %d", got)
	}
	if snap.Assets[0].Metadata["owner"] != "ops" {
		t.Fatalf("asset metadata not applied: %#v", snap.Assets[0].Metadata)
	}
	if got := snap.ActionTypes[0].Handler.Kind; got != types.HandlerKindComposite {
		t.Fatalf("expected composite action handler, got %q", got)
	}
	if got := snap.CustomTools[0].Kind; got != types.CustomToolKindComposite {
		t.Fatalf("expected composite custom tool, got %q", got)
	}
	if !snap.Agents[0].RequireApprovalForActions {
		t.Fatal("agent approval gate not applied")
	}
	if got := snap.Agents[0].Memory.Namespace; got != "planner" {
		t.Fatalf("expected memory namespace planner, got %q", got)
	}
	if got := len(snap.Agents[0].Subagents.AgentRefs); got != 1 {
		t.Fatalf("expected 1 subagent ref, got %d", got)
	}
}
