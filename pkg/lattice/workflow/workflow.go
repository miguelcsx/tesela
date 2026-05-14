// Package workflow defines a lightweight DAG for resumable, multi-step
// pipelines. Steps are pure functions over a typed Context; the runner
// persists checkpoints via a Store so workflows can resume after process
// restarts.
//
// This is intentionally simpler than Temporal/Airflow — no DSL, no
// dynamic scheduling, just typed Go funcs with checkpointing. Use
// Schedule.Add to invoke a workflow on a cron, or invoke directly.

package workflow

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/ids"
)

// Step is one node in a workflow DAG.
type Step struct {
	Name string
	Run  func(ctx context.Context, state State) (State, error)
}

// State is the workflow's mutable state, opaque to the runner. JSON-
// serialized at every checkpoint.
type State map[string]any

// Definition is a sequential workflow. Loops, conditionals and parallel
// branches must be encoded inside individual Step.Run bodies.
type Definition struct {
	Name  string
	Steps []Step
}

// Run is a single execution of a Definition.
type Run struct {
	ID        string    `json:"id"`
	Workflow  string    `json:"workflow"`
	StartedAt time.Time `json:"started_at"`
	UpdatedAt time.Time `json:"updated_at"`
	StepIndex int       `json:"step_index"`
	State     State     `json:"state"`
	Status    string    `json:"status"` // running | succeeded | failed
	LastError string    `json:"last_error,omitempty"`
}

// Store persists Runs so workflows can resume across restarts.
type Store interface {
	Get(ctx context.Context, id string) (Run, error)
	Put(ctx context.Context, r Run) error
	List(ctx context.Context) ([]Run, error)
}

// Engine runs workflows.
type Engine struct {
	store Store
	defs  map[string]Definition
	mu    sync.RWMutex
}

// NewEngine constructs an Engine. Pass NewMemoryStore for development.
func NewEngine(store Store) *Engine {
	return &Engine{store: store, defs: make(map[string]Definition)}
}

// Register a workflow definition under its Name.
func (e *Engine) Register(d Definition) {
	e.mu.Lock()
	defer e.mu.Unlock()
	e.defs[d.Name] = d
}

// Start launches a fresh run of the named workflow with initial state.
func (e *Engine) Start(ctx context.Context, name string, initial State) (Run, error) {
	e.mu.RLock()
	d, ok := e.defs[name]
	e.mu.RUnlock()
	if !ok {
		return Run{}, fmt.Errorf("workflow: unknown definition %q", name)
	}
	now := time.Now().UTC()
	r := Run{
		ID:        ids.NewULID(),
		Workflow:  name,
		StartedAt: now,
		UpdatedAt: now,
		State:     initial,
		Status:    "running",
	}
	if r.State == nil {
		r.State = make(State)
	}
	if err := e.store.Put(ctx, r); err != nil {
		return Run{}, err
	}
	return e.resume(ctx, r, d)
}

// Resume continues a previously checkpointed run.
func (e *Engine) Resume(ctx context.Context, runID string) (Run, error) {
	r, err := e.store.Get(ctx, runID)
	if err != nil {
		return Run{}, err
	}
	if r.Status != "running" {
		return r, nil
	}
	e.mu.RLock()
	d, ok := e.defs[r.Workflow]
	e.mu.RUnlock()
	if !ok {
		return r, fmt.Errorf("workflow: unknown definition %q", r.Workflow)
	}
	return e.resume(ctx, r, d)
}

func (e *Engine) resume(ctx context.Context, r Run, d Definition) (Run, error) {
	for r.StepIndex < len(d.Steps) {
		step := d.Steps[r.StepIndex]
		newState, err := step.Run(ctx, cloneState(r.State))
		if err != nil {
			r.Status = "failed"
			r.LastError = fmt.Sprintf("step %s: %v", step.Name, err)
			r.UpdatedAt = time.Now().UTC()
			_ = e.store.Put(ctx, r)
			return r, err
		}
		r.State = newState
		r.StepIndex++
		r.UpdatedAt = time.Now().UTC()
		// Checkpoint after every step.
		if err := e.store.Put(ctx, r); err != nil {
			return r, err
		}
	}
	r.Status = "succeeded"
	r.UpdatedAt = time.Now().UTC()
	if err := e.store.Put(ctx, r); err != nil {
		return r, err
	}
	return r, nil
}

func cloneState(s State) State {
	if s == nil {
		return State{}
	}
	raw, err := json.Marshal(s)
	if err != nil {
		// Defensive fallback — shouldn't happen for json-clean states.
		out := make(State, len(s))
		for k, v := range s {
			out[k] = v
		}
		return out
	}
	var out State
	_ = json.Unmarshal(raw, &out)
	return out
}

// MemoryStore is the default in-process workflow Store.
type MemoryStore struct {
	mu sync.RWMutex
	m  map[string]Run
}

// NewMemoryStore returns an empty in-memory Store.
func NewMemoryStore() *MemoryStore { return &MemoryStore{m: make(map[string]Run)} }

func (s *MemoryStore) Get(_ context.Context, id string) (Run, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	r, ok := s.m[id]
	if !ok {
		return Run{}, errors.New("workflow: not found")
	}
	return r, nil
}

func (s *MemoryStore) Put(_ context.Context, r Run) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.m[r.ID] = r
	return nil
}

func (s *MemoryStore) List(_ context.Context) ([]Run, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make([]Run, 0, len(s.m))
	for _, r := range s.m {
		out = append(out, r)
	}
	return out, nil
}
