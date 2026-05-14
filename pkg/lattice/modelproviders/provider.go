// Provider is the contract every LLM backend implements. Tools are
// described in OpenAI function-calling format internally; per-provider
// implementations translate to the native protocol.

package modelproviders

import (
	"context"
	"encoding/json"
)

// Message is a single chat-completion message.
type Message struct {
	Role       string          `json:"role"` // system | user | assistant | tool
	Content    string          `json:"content,omitempty"`
	ToolCalls  []ToolCall      `json:"tool_calls,omitempty"`
	ToolCallID string          `json:"tool_call_id,omitempty"`
	Name       string          `json:"name,omitempty"`
	ToolResult json.RawMessage `json:"tool_result,omitempty"`
}

// ToolCall is a model-issued function invocation.
type ToolCall struct {
	ID        string          `json:"id"`
	Name      string          `json:"name"`
	Arguments json.RawMessage `json:"arguments"`
}

// Tool is the OpenAI-format function description.
type Tool struct {
	Name        string          `json:"name"`
	Description string          `json:"description,omitempty"`
	Schema      json.RawMessage `json:"parameters"`
}

// CallRequest is the input to Provider.Call.
type CallRequest struct {
	Model       string    `json:"model"`
	Messages    []Message `json:"messages"`
	Tools       []Tool    `json:"tools,omitempty"`
	Temperature float64   `json:"temperature,omitempty"`
	MaxTokens   int       `json:"max_tokens,omitempty"`
}

// CallResponse is the output of Provider.Call.
type CallResponse struct {
	Message    Message `json:"message"`
	StopReason string  `json:"stop_reason,omitempty"`
	Usage      Usage   `json:"usage"`
}

// Usage tracks token consumption.
type Usage struct {
	InputTokens  int `json:"input_tokens"`
	OutputTokens int `json:"output_tokens"`
}

// Provider is the contract every LLM backend implements.
type Provider interface {
	Name() string
	Call(ctx context.Context, req CallRequest) (CallResponse, error)
}
