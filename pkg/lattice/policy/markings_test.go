package policy_test

import (
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/policy"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func TestMarkingRedactions_RedactsMissingMarking(t *testing.T) {
	ot := types.ObjectType{
		APIName:    "Customer",
		PrimaryKey: "id",
		Properties: []types.Property{
			{APIName: "id"},
			{APIName: "email", Markings: []string{"pii"}},
			{APIName: "ssn", Markings: []string{"pii", "high"}},
		},
	}

	publicActor := types.Actor{Markings: nil}
	r := policy.MarkingRedactions(publicActor, ot)
	if !contains(r, "email") || !contains(r, "ssn") {
		t.Fatalf("expected email+ssn redacted, got %v", r)
	}

	piiActor := types.Actor{Markings: []string{"pii"}}
	r = policy.MarkingRedactions(piiActor, ot)
	if contains(r, "email") {
		t.Fatalf("expected email NOT redacted for pii-cleared actor")
	}
	if !contains(r, "ssn") {
		t.Fatalf("expected ssn STILL redacted (missing 'high'), got %v", r)
	}

	bothActor := types.Actor{Markings: []string{"pii", "high"}}
	r = policy.MarkingRedactions(bothActor, ot)
	if len(r) != 0 {
		t.Fatalf("expected no redactions for fully-cleared actor, got %v", r)
	}
}

func contains(slice []types.APIName, want types.APIName) bool {
	for _, x := range slice {
		if x == want {
			return true
		}
	}
	return false
}
