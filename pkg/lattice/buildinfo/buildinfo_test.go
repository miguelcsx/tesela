package buildinfo

import "testing"

func TestCurrent_DefaultValues(t *testing.T) {
	t.Parallel()

	got := Current()
	if got.Version == "" {
		t.Fatal("Version must not be empty")
	}
	if got.Commit == "" {
		t.Fatal("Commit must not be empty")
	}
	if got.Date == "" {
		t.Fatal("Date must not be empty")
	}
}

func TestCurrent_ReflectsVarOverrides(t *testing.T) {
	origV, origC, origD := Version, Commit, Date
	t.Cleanup(func() {
		Version, Commit, Date = origV, origC, origD
	})

	Version = "v1.2.3"
	Commit = "abc1234"
	Date = "2026-05-06T00:00:00Z"

	got := Current()
	if got.Version != "v1.2.3" || got.Commit != "abc1234" || got.Date != "2026-05-06T00:00:00Z" {
		t.Fatalf("Current did not reflect overrides: %+v", got)
	}
}
