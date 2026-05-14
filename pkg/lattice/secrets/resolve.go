// Reference-resolution helpers. A reference of the form "${NAME}" is looked
// up against the configured provider; any other value is returned literally.

package secrets

import (
	"context"
	"strings"
)

const (
	refPrefix = "${"
	refSuffix = "}"
)

// IsReference reports whether s is a "${...}" secret reference.
func IsReference(s string) bool {
	return strings.HasPrefix(s, refPrefix) && strings.HasSuffix(s, refSuffix) && len(s) > len(refPrefix)+len(refSuffix)
}

// ResolveReference looks up a "${NAME}" reference via p, or returns s
// unchanged when it is not a reference.
func ResolveReference(ctx context.Context, p SecretProvider, s string) (string, error) {
	if !IsReference(s) {
		return s, nil
	}
	name := s[len(refPrefix) : len(s)-len(refSuffix)]
	return p.Lookup(ctx, name)
}

// ResolveReferences applies ResolveReference to every value in the input map,
// returning a new map. Keys with empty values are passed through unchanged.
func ResolveReferences(ctx context.Context, p SecretProvider, in map[string]string) (map[string]string, error) {
	out := make(map[string]string, len(in))
	for k, v := range in {
		resolved, err := ResolveReference(ctx, p, v)
		if err != nil {
			return nil, err
		}
		out[k] = resolved
	}
	return out, nil
}
