package graph

import (
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func TestSchemaGraphShortestPathAndCycles(t *testing.T) {
	o := types.Ontology{
		ObjectTypes: []types.ObjectType{
			{APIName: "customer", Source: types.SourceConfig{Table: "customers"}, Properties: []types.Property{{APIName: "id", SourceColumn: "id"}}},
			{APIName: "order", Source: types.SourceConfig{Table: "orders"}, Properties: []types.Property{{APIName: "customer_id", SourceColumn: "customer_id"}}},
			{APIName: "invoice", Source: types.SourceConfig{Table: "invoices"}, Properties: []types.Property{{APIName: "order_id", SourceColumn: "order_id"}}},
		},
		LinkTypes: []types.LinkType{
			{APIName: "customer_orders", FromObjectType: "customer", ToObjectType: "order"},
			{APIName: "order_invoice", FromObjectType: "order", ToObjectType: "invoice"},
			{APIName: "invoice_customer", FromObjectType: "invoice", ToObjectType: "customer"},
		},
	}
	g := BuildSchemaGraph(o)
	path, ok := g.ShortestPath("customer", "invoice")
	if !ok {
		t.Fatal("expected path to exist")
	}
	if got, want := len(path.Links), 2; got != want {
		t.Fatalf("expected %d hops, got %d", want, got)
	}
	cycles := g.Cycles()
	if len(cycles) == 0 {
		t.Fatal("expected at least one cycle")
	}
}

func TestSchemaGraphLineageAndImpact(t *testing.T) {
	o := types.Ontology{
		ObjectTypes: []types.ObjectType{
			{
				APIName: "customer",
				Source:  types.SourceConfig{Table: "customers"},
				Properties: []types.Property{
					{APIName: "first_name", SourceColumn: "first_name"},
					{APIName: "last_name", SourceColumn: "last_name"},
					{
						APIName: "full_name",
						Computed: &types.ComputedProperty{
							Expression: `first_name + " " + last_name`,
							DependsOn:  []types.APIName{"first_name", "last_name"},
						},
					},
				},
			},
		},
		Assets: []types.Asset{
			{
				APIName: "customer_metrics",
				Dependencies: []types.AssetDependency{
					{Kind: "materialization", Target: "customer"},
				},
			},
		},
	}
	g := BuildSchemaGraph(o)
	edges := g.LineageEdges()
	if len(edges) < 3 {
		t.Fatalf("expected lineage edges, got %d", len(edges))
	}
	report := g.ImpactAnalysis("customer")
	if report.Node != "customer" {
		t.Fatalf("unexpected report node: %s", report.Node)
	}
}
