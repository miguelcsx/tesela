//go:build cabi

// Handle registry. The new spec-based ABI only needs to track *lattice.App;
// object types and policies are addressed by api_name through the App.

package main

import (
	"sync"
	"sync/atomic"

	"github.com/miguelcsx/lattice/pkg/lattice"
)

var (
	handleMu   sync.RWMutex
	apps       = map[uint64]*lattice.App{}
	nextHandle atomic.Uint64
)

func registerApp(a *lattice.App) uint64 {
	id := nextHandle.Add(1)
	handleMu.Lock()
	apps[id] = a
	handleMu.Unlock()
	return id
}

func lookupApp(id uint64) *lattice.App {
	handleMu.RLock()
	defer handleMu.RUnlock()
	return apps[id]
}

func releaseApp(id uint64) {
	handleMu.Lock()
	delete(apps, id)
	handleMu.Unlock()
}
