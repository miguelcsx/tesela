package actions

import (
	"testing"
)

func TestSchemaCache_ValidateRejectsMissingRequired(t *testing.T) {
	c := newSchemaCache()
	schema := []byte(`{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}`)
	if err := c.validate("test", schema, map[string]any{}); err == nil {
		t.Fatal("expected validation error for missing required field")
	}
}

func TestSchemaCache_ValidateAcceptsValid(t *testing.T) {
	c := newSchemaCache()
	schema := []byte(`{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}`)
	if err := c.validate("test", schema, map[string]any{"name": "x"}); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestSchemaCache_NoSchemaIsNoOp(t *testing.T) {
	c := newSchemaCache()
	if err := c.validate("test", nil, map[string]any{}); err != nil {
		t.Fatal(err)
	}
}

func TestSchemaCache_CompileCachesByKey(t *testing.T) {
	c := newSchemaCache()
	schema := []byte(`{"type":"object"}`)
	a, err := c.compile("k", schema)
	if err != nil {
		t.Fatal(err)
	}
	b, err := c.compile("k", schema)
	if err != nil {
		t.Fatal(err)
	}
	if a != b {
		t.Fatal("expected cache hit to return same schema instance")
	}
}
