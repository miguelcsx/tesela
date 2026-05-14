// Auto-derivation of MCP tools, resources, and prompts from a Lattice
// ontology snapshot. Each ObjectType produces `<type>.search` and
// `<type>.get` tools; ActionTypes produce one tool per action.

package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// tool is the wire shape MCP clients expect.
type tool struct {
	Name        string         `json:"name"`
	Description string         `json:"description,omitempty"`
	InputSchema map[string]any `json:"inputSchema"`
}

// toolsList enumerates tools derived from the ontology.
func (s *Server) toolsList() any {
	if s.cfg.Snapshot == nil {
		return map[string]any{"tools": []tool{}}
	}
	out := []tool{}
	for _, ot := range s.cfg.Snapshot.ObjectTypes {
		out = append(out, tool{
			Name:        toolNameSearch(ot),
			Description: fmt.Sprintf("Search %s objects with optional filters and pagination", ot.APIName),
			InputSchema: searchInputSchema(ot),
		})
		out = append(out, tool{
			Name:        toolNameGet(ot),
			Description: fmt.Sprintf("Fetch a single %s by primary key", ot.APIName),
			InputSchema: getInputSchema(ot),
		})
	}
	for _, at := range s.cfg.Snapshot.ActionTypes {
		out = append(out, tool{
			Name:        string(at.APIName),
			Description: at.Description,
			InputSchema: actionInputSchema(at),
		})
	}
	return map[string]any{"tools": out}
}

// toolsCall dispatches a single tool invocation. The returned value is
// wrapped in {content: [...]} per MCP spec.
func (s *Server) toolsCall(ctx context.Context, params json.RawMessage) (any, error) {
	var p struct {
		Name      string         `json:"name"`
		Arguments map[string]any `json:"arguments"`
	}
	if err := json.Unmarshal(params, &p); err != nil {
		return nil, err
	}
	if s.cfg.Snapshot == nil {
		return nil, fmt.Errorf("no ontology")
	}

	// <type>.search
	if ot, ok := matchSuffix(s.cfg.Snapshot, p.Name, ".search"); ok {
		spec := buildQuerySpec(p.Arguments)
		page, err := s.cfg.Search(ctx, ot, spec)
		if err != nil {
			return nil, err
		}
		return wrapJSONContent(page), nil
	}
	// <type>.get
	if ot, ok := matchSuffix(s.cfg.Snapshot, p.Name, ".get"); ok {
		pk := p.Arguments[string(ot.PrimaryKey)]
		if pk == nil {
			pk = p.Arguments["primary_key"]
		}
		rec, err := s.cfg.Get(ctx, ot, pk)
		if err != nil {
			return nil, err
		}
		return wrapJSONContent(rec), nil
	}
	// action — actions are namespaced by api_name; we resolve by exact match.
	for _, at := range s.cfg.Snapshot.ActionTypes {
		if string(at.APIName) == p.Name {
			// Actions are not yet executed via MCP; return descriptor so the
			// caller can decide. Implementations layer their own runner.
			return wrapJSONContent(map[string]any{
				"action":  string(at.APIName),
				"args":    p.Arguments,
				"status":  "pending",
				"message": "action descriptors are exposed by MCP; execute via Lattice action API",
			}), nil
		}
	}
	return nil, fmt.Errorf("unknown tool %q", p.Name)
}

// resourcesList exposes the ontology spec as MCP resources.
func (s *Server) resourcesList() any {
	if s.cfg.Snapshot == nil {
		return map[string]any{"resources": []any{}}
	}
	out := []map[string]any{
		{
			"uri":         "lattice://ontology",
			"name":        "Ontology",
			"description": "Full ontology snapshot",
			"mimeType":    "application/json",
		},
	}
	for _, ot := range s.cfg.Snapshot.ObjectTypes {
		out = append(out, map[string]any{
			"uri":         "lattice://object_type/" + string(ot.APIName),
			"name":        string(ot.APIName),
			"description": ot.Description,
			"mimeType":    "application/json",
		})
	}
	return map[string]any{"resources": out}
}

func (s *Server) resourcesRead(_ context.Context, params json.RawMessage) (any, error) {
	var p struct {
		URI string `json:"uri"`
	}
	if err := json.Unmarshal(params, &p); err != nil {
		return nil, err
	}
	if s.cfg.Snapshot == nil {
		return nil, fmt.Errorf("no ontology")
	}
	switch {
	case p.URI == "lattice://ontology":
		body, _ := json.MarshalIndent(s.cfg.Snapshot, "", "  ")
		return map[string]any{
			"contents": []map[string]any{{
				"uri":      p.URI,
				"mimeType": "application/json",
				"text":     string(body),
			}},
		}, nil
	case strings.HasPrefix(p.URI, "lattice://object_type/"):
		name := types.APIName(strings.TrimPrefix(p.URI, "lattice://object_type/"))
		ot, ok := s.cfg.Snapshot.ObjectTypeByName(name)
		if !ok {
			return nil, fmt.Errorf("not found: %s", p.URI)
		}
		body, _ := json.MarshalIndent(ot, "", "  ")
		return map[string]any{
			"contents": []map[string]any{{
				"uri":      p.URI,
				"mimeType": "application/json",
				"text":     string(body),
			}},
		}, nil
	}
	return nil, fmt.Errorf("unknown resource: %s", p.URI)
}

// --- helpers ---

func toolNameSearch(ot types.ObjectType) string { return string(ot.APIName) + ".search" }
func toolNameGet(ot types.ObjectType) string    { return string(ot.APIName) + ".get" }

func matchSuffix(snap *types.Ontology, name, suffix string) (types.ObjectType, bool) {
	if !strings.HasSuffix(name, suffix) {
		return types.ObjectType{}, false
	}
	prefix := types.APIName(strings.TrimSuffix(name, suffix))
	return snap.ObjectTypeByName(prefix)
}

func searchInputSchema(ot types.ObjectType) map[string]any {
	props := map[string]any{
		"limit":  map[string]any{"type": "integer", "description": "max records (default 50)"},
		"cursor": map[string]any{"type": "string", "description": "pagination cursor"},
	}
	for _, p := range ot.Properties {
		if !p.Indexed && p.APIName != ot.PrimaryKey {
			continue
		}
		props[string(p.APIName)] = map[string]any{
			"type":        jsonSchemaTypeOf(p.DataType),
			"description": fmt.Sprintf("filter by %s exact match", p.APIName),
		}
	}
	return map[string]any{"type": "object", "properties": props}
}

func getInputSchema(ot types.ObjectType) map[string]any {
	pk, _ := ot.PrimaryKeyProperty()
	return map[string]any{
		"type": "object",
		"properties": map[string]any{
			string(ot.PrimaryKey): map[string]any{
				"type":        jsonSchemaTypeOf(pk.DataType),
				"description": "primary key",
			},
		},
		"required": []string{string(ot.PrimaryKey)},
	}
}

func actionInputSchema(at types.ActionType) map[string]any {
	if len(at.InputSchema) > 0 {
		var m map[string]any
		if err := json.Unmarshal(at.InputSchema, &m); err == nil && m != nil {
			return m
		}
	}
	return map[string]any{"type": "object", "additionalProperties": true}
}

// buildQuerySpec converts a flat arguments map into a QuerySpec.
// Indexed properties become equality filters; limit/cursor go to Page.
func buildQuerySpec(args map[string]any) types.QuerySpec {
	spec := types.QuerySpec{}
	if v, ok := args["limit"]; ok {
		switch n := v.(type) {
		case float64:
			spec.Page.Limit = int(n)
		case int:
			spec.Page.Limit = n
		}
	}
	if v, ok := args["cursor"]; ok {
		if s, ok := v.(string); ok {
			spec.Page.Cursor = s
		}
	}
	for k, v := range args {
		if k == "limit" || k == "cursor" {
			continue
		}
		spec.Filter = types.AndFilters(spec.Filter, types.Filter{
			Property: k,
			Op:       types.FilterOpEq,
			Value:    v,
		})
	}
	return spec
}

func wrapJSONContent(v any) any {
	body, _ := json.MarshalIndent(v, "", "  ")
	return map[string]any{
		"content": []map[string]any{{"type": "text", "text": string(body)}},
	}
}

func jsonSchemaTypeOf(dt types.DataType) string {
	switch dt {
	case types.DataTypeInteger, types.DataTypeBigInt:
		return "integer"
	case types.DataTypeFloat, types.DataTypeDecimal:
		return "number"
	case types.DataTypeBoolean:
		return "boolean"
	case types.DataTypeJSON:
		return "object"
	default:
		return "string"
	}
}
