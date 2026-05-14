package ids_test

import (
	"regexp"
	"strings"
	"sync"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/ids"
)

var (
	ulidRE = regexp.MustCompile(`^[0-9A-HJKMNP-TV-Z]{26}$`)
	uuidRE = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)
)

func TestNewULID_FormatAndUniqueness(t *testing.T) {
	t.Parallel()

	const n = 5000
	seen := make(map[string]struct{}, n)
	for i := 0; i < n; i++ {
		id := ids.NewULID()
		if !ulidRE.MatchString(id) {
			t.Fatalf("invalid ULID format: %q", id)
		}
		if _, dup := seen[id]; dup {
			t.Fatalf("duplicate ULID at i=%d: %q", i, id)
		}
		seen[id] = struct{}{}
	}
}

func TestNewULID_MonotonicWithinSameMillisecond(t *testing.T) {
	// Lexicographic order must follow generation order.
	const n = 1000
	prev := ids.NewULID()
	for i := 1; i < n; i++ {
		curr := ids.NewULID()
		if curr <= prev {
			t.Fatalf("ULIDs out of order at i=%d: prev=%s curr=%s", i, prev, curr)
		}
		prev = curr
	}
}

func TestNewULID_ConcurrentSafe(t *testing.T) {
	const goroutines = 32
	const perG = 200

	var (
		wg sync.WaitGroup
		mu sync.Mutex
	)
	results := make(map[string]struct{}, goroutines*perG)

	wg.Add(goroutines)
	for g := 0; g < goroutines; g++ {
		go func() {
			defer wg.Done()
			local := make([]string, 0, perG)
			for i := 0; i < perG; i++ {
				local = append(local, ids.NewULID())
			}
			mu.Lock()
			for _, id := range local {
				if _, dup := results[id]; dup {
					t.Errorf("concurrent duplicate: %q", id)
				}
				results[id] = struct{}{}
			}
			mu.Unlock()
		}()
	}
	wg.Wait()
}

func TestNewUUID_FormatAndUniqueness(t *testing.T) {
	t.Parallel()

	const n = 1000
	seen := make(map[string]struct{}, n)
	for i := 0; i < n; i++ {
		id := ids.NewUUID()
		if !uuidRE.MatchString(id) {
			t.Fatalf("invalid UUID format: %q", id)
		}
		if _, dup := seen[id]; dup {
			t.Fatalf("duplicate UUID: %q", id)
		}
		seen[id] = struct{}{}
	}
}

func TestNewUUID_VersionAndVariantBits(t *testing.T) {
	t.Parallel()

	id := ids.NewUUID()
	// Version nibble must be 4 (UUIDv4).
	if id[14] != '4' {
		t.Fatalf("UUID version nibble: want '4', got %q in %q", id[14], id)
	}
	// Variant nibble (RFC 4122) must be one of 8, 9, a, b.
	v := strings.ToLower(string(id[19]))
	if !strings.ContainsAny(v, "89ab") {
		t.Fatalf("UUID variant nibble: want one of 89ab, got %q in %q", v, id)
	}
}
