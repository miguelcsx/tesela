// Package mcp implements a Model Context Protocol server endpoint over
// HTTP and stdio. The protocol is JSON-RPC 2.0; tools, resources and
// prompts are auto-derived from a Lattice ontology snapshot.
//
// The server is intentionally thin: it does not require external state.
// Each request takes a snapshot + capability functions and dispatches.
//
// Reference: https://modelcontextprotocol.io/specification

package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

const ProtocolVersion = "2024-11-05"

// SearchFn is the search adapter the server invokes from tools/call.
type SearchFn func(ctx context.Context, ot types.ObjectType, spec types.QuerySpec) (types.Page, error)

// GetFn is the get adapter.
type GetFn func(ctx context.Context, ot types.ObjectType, pk any) (types.Record, error)

// ServerConfig wires the MCP server to the host App.
type ServerConfig struct {
	ServerName string
	Snapshot   *types.Ontology
	Actor      types.Actor
	Search     SearchFn
	Get        GetFn
}

// Server handles JSON-RPC 2.0 over HTTP (POST) and provides a Run method
// for stdio transports.
type Server struct {
	cfg ServerConfig
}

// NewServer constructs an MCP server bound to a particular ontology snapshot.
func NewServer(cfg ServerConfig) *Server { return &Server{cfg: cfg} }

// rpcReq is the JSON-RPC 2.0 request envelope.
type rpcReq struct {
	JSONRPC string          `json:"jsonrpc"`
	ID      json.RawMessage `json:"id,omitempty"`
	Method  string          `json:"method"`
	Params  json.RawMessage `json:"params,omitempty"`
}

type rpcResp struct {
	JSONRPC string          `json:"jsonrpc"`
	ID      json.RawMessage `json:"id,omitempty"`
	Result  any             `json:"result,omitempty"`
	Error   *rpcErr         `json:"error,omitempty"`
}

type rpcErr struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
	Data    any    `json:"data,omitempty"`
}

// ServeHTTP handles a single JSON-RPC request and writes the response.
func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}
	resp := s.dispatch(r.Context(), body)
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(resp)
}

// Run reads JSON-RPC messages from in (one per line) and writes responses
// to out. Suitable for `--mcp-stdio` style invocations from Claude Desktop.
func (s *Server) Run(ctx context.Context, in io.Reader, out io.Writer) error {
	dec := json.NewDecoder(in)
	enc := json.NewEncoder(out)
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}
		var raw json.RawMessage
		if err := dec.Decode(&raw); err != nil {
			if err == io.EOF {
				return nil
			}
			return err
		}
		resp := s.dispatch(ctx, raw)
		if err := enc.Encode(resp); err != nil {
			return err
		}
	}
}

// dispatch routes the JSON-RPC method to the appropriate handler. Unknown
// methods produce a -32601 "Method not found" error per spec.
func (s *Server) dispatch(ctx context.Context, body []byte) rpcResp {
	var req rpcReq
	if err := json.Unmarshal(body, &req); err != nil {
		return rpcResp{JSONRPC: "2.0", Error: &rpcErr{Code: -32700, Message: "parse error: " + err.Error()}}
	}
	resp := rpcResp{JSONRPC: "2.0", ID: req.ID}
	switch req.Method {
	case "initialize":
		resp.Result = s.initialize()
	case "tools/list":
		resp.Result = s.toolsList()
	case "tools/call":
		out, err := s.toolsCall(ctx, req.Params)
		if err != nil {
			resp.Error = &rpcErr{Code: -32000, Message: err.Error()}
		} else {
			resp.Result = out
		}
	case "resources/list":
		resp.Result = s.resourcesList()
	case "resources/read":
		out, err := s.resourcesRead(ctx, req.Params)
		if err != nil {
			resp.Error = &rpcErr{Code: -32000, Message: err.Error()}
		} else {
			resp.Result = out
		}
	case "prompts/list":
		resp.Result = map[string]any{"prompts": []any{}}
	case "ping":
		resp.Result = map[string]any{}
	default:
		resp.Error = &rpcErr{Code: -32601, Message: fmt.Sprintf("method %q not found", req.Method)}
	}
	return resp
}

// initialize advertises capabilities to the client.
func (s *Server) initialize() any {
	return map[string]any{
		"protocolVersion": ProtocolVersion,
		"serverInfo": map[string]any{
			"name":    s.cfg.ServerName,
			"version": "lattice-dev",
		},
		"capabilities": map[string]any{
			"tools":     map[string]any{"listChanged": false},
			"resources": map[string]any{"listChanged": false, "subscribe": false},
			"prompts":   map[string]any{"listChanged": false},
		},
	}
}
