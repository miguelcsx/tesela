package evals_test

import (
	"context"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/evals"
)

func TestRun_ExactMatch(t *testing.T) {
	s := evals.Suite{
		Name: "smoke",
		Cases: []evals.Case{
			{ID: "1", Input: map[string]any{"a": 1}, Expected: map[string]any{"a": 1}},
			{ID: "2", Input: map[string]any{"a": 2}, Expected: map[string]any{"a": 99}},
		},
	}
	target := func(_ context.Context, c evals.Case) (map[string]any, error) {
		return c.Input, nil
	}
	r, err := evals.Run(context.Background(), s, target, nil)
	if err != nil {
		t.Fatal(err)
	}
	if r.Pass != 1 || r.Fail != 1 {
		t.Fatalf("expected 1/1, got %d/%d", r.Pass, r.Fail)
	}
	if r.Mean != 0.5 {
		t.Fatalf("expected mean 0.5, got %v", r.Mean)
	}
}

func TestRun_FieldMatch(t *testing.T) {
	s := evals.Suite{
		Name: "fm",
		Cases: []evals.Case{
			{ID: "1", Input: map[string]any{}, Expected: map[string]any{"name": "x", "age": 10}},
		},
	}
	target := func(_ context.Context, _ evals.Case) (map[string]any, error) {
		return map[string]any{"name": "x", "age": 11, "extra": "ignored"}, nil
	}
	r, _ := evals.Run(context.Background(), s, target, evals.FieldMatch("name", "age"))
	if r.Results[0].Score.Pass {
		t.Fatalf("expected fail (age mismatch)")
	}
	if r.Results[0].Score.Value != 0.5 {
		t.Fatalf("expected 0.5, got %v", r.Results[0].Score.Value)
	}
}

func TestRun_TargetError(t *testing.T) {
	s := evals.Suite{Cases: []evals.Case{{ID: "1"}}}
	target := func(_ context.Context, _ evals.Case) (map[string]any, error) {
		return nil, errCustom{}
	}
	r, _ := evals.Run(context.Background(), s, target, nil)
	if r.Fail != 1 || r.Results[0].Err == "" {
		t.Fatalf("expected error captured, got %+v", r.Results[0])
	}
}

type errCustom struct{}

func (errCustom) Error() string { return "boom" }
