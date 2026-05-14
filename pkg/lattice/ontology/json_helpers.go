// Tiny indirection so test files can override jsonUnmarshal without dragging
// encoding/json into multiple files.

package ontology

import "encoding/json"

func jsonUnmarshal(b []byte, v any) error { return json.Unmarshal(b, v) }
