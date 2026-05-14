// Datasource declares a connection to an external data store. Credentials are
// always sealed at rest; the in-memory representation here holds the resolved
// configuration that adapters consume.

package types

import "time"

// DatasourceID is the canonical handle for a datasource.
type DatasourceID string

// Datasource is a configured connection to a data store. It identifies the
// adapter type (postgres, bigquery, duckdb, ...) and carries adapter-specific
// configuration plus optional encrypted credentials.
type Datasource struct {
	ID          DatasourceID `json:"id"`
	WorkspaceID WorkspaceID  `json:"workspace_id"`
	APIName     APIName      `json:"api_name"`
	DisplayName string       `json:"display_name"`
	AdapterType string       `json:"adapter_type"`
	Config      ConfigMap    `json:"config"`
	// SealedCredentials is the encrypted blob produced by internal/crypto.Sealer.
	// Adapters never see this field directly; the registry calls Sealer.Open
	// and merges the result into a working Config copy before invoking Connect.
	SealedCredentials []byte    `json:"-"`
	CreatedAt         time.Time `json:"created_at"`
	UpdatedAt         time.Time `json:"updated_at"`
}

// ConfigMap is the adapter-specific configuration map. Values are typically
// strings or numbers; nested structures are represented as map[string]any.
type ConfigMap map[string]any

// Get returns the typed string value at key, with ok=false when absent or
// not a string. Adapters use this for the common case of fetching scalar
// configuration entries.
func (c ConfigMap) Get(key string) (string, bool) {
	v, ok := c[key]
	if !ok {
		return "", false
	}
	s, ok := v.(string)
	return s, ok
}

// SourceConfig describes where instances of an object type are stored within
// a datasource (typically a table or view name plus optional schema).
type SourceConfig struct {
	DatasourceAPIName APIName `json:"datasource"`
	Schema            string  `json:"schema,omitempty"`
	Table             string  `json:"table"`
}
