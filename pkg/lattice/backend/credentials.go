// Sealed credentials are a JSON-encoded map[string]any blob produced at
// Datasource creation time. decodeCredentials is the inverse — used by the
// registry when opening a sealed payload before it merges into ConfigMap.

package backend

import (
	"encoding/json"
	"fmt"
)

func decodeCredentials(raw []byte) (map[string]any, error) {
	var m map[string]any
	if err := json.Unmarshal(raw, &m); err != nil {
		return nil, fmt.Errorf("unmarshal credentials json: %w", err)
	}
	return m, nil
}

// EncodeCredentials is the inverse of decodeCredentials and is exported for
// the API layer's CreateDatasource handler, which seals the user-supplied
// credential map before persisting.
func EncodeCredentials(creds map[string]any) ([]byte, error) {
	return json.Marshal(creds)
}
