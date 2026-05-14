// Record is the adapter-level row representation: keyed by ObjectType
// property api_names, with values typed via DataType conventions.

package types

// Record is a single row in property-keyed form. Values follow the encoding
// conventions documented per DataType (e.g., temporal fields are time.Time,
// JSON fields are decoded any).
type Record struct {
	Values map[APIName]any `json:"values"`
}

// NewRecord returns an empty record with a pre-allocated map.
func NewRecord(capacity int) Record {
	return Record{Values: make(map[APIName]any, capacity)}
}

// Get returns the value for the property along with whether it was present.
func (r Record) Get(name APIName) (any, bool) {
	if r.Values == nil {
		return nil, false
	}
	v, ok := r.Values[name]
	return v, ok
}

// Set assigns a value for the property.
func (r Record) Set(name APIName, value any) {
	r.Values[name] = value
}

// Without returns a copy of the record with the named properties removed.
// Used by the policy engine to apply per-actor redactions.
func (r Record) Without(names ...APIName) Record {
	if len(names) == 0 || len(r.Values) == 0 {
		return r
	}
	skip := make(map[APIName]struct{}, len(names))
	for _, n := range names {
		skip[n] = struct{}{}
	}
	out := NewRecord(len(r.Values))
	for k, v := range r.Values {
		if _, drop := skip[k]; drop {
			continue
		}
		out.Set(k, v)
	}
	return out
}
