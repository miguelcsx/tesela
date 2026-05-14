// Anthropic provider — wraps the Messages API. The HTTP shape differs from
// OpenAI but every translation lives in one helper file.

package modelproviders

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// Anthropic is a Provider backed by api.anthropic.com.
type Anthropic struct {
	apiKey  string
	baseURL string
	client  *http.Client
}

// NewAnthropic returns a configured provider.
func NewAnthropic(apiKey string) *Anthropic {
	return &Anthropic{
		apiKey:  apiKey,
		baseURL: "https://api.anthropic.com/v1/messages",
		client:  &http.Client{Timeout: 5 * time.Minute},
	}
}

// Name implements Provider.
func (*Anthropic) Name() string { return "anthropic" }

type anthMessage struct {
	Role    string        `json:"role"`
	Content []anthContent `json:"content"`
}

type anthContent struct {
	Type      string          `json:"type"`
	Text      string          `json:"text,omitempty"`
	ID        string          `json:"id,omitempty"`
	Name      string          `json:"name,omitempty"`
	Input     json.RawMessage `json:"input,omitempty"`
	ToolUseID string          `json:"tool_use_id,omitempty"`
	Content   string          `json:"content,omitempty"`
}

type anthRequest struct {
	Model       string         `json:"model"`
	Messages    []anthMessage  `json:"messages"`
	System      string         `json:"system,omitempty"`
	MaxTokens   int            `json:"max_tokens"`
	Temperature float64        `json:"temperature,omitempty"`
	Tools       []anthToolDecl `json:"tools,omitempty"`
}

type anthToolDecl struct {
	Name        string          `json:"name"`
	Description string          `json:"description,omitempty"`
	InputSchema json.RawMessage `json:"input_schema"`
}

type anthResponse struct {
	StopReason string        `json:"stop_reason"`
	Content    []anthContent `json:"content"`
	Usage      struct {
		InputTokens  int `json:"input_tokens"`
		OutputTokens int `json:"output_tokens"`
	} `json:"usage"`
}

// Call implements Provider.
func (a *Anthropic) Call(ctx context.Context, req CallRequest) (CallResponse, error) {
	body, err := json.Marshal(buildAnthRequest(req))
	if err != nil {
		return CallResponse{}, err
	}
	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, a.baseURL, bytes.NewReader(body))
	if err != nil {
		return CallResponse{}, err
	}
	httpReq.Header.Set("x-api-key", a.apiKey)
	httpReq.Header.Set("anthropic-version", "2023-06-01")
	httpReq.Header.Set("content-type", "application/json")
	resp, err := a.client.Do(httpReq)
	if err != nil {
		return CallResponse{}, err
	}
	defer resp.Body.Close()
	raw, _ := io.ReadAll(resp.Body)
	if resp.StatusCode >= 400 {
		return CallResponse{}, fmt.Errorf("anthropic %d: %s", resp.StatusCode, string(raw))
	}
	var ar anthResponse
	if err := json.Unmarshal(raw, &ar); err != nil {
		return CallResponse{}, fmt.Errorf("decode response: %w", err)
	}
	return decodeAnthResponse(ar), nil
}

func buildAnthRequest(req CallRequest) anthRequest {
	out := anthRequest{
		Model: req.Model, MaxTokens: req.MaxTokens,
		Temperature: req.Temperature,
	}
	if out.MaxTokens == 0 {
		out.MaxTokens = 1024
	}
	for _, m := range req.Messages {
		if m.Role == "system" {
			out.System = m.Content
			continue
		}
		out.Messages = append(out.Messages, encodeAnthMessage(m))
	}
	for _, t := range req.Tools {
		out.Tools = append(out.Tools, anthToolDecl{
			Name: t.Name, Description: t.Description, InputSchema: t.Schema,
		})
	}
	return out
}

func encodeAnthMessage(m Message) anthMessage {
	out := anthMessage{Role: m.Role}
	if m.Content != "" {
		out.Content = append(out.Content, anthContent{Type: "text", Text: m.Content})
	}
	for _, tc := range m.ToolCalls {
		out.Content = append(out.Content, anthContent{
			Type: "tool_use", ID: tc.ID, Name: tc.Name, Input: tc.Arguments,
		})
	}
	if m.Role == "tool" {
		out.Role = "user"
		out.Content = []anthContent{{
			Type: "tool_result", ToolUseID: m.ToolCallID, Content: m.Content,
		}}
	}
	return out
}

func decodeAnthResponse(ar anthResponse) CallResponse {
	msg := Message{Role: "assistant"}
	for _, c := range ar.Content {
		switch c.Type {
		case "text":
			msg.Content += c.Text
		case "tool_use":
			msg.ToolCalls = append(msg.ToolCalls, ToolCall{
				ID: c.ID, Name: c.Name, Arguments: c.Input,
			})
		}
	}
	return CallResponse{
		Message: msg, StopReason: ar.StopReason,
		Usage: Usage{InputTokens: ar.Usage.InputTokens, OutputTokens: ar.Usage.OutputTokens},
	}
}
