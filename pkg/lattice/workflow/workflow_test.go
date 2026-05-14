package workflow_test

import (
	"context"
	"errors"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/workflow"
)

func TestEngine_RunToCompletion(t *testing.T) {
	store := workflow.NewMemoryStore()
	e := workflow.NewEngine(store)
	e.Register(workflow.Definition{
		Name: "demo",
		Steps: []workflow.Step{
			{Name: "a", Run: func(_ context.Context, s workflow.State) (workflow.State, error) {
				s["a"] = 1
				return s, nil
			}},
			{Name: "b", Run: func(_ context.Context, s workflow.State) (workflow.State, error) {
				s["b"] = 2
				return s, nil
			}},
		},
	})
	r, err := e.Start(context.Background(), "demo", workflow.State{})
	if err != nil {
		t.Fatal(err)
	}
	if r.Status != "succeeded" {
		t.Fatalf("expected succeeded, got %s", r.Status)
	}
	// JSON round-trip turns ints into floats; accept either.
	if v, ok := r.State["a"]; !ok || (v != 1 && v != float64(1)) {
		t.Fatalf("step a missing: %v", r.State)
	}
}

func TestEngine_FailureCheckpoints(t *testing.T) {
	store := workflow.NewMemoryStore()
	e := workflow.NewEngine(store)
	calls := 0
	e.Register(workflow.Definition{
		Name: "flaky",
		Steps: []workflow.Step{
			{Name: "a", Run: func(_ context.Context, s workflow.State) (workflow.State, error) {
				s["a"] = 1
				return s, nil
			}},
			{Name: "b", Run: func(_ context.Context, s workflow.State) (workflow.State, error) {
				calls++
				if calls == 1 {
					return s, errors.New("transient")
				}
				s["b"] = 2
				return s, nil
			}},
		},
	})
	r, err := e.Start(context.Background(), "flaky", nil)
	if err == nil {
		t.Fatal("expected error on first run")
	}
	if r.Status != "failed" || r.StepIndex != 1 {
		t.Fatalf("expected failed at step 1, got status=%s idx=%d", r.Status, r.StepIndex)
	}
	// Resume — should pick up at step b.
	r.Status = "running" // simulate operator marking it eligible to retry
	_ = store.Put(context.Background(), r)
	r2, err := e.Resume(context.Background(), r.ID)
	if err != nil {
		t.Fatal(err)
	}
	if r2.Status != "succeeded" {
		t.Fatalf("expected succeeded after resume, got %s", r2.Status)
	}
}
