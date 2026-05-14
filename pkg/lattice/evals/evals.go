// Package evals is a minimal AIP-style evaluation harness. A Suite is a
// JSON-loadable collection of Cases, each with input + expected output.
// A Runner invokes a target function (typically an agent or action) for
// every case, applies a Scorer, and aggregates a Score.
//
// The runner is generic over the target signature: target functions take
// a Case.Input map and return any JSON-serializable result. Scorers are
// pure functions over (Case, result) → number-or-bool.

package evals

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"reflect"
	"time"
)

// Case is a single input/expected-output pair.
type Case struct {
	ID       string         `json:"id"`
	Tags     []string       `json:"tags,omitempty"`
	Input    map[string]any `json:"input"`
	Expected map[string]any `json:"expected,omitempty"`
}

// Suite is a named collection of evaluation cases.
type Suite struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	Cases       []Case `json:"cases"`
}

// LoadSuite reads a JSON file from disk into a Suite.
func LoadSuite(path string) (Suite, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return Suite{}, err
	}
	var s Suite
	if err := json.Unmarshal(raw, &s); err != nil {
		return Suite{}, err
	}
	return s, nil
}

// Target is the function under test. Returns the actual output for one case.
type Target func(ctx context.Context, c Case) (map[string]any, error)

// Scorer evaluates a single result. Score is in [0, 1]; Reason explains.
type Scorer func(c Case, got map[string]any) Score

// Score is the per-case scoring outcome.
type Score struct {
	Value  float64 `json:"value"`
	Pass   bool    `json:"pass"`
	Reason string  `json:"reason,omitempty"`
}

// Result is the per-case outcome including timing and error.
type Result struct {
	Case    Case           `json:"case"`
	Got     map[string]any `json:"got,omitempty"`
	Score   Score          `json:"score"`
	Err     string         `json:"err,omitempty"`
	Latency time.Duration  `json:"latency_ns"`
}

// Report aggregates results across a suite run.
type Report struct {
	Suite   string    `json:"suite"`
	Started time.Time `json:"started"`
	Ended   time.Time `json:"ended"`
	Results []Result  `json:"results"`
	Mean    float64   `json:"mean"`
	Pass    int       `json:"pass"`
	Fail    int       `json:"fail"`
}

// Run executes target against every case in s, scoring with scorer.
func Run(ctx context.Context, s Suite, target Target, scorer Scorer) (Report, error) {
	if target == nil {
		return Report{}, errors.New("evals: nil target")
	}
	if scorer == nil {
		scorer = ExactMatch
	}
	r := Report{Suite: s.Name, Started: time.Now().UTC()}
	for _, c := range s.Cases {
		t0 := time.Now()
		got, err := target(ctx, c)
		dt := time.Since(t0)
		res := Result{Case: c, Got: got, Latency: dt}
		if err != nil {
			res.Err = err.Error()
			res.Score = Score{Value: 0, Pass: false, Reason: "error: " + err.Error()}
		} else {
			res.Score = scorer(c, got)
		}
		if res.Score.Pass {
			r.Pass++
		} else {
			r.Fail++
		}
		r.Mean += res.Score.Value
		r.Results = append(r.Results, res)
	}
	r.Ended = time.Now().UTC()
	if n := len(r.Results); n > 0 {
		r.Mean /= float64(n)
	}
	return r, nil
}

// ExactMatch is the default scorer: pass iff got equals Case.Expected
// (deep equality on the JSON-marshaled forms).
func ExactMatch(c Case, got map[string]any) Score {
	if reflect.DeepEqual(c.Expected, got) {
		return Score{Value: 1, Pass: true}
	}
	return Score{Value: 0, Pass: false, Reason: "mismatch"}
}

// FieldMatch returns a Scorer that checks specific top-level fields for
// equality. Use when only a subset of the response must match.
func FieldMatch(fields ...string) Scorer {
	return func(c Case, got map[string]any) Score {
		matches := 0
		for _, f := range fields {
			if reflect.DeepEqual(c.Expected[f], got[f]) {
				matches++
			}
		}
		v := 0.0
		if len(fields) > 0 {
			v = float64(matches) / float64(len(fields))
		}
		pass := matches == len(fields)
		reason := ""
		if !pass {
			reason = fmt.Sprintf("%d/%d fields matched", matches, len(fields))
		}
		return Score{Value: v, Pass: pass, Reason: reason}
	}
}

// Threshold wraps a Scorer to require at least minScore for pass.
func Threshold(s Scorer, minScore float64) Scorer {
	return func(c Case, got map[string]any) Score {
		out := s(c, got)
		out.Pass = out.Value >= minScore
		return out
	}
}
