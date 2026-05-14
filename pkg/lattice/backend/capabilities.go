// Capability assertion helpers. The runtime uses these to convert a generic
// Connection to a specific capability interface, returning a typed error
// when the backend doesn't implement the requested capability.

package backend

import "fmt"

// CapabilityError is returned by AsX helpers when a backend does not
// implement the requested capability.
type CapabilityError struct {
	Backend    string
	Capability string
}

// Error implements the error interface.
func (e *CapabilityError) Error() string {
	return fmt.Sprintf("backend %q does not implement %s", e.Backend, e.Capability)
}

// AsSearcher returns the Searcher view of c, or a CapabilityError.
func AsSearcher(c Connection) (Searcher, error) {
	s, ok := c.(Searcher)
	if !ok {
		return nil, &CapabilityError{Backend: connName(c), Capability: "Searcher"}
	}
	return s, nil
}

// AsGetter returns the Getter view of c, or a CapabilityError.
func AsGetter(c Connection) (Getter, error) {
	g, ok := c.(Getter)
	if !ok {
		return nil, &CapabilityError{Backend: connName(c), Capability: "Getter"}
	}
	return g, nil
}

// AsAggregator returns the Aggregator view of c, or a CapabilityError.
func AsAggregator(c Connection) (Aggregator, error) {
	a, ok := c.(Aggregator)
	if !ok {
		return nil, &CapabilityError{Backend: connName(c), Capability: "Aggregator"}
	}
	return a, nil
}

// AsTraverser returns the Traverser view of c, or a CapabilityError.
func AsTraverser(c Connection) (Traverser, error) {
	t, ok := c.(Traverser)
	if !ok {
		return nil, &CapabilityError{Backend: connName(c), Capability: "Traverser"}
	}
	return t, nil
}

// AsMutator returns the Mutator view of c, or a CapabilityError.
func AsMutator(c Connection) (Mutator, error) {
	m, ok := c.(Mutator)
	if !ok {
		return nil, &CapabilityError{Backend: connName(c), Capability: "Mutator"}
	}
	return m, nil
}

// AsBulkLoader returns the BulkLoader view of c, or a CapabilityError.
func AsBulkLoader(c Connection) (BulkLoader, error) {
	b, ok := c.(BulkLoader)
	if !ok {
		return nil, &CapabilityError{Backend: connName(c), Capability: "BulkLoader"}
	}
	return b, nil
}

// connName returns a friendly identifier for c, used in error messages.
func connName(c Connection) string {
	if n, ok := c.(interface{ Name() string }); ok {
		return n.Name()
	}
	return fmt.Sprintf("%T", c)
}
