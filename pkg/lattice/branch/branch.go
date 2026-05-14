// Package branch implements ontology branching and version promotion.
// A branch is a mutable working copy of an ontology snapshot. Two
// supported lifecycles:
//
//   - Draft → Review → Published. The Review state is a soft gate;
//     the only enforcement is that promotion requires a non-empty
//     reviewer list (configurable per branch).
//   - main / feature branches like git: branch from a base, mutate,
//     diff against base, merge back (last-write-wins on conflicts).
//
// Storage is pluggable via the Store interface; an in-memory
// implementation is provided for development and testing.

package branch

import (
	"context"
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Lifecycle is the high-level branch state machine.
type Lifecycle string

const (
	LifecycleDraft     Lifecycle = "draft"
	LifecycleReview    Lifecycle = "review"
	LifecyclePublished Lifecycle = "published"
)

// Branch is a named, mutable copy of an ontology with a base reference.
type Branch struct {
	ID         string
	Name       string
	BaseBranch string
	Snapshot   types.Ontology
	Lifecycle  Lifecycle
	CreatedBy  string
	CreatedAt  time.Time
	UpdatedAt  time.Time
	Reviewers  []string
	Notes      string
}

// Store abstracts branch persistence. Implementations must be safe for
// concurrent use.
type Store interface {
	Get(ctx context.Context, name string) (Branch, error)
	List(ctx context.Context) ([]Branch, error)
	Put(ctx context.Context, b Branch) error
	Delete(ctx context.Context, name string) error
}

// Manager is the public API for branch operations. Composition over the
// Store keeps mutation logic independent of how branches are persisted.
type Manager struct {
	store Store
}

// NewManager wires a Manager to a Store.
func NewManager(store Store) *Manager { return &Manager{store: store} }

// Create a branch off base. If base is "" the branch starts empty.
func (m *Manager) Create(ctx context.Context, name, base, createdBy string, snap types.Ontology) (Branch, error) {
	if name == "" {
		return Branch{}, errors.New("branch: name required")
	}
	if _, err := m.store.Get(ctx, name); err == nil {
		return Branch{}, fmt.Errorf("branch %q exists", name)
	}
	now := time.Now().UTC()
	b := Branch{
		ID:         ids.NewULID(),
		Name:       name,
		BaseBranch: base,
		Snapshot:   snap,
		Lifecycle:  LifecycleDraft,
		CreatedBy:  createdBy,
		CreatedAt:  now,
		UpdatedAt:  now,
	}
	return b, m.store.Put(ctx, b)
}

// Get returns the named branch.
func (m *Manager) Get(ctx context.Context, name string) (Branch, error) {
	return m.store.Get(ctx, name)
}

// List enumerates all known branches.
func (m *Manager) List(ctx context.Context) ([]Branch, error) {
	return m.store.List(ctx)
}

// Update replaces the snapshot of the given branch (idempotent for the
// content, bumps UpdatedAt). Returns ErrFrozen if the branch has been
// promoted to Published.
func (m *Manager) Update(ctx context.Context, name string, snap types.Ontology) (Branch, error) {
	b, err := m.store.Get(ctx, name)
	if err != nil {
		return Branch{}, err
	}
	if b.Lifecycle == LifecyclePublished {
		return Branch{}, ErrFrozen
	}
	b.Snapshot = snap
	b.UpdatedAt = time.Now().UTC()
	return b, m.store.Put(ctx, b)
}

// SubmitForReview transitions a Draft branch to Review.
func (m *Manager) SubmitForReview(ctx context.Context, name string, reviewers []string) (Branch, error) {
	b, err := m.store.Get(ctx, name)
	if err != nil {
		return Branch{}, err
	}
	if b.Lifecycle != LifecycleDraft {
		return Branch{}, fmt.Errorf("branch %q not in draft (was %s)", name, b.Lifecycle)
	}
	b.Lifecycle = LifecycleReview
	b.Reviewers = reviewers
	b.UpdatedAt = time.Now().UTC()
	return b, m.store.Put(ctx, b)
}

// Promote a branch to Published. Requires at least one reviewer to have
// been recorded in SubmitForReview.
func (m *Manager) Promote(ctx context.Context, name string) (Branch, error) {
	b, err := m.store.Get(ctx, name)
	if err != nil {
		return Branch{}, err
	}
	if b.Lifecycle != LifecycleReview {
		return Branch{}, fmt.Errorf("branch %q not in review (was %s)", name, b.Lifecycle)
	}
	if len(b.Reviewers) == 0 {
		return Branch{}, errors.New("branch: at least one reviewer required to promote")
	}
	b.Lifecycle = LifecyclePublished
	b.UpdatedAt = time.Now().UTC()
	return b, m.store.Put(ctx, b)
}

// Delete a branch. Published branches cannot be deleted to preserve
// audit history; promote them out of the way instead.
func (m *Manager) Delete(ctx context.Context, name string) error {
	b, err := m.store.Get(ctx, name)
	if err != nil {
		return err
	}
	if b.Lifecycle == LifecyclePublished {
		return ErrFrozen
	}
	return m.store.Delete(ctx, name)
}

// Diff returns the structural differences between two branches.
func (m *Manager) Diff(ctx context.Context, fromName, toName string) (types.Diff, error) {
	from, err := m.store.Get(ctx, fromName)
	if err != nil {
		return types.Diff{}, err
	}
	to, err := m.store.Get(ctx, toName)
	if err != nil {
		return types.Diff{}, err
	}
	return DiffSnapshots(from.Snapshot, to.Snapshot), nil
}

// Merge applies the changes from src into dst (last-write-wins). The
// resulting snapshot is stored on dst; src is left intact.
func (m *Manager) Merge(ctx context.Context, srcName, dstName string) (Branch, error) {
	src, err := m.store.Get(ctx, srcName)
	if err != nil {
		return Branch{}, err
	}
	dst, err := m.store.Get(ctx, dstName)
	if err != nil {
		return Branch{}, err
	}
	if dst.Lifecycle == LifecyclePublished {
		return Branch{}, ErrFrozen
	}
	dst.Snapshot = MergeSnapshots(dst.Snapshot, src.Snapshot)
	dst.UpdatedAt = time.Now().UTC()
	return dst, m.store.Put(ctx, dst)
}

// ErrFrozen is returned when a mutating operation targets a Published
// branch.
var ErrFrozen = errors.New("branch: published branches are frozen")

// MemoryStore is the default in-process Store. Safe for concurrent use.
type MemoryStore struct {
	mu sync.RWMutex
	m  map[string]Branch
}

// NewMemoryStore constructs an empty in-memory store.
func NewMemoryStore() *MemoryStore { return &MemoryStore{m: make(map[string]Branch)} }

func (s *MemoryStore) Get(_ context.Context, name string) (Branch, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	b, ok := s.m[name]
	if !ok {
		return Branch{}, fmt.Errorf("branch %q not found", name)
	}
	return b, nil
}

func (s *MemoryStore) List(_ context.Context) ([]Branch, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make([]Branch, 0, len(s.m))
	for _, b := range s.m {
		out = append(out, b)
	}
	return out, nil
}

func (s *MemoryStore) Put(_ context.Context, b Branch) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.m[b.Name] = b
	return nil
}

func (s *MemoryStore) Delete(_ context.Context, name string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.m, name)
	return nil
}
