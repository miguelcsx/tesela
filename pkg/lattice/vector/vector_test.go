package vector_test

import (
	"context"
	"strings"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/vector"
)

type stubEmbedder struct{}

// Trivial bag-of-chars embedder for deterministic tests: 26 dims of letter
// counts. Same string ⇒ same vector ⇒ similarity 1.
func (stubEmbedder) Embed(_ context.Context, s string) (vector.Vector, error) {
	v := make(vector.Vector, 26)
	for _, r := range strings.ToLower(s) {
		if r >= 'a' && r <= 'z' {
			v[r-'a']++
		}
	}
	return v, nil
}

func TestVectorIndex_AddAndSearch(t *testing.T) {
	idx := &vector.Index{
		ObjectType: "Doc",
		Workspace:  "ws",
		Embedder:   stubEmbedder{},
		Backend:    vector.NewMemoryBackend(),
	}
	ctx := context.Background()
	if err := idx.Add(ctx, "1", "machine learning systems", nil); err != nil {
		t.Fatal(err)
	}
	if err := idx.Add(ctx, "2", "cookie recipes for the holidays", nil); err != nil {
		t.Fatal(err)
	}

	hits, err := idx.Search(ctx, "machine learning", 1)
	if err != nil {
		t.Fatal(err)
	}
	if len(hits) != 1 || hits[0].PrimaryKey != "1" {
		t.Fatalf("expected doc 1 to win, got %+v", hits)
	}
}
