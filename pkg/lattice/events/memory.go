package events

import (
	"context"
	"errors"
	"sync"
	"sync/atomic"

	"github.com/miguelcsx/lattice/pkg/lattice/ids"
)

// MemoryBus is an in-process Bus. Each subscriber gets its own goroutine
// pumping a buffered channel; if the channel fills the publish path drops
// the event for that subscriber and increments a drop counter, but never
// blocks the publisher.
type MemoryBus struct {
	mu     sync.RWMutex
	subs   map[uint64]*memSub
	closed atomic.Bool
	bufLen int
}

// MemoryBusOption configures the MemoryBus.
type MemoryBusOption func(*MemoryBus)

// WithBuffer sets the per-subscriber buffer size. Default 256.
func WithBuffer(n int) MemoryBusOption {
	return func(b *MemoryBus) {
		if n > 0 {
			b.bufLen = n
		}
	}
}

// NewMemoryBus constructs an in-process Bus.
func NewMemoryBus(opts ...MemoryBusOption) *MemoryBus {
	b := &MemoryBus{
		subs:   make(map[uint64]*memSub),
		bufLen: 256,
	}
	for _, o := range opts {
		o(b)
	}
	return b
}

type memSub struct {
	id      uint64
	bus     *MemoryBus
	filter  Filter
	handler Handler
	ch      chan Event
	done    chan struct{}
	drops   atomic.Uint64
}

func (s *memSub) Close() error {
	s.bus.mu.Lock()
	if _, ok := s.bus.subs[s.id]; !ok {
		s.bus.mu.Unlock()
		return nil
	}
	delete(s.bus.subs, s.id)
	s.bus.mu.Unlock()
	close(s.ch)
	<-s.done
	return nil
}

// Drops returns the number of events dropped because the subscriber's
// buffer was full. Useful for observability.
func (s *memSub) Drops() uint64 { return s.drops.Load() }

func (b *MemoryBus) Publish(ctx context.Context, e Event) error {
	if b.closed.Load() {
		return errors.New("events: bus closed")
	}
	if e.ID == "" {
		e.ID = ids.NewULID()
	}
	if e.OccurredAt.IsZero() {
		e.OccurredAt = nowFn()
	}
	b.mu.RLock()
	for _, s := range b.subs {
		if !s.filter.Match(e) {
			continue
		}
		select {
		case s.ch <- e:
		default:
			s.drops.Add(1)
		}
	}
	b.mu.RUnlock()
	return nil
}

func (b *MemoryBus) Subscribe(filter Filter, h Handler) (Subscription, error) {
	if b.closed.Load() {
		return nil, errors.New("events: bus closed")
	}
	if h == nil {
		return nil, errors.New("events: nil handler")
	}
	id := nextSubID.Add(1)
	s := &memSub{
		id:      id,
		bus:     b,
		filter:  filter,
		handler: h,
		ch:      make(chan Event, b.bufLen),
		done:    make(chan struct{}),
	}
	b.mu.Lock()
	b.subs[id] = s
	b.mu.Unlock()
	go s.run()
	return s, nil
}

func (s *memSub) run() {
	defer close(s.done)
	for e := range s.ch {
		// Handler errors are intentionally swallowed — subscribers that
		// need durable retry semantics should layer their own.
		_ = s.handler(context.Background(), e)
	}
}

func (b *MemoryBus) Close() error {
	if !b.closed.CompareAndSwap(false, true) {
		return nil
	}
	b.mu.Lock()
	subs := b.subs
	b.subs = make(map[uint64]*memSub)
	b.mu.Unlock()
	for _, s := range subs {
		close(s.ch)
		<-s.done
	}
	return nil
}

var nextSubID atomic.Uint64
