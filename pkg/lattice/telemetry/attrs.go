// Small adapters around go.opentelemetry.io/otel/attribute. Kept private so
// the rest of the codebase imports telemetry instead of direct OTEL packages
// for the common cases (telemetry.Span, telemetry.NewLogger).

package telemetry

import "go.opentelemetry.io/otel/attribute"

// toAttrs accepts a heterogeneous slice produced by buildResource and converts
// it into the strongly typed attribute.KeyValue slice OTEL expects.
func toAttrs(in []any) []attribute.KeyValue {
	out := make([]attribute.KeyValue, 0, len(in))
	for _, v := range in {
		if kv, ok := v.(attribute.KeyValue); ok {
			out = append(out, kv)
		}
	}
	return out
}
