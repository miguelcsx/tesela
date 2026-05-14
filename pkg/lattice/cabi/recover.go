//go:build cabi

// Recovery helper. Each //export function defers recoverToStderr to ensure
// a panic in Lattice doesn't kill the host process (Python/Node/Rust).
// Instead, the panic is logged to stderr and the function returns its
// zero-value.

package main

import (
	"fmt"
	"os"
	"runtime/debug"
)

func recoverToStderr(fn string) {
	if r := recover(); r != nil {
		fmt.Fprintf(os.Stderr, "lattice cabi: panic in %s: %v\n%s\n", fn, r, debug.Stack())
	}
}
