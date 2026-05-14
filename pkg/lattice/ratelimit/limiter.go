// Token-bucket limiter keyed by (workspace_id, key). Safe for concurrent use.

package ratelimit

import (
	"sync"
	"time"
)

// Limiter is the contract every backend implements.
type Limiter interface {
	Allow(workspace, key string) bool
}

// MemoryLimiter is the in-memory token bucket implementation.
type MemoryLimiter struct {
	rate  float64
	burst float64
	now   func() time.Time
	mu    sync.Mutex
	state map[bucketKey]*bucket
}

type bucketKey struct{ workspace, key string }

type bucket struct {
	tokens float64
	last   time.Time
}

// NewMemory builds a MemoryLimiter at rate tokens/second with the given burst.
func NewMemory(rate, burst float64) *MemoryLimiter {
	return &MemoryLimiter{
		rate: rate, burst: burst, now: time.Now,
		state: make(map[bucketKey]*bucket),
	}
}

// Allow consumes one token if available; returns false otherwise.
func (l *MemoryLimiter) Allow(workspace, key string) bool {
	if l == nil {
		return true
	}
	l.mu.Lock()
	defer l.mu.Unlock()
	k := bucketKey{workspace: workspace, key: key}
	b, ok := l.state[k]
	now := l.now()
	if !ok {
		b = &bucket{tokens: l.burst, last: now}
		l.state[k] = b
	}
	elapsed := now.Sub(b.last).Seconds()
	b.tokens = min(l.burst, b.tokens+elapsed*l.rate)
	b.last = now
	if b.tokens >= 1 {
		b.tokens--
		return true
	}
	return false
}
