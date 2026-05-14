// Package buildinfo exposes build-time metadata injected via -ldflags.
//
// Values are set at link time from the Makefile (see VERSION/COMMIT/BUILD_DATE).
// Reading these is safe even from concurrent goroutines because they are immutable
// once the binary is loaded.
package buildinfo

// Version is the human-readable release identifier (typically a git tag or "dev").
var Version = "dev"

// Commit is the short git SHA the binary was built from.
var Commit = "unknown"

// Date is the UTC timestamp of the build, RFC3339-formatted.
var Date = "unknown"

// Info bundles build metadata for serialization (e.g., the /v1/version endpoint).
type Info struct {
	Version string `json:"version"`
	Commit  string `json:"commit"`
	Date    string `json:"date"`
}

// Current returns a snapshot of the build metadata.
func Current() Info {
	return Info{Version: Version, Commit: Commit, Date: Date}
}
