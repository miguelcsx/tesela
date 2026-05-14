// Time-ordered Crockford-base32 identifier (26 chars). Backed by a monotonic
// entropy source so identifiers minted from the same millisecond are still
// strictly increasing.

package ids

import (
	"crypto/rand"
	"sync"
	"time"

	"github.com/oklog/ulid/v2"
)

// monotonicEntropy serializes access to the monotonic entropy source, which
// is not safe for concurrent use on its own.
var (
	monotonicMu      sync.Mutex
	monotonicEntropy = ulid.Monotonic(rand.Reader, 0)
)

// NewULID returns a freshly minted ULID.
//
// The 48-bit timestamp is the current Unix milliseconds; the 80 bits of
// entropy are drawn from a crypto/rand-backed monotonic source, ensuring
// strict ordering within a single millisecond and uniqueness across
// goroutines.
func NewULID() string {
	monotonicMu.Lock()
	defer monotonicMu.Unlock()
	return ulid.MustNew(ulid.Timestamp(time.Now()), monotonicEntropy).String()
}
