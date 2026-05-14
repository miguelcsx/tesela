package branch_test

import (
	"context"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/branch"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func snap(names ...types.APIName) types.Ontology {
	o := types.Ontology{}
	for _, n := range names {
		o.ObjectTypes = append(o.ObjectTypes, types.ObjectType{APIName: n, PrimaryKey: "id"})
	}
	return o
}

func TestBranch_CreateAndPromoteLifecycle(t *testing.T) {
	m := branch.NewManager(branch.NewMemoryStore())
	ctx := context.Background()

	b, err := m.Create(ctx, "feature-1", "main", "alice", snap("Customer"))
	if err != nil {
		t.Fatal(err)
	}
	if b.Lifecycle != branch.LifecycleDraft {
		t.Fatalf("expected draft, got %s", b.Lifecycle)
	}

	if _, err := m.Promote(ctx, "feature-1"); err == nil {
		t.Fatal("expected promote-without-review to fail")
	}

	b, err = m.SubmitForReview(ctx, "feature-1", []string{"bob"})
	if err != nil {
		t.Fatal(err)
	}
	if b.Lifecycle != branch.LifecycleReview {
		t.Fatalf("expected review, got %s", b.Lifecycle)
	}

	b, err = m.Promote(ctx, "feature-1")
	if err != nil {
		t.Fatal(err)
	}
	if b.Lifecycle != branch.LifecyclePublished {
		t.Fatalf("expected published, got %s", b.Lifecycle)
	}

	if _, err := m.Update(ctx, "feature-1", snap("Customer", "Order")); err == nil {
		t.Fatal("expected update on published to fail (frozen)")
	}
}

func TestBranch_DiffAndMerge(t *testing.T) {
	m := branch.NewManager(branch.NewMemoryStore())
	ctx := context.Background()

	_, _ = m.Create(ctx, "main", "", "a", snap("Customer"))
	_, _ = m.Create(ctx, "feat", "main", "a", snap("Customer", "Order"))

	d, err := m.Diff(ctx, "main", "feat")
	if err != nil {
		t.Fatal(err)
	}
	if len(d.Created) != 1 || d.Created[0].APIName != "Order" {
		t.Fatalf("expected Order created, got %+v", d.Created)
	}

	if _, err := m.Merge(ctx, "feat", "main"); err != nil {
		t.Fatal(err)
	}
	main, _ := m.Get(ctx, "main")
	if len(main.Snapshot.ObjectTypes) != 2 {
		t.Fatalf("after merge, main should have 2 types, got %d", len(main.Snapshot.ObjectTypes))
	}
}
