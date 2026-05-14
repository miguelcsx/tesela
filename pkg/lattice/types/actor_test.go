package types_test

import (
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func TestActor_HasRole(t *testing.T) {
	t.Parallel()

	a := types.Actor{Roles: []string{"admin", "analyst"}}
	if !a.HasRole("admin") {
		t.Fatal("HasRole(admin) must be true")
	}
	if a.HasRole("auditor") {
		t.Fatal("HasRole(auditor) must be false")
	}
}

func TestActor_Claim(t *testing.T) {
	t.Parallel()

	a := types.Actor{Claims: map[string]any{"region": "US", "tier": 3}}
	if got, ok := a.Claim("region"); !ok || got != "US" {
		t.Fatalf("Claim(region) = (%v, %v), want (US, true)", got, ok)
	}
	if _, ok := a.Claim("missing"); ok {
		t.Fatal("Claim(missing) ok must be false")
	}
}

func TestActor_IsAuthenticated(t *testing.T) {
	t.Parallel()

	if (types.Actor{}).IsAuthenticated() {
		t.Fatal("zero-value Actor must not be authenticated")
	}
	if !(types.Actor{UserID: "u1"}).IsAuthenticated() {
		t.Fatal("Actor with UserID must be authenticated")
	}
}
