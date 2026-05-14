package mcp_test

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/mcp"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func newSnapshot() *types.Ontology {
	return &types.Ontology{
		ObjectTypes: []types.ObjectType{{
			APIName:    "Customer",
			PrimaryKey: "id",
			Properties: []types.Property{
				{APIName: "id", DataType: types.DataTypeUUID, Indexed: true},
				{APIName: "email", DataType: types.DataTypeString},
				{APIName: "region", DataType: types.DataTypeString, Indexed: true},
			},
		}},
	}
}

func call(t *testing.T, srv *mcp.Server, method string, params any) map[string]any {
	t.Helper()
	body := map[string]any{"jsonrpc": "2.0", "id": 1, "method": method}
	if params != nil {
		body["params"] = params
	}
	raw, _ := json.Marshal(body)
	r := httptest.NewRequest(http.MethodPost, "/mcp", bytes.NewReader(raw))
	w := httptest.NewRecorder()
	srv.ServeHTTP(w, r)
	if w.Code != http.StatusOK {
		t.Fatalf("status=%d body=%s", w.Code, w.Body.String())
	}
	var out map[string]any
	if err := json.Unmarshal(w.Body.Bytes(), &out); err != nil {
		t.Fatal(err)
	}
	return out
}

func TestMCP_Initialize(t *testing.T) {
	srv := mcp.NewServer(mcp.ServerConfig{ServerName: "test", Snapshot: newSnapshot()})
	out := call(t, srv, "initialize", nil)
	res := out["result"].(map[string]any)
	if res["protocolVersion"].(string) != mcp.ProtocolVersion {
		t.Fatalf("bad protocol version: %v", res["protocolVersion"])
	}
}

func TestMCP_ToolsList(t *testing.T) {
	srv := mcp.NewServer(mcp.ServerConfig{ServerName: "t", Snapshot: newSnapshot()})
	out := call(t, srv, "tools/list", nil)
	tools := out["result"].(map[string]any)["tools"].([]any)
	if len(tools) != 2 {
		t.Fatalf("want 2 tools, got %d", len(tools))
	}
	names := map[string]bool{}
	for _, x := range tools {
		names[x.(map[string]any)["name"].(string)] = true
	}
	if !names["Customer.search"] || !names["Customer.get"] {
		t.Fatalf("missing tool names: %v", names)
	}
}

func TestMCP_ToolsCallSearch(t *testing.T) {
	calls := 0
	srv := mcp.NewServer(mcp.ServerConfig{
		ServerName: "t",
		Snapshot:   newSnapshot(),
		Search: func(_ context.Context, ot types.ObjectType, spec types.QuerySpec) (types.Page, error) {
			calls++
			if ot.APIName != "Customer" {
				t.Fatalf("wrong type: %s", ot.APIName)
			}
			return types.Page{Records: []types.Record{{Values: map[types.APIName]any{"id": "x"}}}}, nil
		},
	})
	out := call(t, srv, "tools/call", map[string]any{
		"name":      "Customer.search",
		"arguments": map[string]any{"region": "EU", "limit": 5},
	})
	res := out["result"].(map[string]any)
	content := res["content"].([]any)[0].(map[string]any)
	text := content["text"].(string)
	if !strings.Contains(text, `"id": "x"`) {
		t.Fatalf("response missing record: %s", text)
	}
	if calls != 1 {
		t.Fatalf("expected 1 search invocation, got %d", calls)
	}
}

func TestMCP_UnknownMethod(t *testing.T) {
	srv := mcp.NewServer(mcp.ServerConfig{ServerName: "t", Snapshot: newSnapshot()})
	out := call(t, srv, "nope/whatever", nil)
	if out["error"] == nil {
		t.Fatalf("expected error, got: %v", out)
	}
}

func TestMCP_Stdio(t *testing.T) {
	srv := mcp.NewServer(mcp.ServerConfig{ServerName: "t", Snapshot: newSnapshot()})
	in := strings.NewReader(`{"jsonrpc":"2.0","id":1,"method":"initialize"}` + "\n")
	out := &bytes.Buffer{}
	if err := srv.Run(context.Background(), in, out); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(out.String(), `"protocolVersion"`) {
		t.Fatalf("stdio response missing protocolVersion: %s", out.String())
	}
	fmt.Sprintf("ok") // touch fmt to not be unused
}
