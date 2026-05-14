// JSON Schema validation for action inputs. Each ActionType.InputSchema is
// compiled once and cached; subsequent validations reuse the compiled schema.

package actions

import (
	"encoding/json"
	"fmt"
	"strings"
	"sync"

	"github.com/santhosh-tekuri/jsonschema/v6"
)

type schemaCache struct {
	mu    sync.RWMutex
	cache map[string]*jsonschema.Schema
}

func newSchemaCache() *schemaCache { return &schemaCache{cache: make(map[string]*jsonschema.Schema)} }

func (c *schemaCache) compile(key string, raw []byte) (*jsonschema.Schema, error) {
	c.mu.RLock()
	if s, ok := c.cache[key]; ok {
		c.mu.RUnlock()
		return s, nil
	}
	c.mu.RUnlock()

	c.mu.Lock()
	defer c.mu.Unlock()
	if s, ok := c.cache[key]; ok {
		return s, nil
	}
	if len(raw) == 0 {
		return nil, fmt.Errorf("empty schema")
	}
	var doc any
	if err := json.Unmarshal(raw, &doc); err != nil {
		return nil, fmt.Errorf("schema json: %w", err)
	}
	compiler := jsonschema.NewCompiler()
	if err := compiler.AddResource(key+"://schema.json", doc); err != nil {
		return nil, fmt.Errorf("add resource: %w", err)
	}
	s, err := compiler.Compile(key + "://schema.json")
	if err != nil {
		return nil, fmt.Errorf("compile: %w", err)
	}
	c.cache[key] = s
	return s, nil
}

// validateInput compiles (or fetches) the schema for at and validates input.
func (c *schemaCache) validate(key string, raw []byte, input map[string]any) error {
	if len(raw) == 0 {
		return nil
	}
	s, err := c.compile(key, raw)
	if err != nil {
		return err
	}
	if err := s.Validate(input); err != nil {
		return fmt.Errorf("schema validation: %s", summarizeValidationError(err))
	}
	return nil
}

func summarizeValidationError(err error) string {
	var verr *jsonschema.ValidationError
	if !asJSONSchemaErr(err, &verr) {
		return err.Error()
	}
	var b strings.Builder
	b.WriteString(verr.Error())
	return b.String()
}

func asJSONSchemaErr(err error, out **jsonschema.ValidationError) bool {
	v, ok := err.(*jsonschema.ValidationError)
	if !ok {
		return false
	}
	*out = v
	return true
}
