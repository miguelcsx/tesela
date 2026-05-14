// Span is a thin facade around the OTEL tracer. It exists so the rest of the
// codebase doesn't import otel directly for the common case of starting a
// child span; this also keeps the tracer name consistent.

package telemetry

import (
	"context"

	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/trace"
)

// tracerName is the name passed to the global tracer provider for every
// Lattice-internal span.
const tracerName = "github.com/miguelcsx/lattice/pkg/lattice"

// Span starts a child span on the global tracer and returns the new context
// alongside the span. Callers must End() the span; defer is recommended.
func Span(ctx context.Context, name string, opts ...trace.SpanStartOption) (context.Context, trace.Span) {
	return otel.Tracer(tracerName).Start(ctx, name, opts...)
}
