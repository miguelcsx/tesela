// Package vector adds RAG-friendly semantic-search capability to the
// Lattice ontology. Backends are pluggable (Qdrant, Pinecone, pgvector,
// in-memory test impl) via the Backend interface. The Index helper
// pairs an object type with an embedder + backend and exposes a typed
// SemanticSearch API.

package vector

import (
	"context"
	"errors"
	"sync"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Vector is a dense embedding.
type Vector []float32

// Hit is a single semantic-search result. Score is implementation-defined
// (cosine similarity, dot product, distance — backends should document).
type Hit struct {
	PrimaryKey string         `json:"primary_key"`
	Score      float32        `json:"score"`
	Payload    map[string]any `json:"payload,omitempty"`
}

// Embedder converts text or arbitrary content into a Vector. Implementations
// typically wrap a remote model API (OpenAI text-embedding-3, Cohere embed,
// local ggml model, etc.).
type Embedder interface {
	Embed(ctx context.Context, text string) (Vector, error)
}

// Backend is the storage capability — upserts vectors and queries them by
// nearest-neighbor.
type Backend interface {
	Upsert(ctx context.Context, namespace string, items []Item) error
	Query(ctx context.Context, namespace string, q Vector, topK int) ([]Hit, error)
	Delete(ctx context.Context, namespace string, primaryKey string) error
}

// Item is one record going into a vector backend.
type Item struct {
	PrimaryKey string
	Vector     Vector
	Payload    map[string]any
}

// Index ties an ObjectType to an Embedder and a Backend. The namespace
// convention is "<workspace>:<object_type>".
type Index struct {
	ObjectType types.APIName
	Workspace  types.WorkspaceID
	Embedder   Embedder
	Backend    Backend
	// Property selects the source text for embeddings. Empty means the
	// caller will pass an explicit string to Add().
	Property types.APIName
}

// Namespace returns the canonical backend namespace string.
func (i *Index) Namespace() string {
	return string(i.Workspace) + ":" + string(i.ObjectType)
}

// Add embeds the provided text and stores it under primaryKey.
func (i *Index) Add(ctx context.Context, primaryKey, text string, payload map[string]any) error {
	if i.Embedder == nil || i.Backend == nil {
		return errors.New("vector: embedder or backend not configured")
	}
	v, err := i.Embedder.Embed(ctx, text)
	if err != nil {
		return err
	}
	return i.Backend.Upsert(ctx, i.Namespace(), []Item{{
		PrimaryKey: primaryKey, Vector: v, Payload: payload,
	}})
}

// Search runs a semantic search over the index.
func (i *Index) Search(ctx context.Context, query string, topK int) ([]Hit, error) {
	if i.Embedder == nil || i.Backend == nil {
		return nil, errors.New("vector: embedder or backend not configured")
	}
	if topK <= 0 {
		topK = 10
	}
	v, err := i.Embedder.Embed(ctx, query)
	if err != nil {
		return nil, err
	}
	return i.Backend.Query(ctx, i.Namespace(), v, topK)
}

// MemoryBackend is an in-process Backend that stores vectors in a map and
// scores by cosine similarity. Suitable for tests and small datasets.
type MemoryBackend struct {
	mu    sync.RWMutex
	store map[string]map[string]Item // namespace → pk → item
}

// NewMemoryBackend constructs an in-process vector backend.
func NewMemoryBackend() *MemoryBackend {
	return &MemoryBackend{store: make(map[string]map[string]Item)}
}

func (b *MemoryBackend) Upsert(_ context.Context, ns string, items []Item) error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if _, ok := b.store[ns]; !ok {
		b.store[ns] = make(map[string]Item)
	}
	for _, it := range items {
		b.store[ns][it.PrimaryKey] = it
	}
	return nil
}

func (b *MemoryBackend) Query(_ context.Context, ns string, q Vector, topK int) ([]Hit, error) {
	b.mu.RLock()
	defer b.mu.RUnlock()
	pool, ok := b.store[ns]
	if !ok {
		return nil, nil
	}
	hits := make([]Hit, 0, len(pool))
	for pk, it := range pool {
		hits = append(hits, Hit{
			PrimaryKey: pk,
			Score:      cosine(q, it.Vector),
			Payload:    it.Payload,
		})
	}
	// Selection sort top-k descending; for small k this is faster than
	// a full sort + slice on large pools.
	if topK > len(hits) {
		topK = len(hits)
	}
	for i := 0; i < topK; i++ {
		best := i
		for j := i + 1; j < len(hits); j++ {
			if hits[j].Score > hits[best].Score {
				best = j
			}
		}
		hits[i], hits[best] = hits[best], hits[i]
	}
	return hits[:topK], nil
}

func (b *MemoryBackend) Delete(_ context.Context, ns, pk string) error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if pool, ok := b.store[ns]; ok {
		delete(pool, pk)
	}
	return nil
}

// cosine computes cosine similarity, returning 0 when either vector is
// zero or dimensions disagree (defensive — bad inputs shouldn't NaN).
func cosine(a, b Vector) float32 {
	if len(a) != len(b) || len(a) == 0 {
		return 0
	}
	var dot, na, nb float32
	for i := range a {
		dot += a[i] * b[i]
		na += a[i] * a[i]
		nb += b[i] * b[i]
	}
	if na == 0 || nb == 0 {
		return 0
	}
	return dot / (sqrt32(na) * sqrt32(nb))
}

// sqrt32 — tiny Newton-Raphson; avoids math.Sqrt's float64 cast.
func sqrt32(x float32) float32 {
	if x <= 0 {
		return 0
	}
	z := x
	for i := 0; i < 8; i++ {
		z = (z + x/z) / 2
	}
	return z
}
