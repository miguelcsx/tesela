// Package telemetry wires OpenTelemetry tracing/metrics and the structured
// logger that every Lattice binary uses.
//
// Bootstrap returns a Runtime whose Shutdown method drains all exporters
// and is safe to call multiple times. When Config.Enabled is false the
// returned runtime is a no-op — useful for tests and local development.
//
// The logger produced by NewLogger is a *slog.Logger configured with either
// JSON or text handlers and the requested minimum level. It is the
// canonical logger for the codebase: do not call slog.Default; call
// telemetry.NewLogger and pass the result down explicitly.
package telemetry
