// Cache is the per-workspace atomic snapshot store. Readers Load with no
// lock; writers Set under a mutex so versions/notifications stay coherent.

package ontology

import (
	"sync"
	"sync/atomic"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Cache holds the latest *types.Ontology snapshot per workspace.
type Cache struct {
	mu        sync.Mutex
	snapshots sync.Map // map[types.WorkspaceID]*atomic.Pointer[types.Ontology]

	subsMu sync.RWMutex
	subs   map[types.WorkspaceID][]chan types.Change
}

// NewCache constructs an empty cache.
func NewCache() *Cache {
	return &Cache{subs: make(map[types.WorkspaceID][]chan types.Change)}
}

// Load returns the current snapshot for ws or nil if none exists.
func (c *Cache) Load(ws types.WorkspaceID) *types.Ontology {
	v, ok := c.snapshots.Load(ws)
	if !ok {
		return nil
	}
	return v.(*atomic.Pointer[types.Ontology]).Load()
}

// Store atomically swaps in a new snapshot for ws and notifies subscribers.
func (c *Cache) Store(ws types.WorkspaceID, snap *types.Ontology, change types.Change) {
	c.mu.Lock()
	v, ok := c.snapshots.Load(ws)
	if !ok {
		ptr := &atomic.Pointer[types.Ontology]{}
		ptr.Store(snap)
		c.snapshots.Store(ws, ptr)
	} else {
		v.(*atomic.Pointer[types.Ontology]).Store(snap)
	}
	c.mu.Unlock()
	c.notify(ws, change)
}

// Subscribe returns a buffered channel of Change events. The caller is
// responsible for draining it; if the buffer fills, future changes for that
// channel are dropped (the producer never blocks).
func (c *Cache) Subscribe(ws types.WorkspaceID) <-chan types.Change {
	ch := make(chan types.Change, 16)
	c.subsMu.Lock()
	c.subs[ws] = append(c.subs[ws], ch)
	c.subsMu.Unlock()
	return ch
}

// Unsubscribe removes ch from the subscriber set for ws and closes it.
func (c *Cache) Unsubscribe(ws types.WorkspaceID, ch <-chan types.Change) {
	c.subsMu.Lock()
	defer c.subsMu.Unlock()
	subs := c.subs[ws]
	for i, s := range subs {
		if s == ch {
			c.subs[ws] = append(subs[:i], subs[i+1:]...)
			close(s)
			return
		}
	}
}

func (c *Cache) notify(ws types.WorkspaceID, change types.Change) {
	c.subsMu.RLock()
	subs := append([]chan types.Change(nil), c.subs[ws]...)
	c.subsMu.RUnlock()
	for _, ch := range subs {
		select {
		case ch <- change:
		default:
			// Drop event for slow consumers; documented in package comment.
		}
	}
}
