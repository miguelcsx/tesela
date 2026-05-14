// JSON helpers used by the registry to encode credential maps before sealing.

package ontology

import "encoding/json"

func encodeJSON(v any) ([]byte, error) { return json.Marshal(v) }
