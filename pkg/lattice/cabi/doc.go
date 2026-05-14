// Package cabi exports the public Lattice API as `extern "C"` functions
// suitable for consumption by FFI bindings (Python via cffi, Node via
// napi-rs, Rust via direct FFI).
//
// Build with:
//
//	go build -tags cabi -buildmode=c-shared -o dist/liblattice.so ./pkg/lattice/cabi
//
// The build tag `cabi` keeps this package out of normal `go build ./...`
// runs because it must be built as a c-shared library, not a regular Go
// program.
//
// # ABI conventions
//
//   - Handles are opaque uintptr_t values returned by constructors. The Go
//     side maintains a registry; bindings never dereference them.
//   - JSON is the wire format between binding and core. Records, queries,
//     filters, mutations, errors all cross as JSON strings.
//   - Strings returned by Lattice live in a Go-owned arena. Bindings must
//     call lattice_free(ptr) when done, NOT C's free().
//   - Errors: every export returns either a result struct or a status code
//     plus an out-parameter for the error string. See errors.go.
//   - Callbacks: bindings register C function pointers against object
//     types. Lattice invokes them on its own goroutines; bindings are
//     responsible for thread safety (Python's cffi handles the GIL).
//
//go:build cabi

package main
