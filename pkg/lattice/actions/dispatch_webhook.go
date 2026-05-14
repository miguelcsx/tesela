// Webhook handler — POSTs the action input to the configured URL with a
// HMAC-SHA256 body signature, retries with exponential+jitter backoff up to
// max_retries.

package actions

import (
	"bytes"
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	"github.com/cenkalti/backoff/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/secrets"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// WebhookHandler dispatches HTTP webhook actions.
type WebhookHandler struct {
	client  *http.Client
	secrets secrets.SecretProvider
}

// NewWebhookHandler returns a webhook handler with a 30s default client.
func NewWebhookHandler(s secrets.SecretProvider) *WebhookHandler {
	return &WebhookHandler{
		client:  &http.Client{Timeout: 30 * time.Second},
		secrets: s,
	}
}

// Dispatch implements Handler.
func (h *WebhookHandler) Dispatch(ctx context.Context, ev DispatchEvent) (DispatchResult, error) {
	cfg := ev.ActionType.Handler.Webhook
	if cfg == nil || cfg.URL == "" {
		return DispatchResult{}, fmt.Errorf("webhook: url is required")
	}
	body, err := json.Marshal(buildWebhookBody(ev))
	if err != nil {
		return DispatchResult{}, fmt.Errorf("webhook: marshal body: %w", err)
	}
	signingKey, err := h.resolveSigningKey(ctx, cfg.SigningSecretRef)
	if err != nil {
		return DispatchResult{}, err
	}
	op := func() ([]byte, error) {
		return h.attempt(ctx, ev, cfg, body, signingKey)
	}
	bo := buildBackoff(cfg)
	out, err := backoff.Retry(ctx, op, backoff.WithBackOff(bo), backoff.WithMaxTries(uint(maxTries(cfg))))
	if err != nil {
		return DispatchResult{}, fmt.Errorf("webhook: %w", err)
	}
	return DispatchResult{Output: out}, nil
}

func (h *WebhookHandler) attempt(ctx context.Context, ev DispatchEvent, cfg *types.WebhookHandler, body []byte, signingKey string) ([]byte, error) {
	timeout := time.Duration(cfg.TimeoutSeconds) * time.Second
	if timeout <= 0 {
		timeout = 30 * time.Second
	}
	cctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	req, err := http.NewRequestWithContext(cctx, http.MethodPost, cfg.URL, bytes.NewReader(body))
	if err != nil {
		return nil, backoff.Permanent(err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	req.Header.Set("X-Lattice-Action", string(ev.ActionType.APIName))
	req.Header.Set("X-Lattice-Idempotency-Key", ev.IdempotencyKey)
	if signingKey != "" {
		ts := fmt.Sprintf("%d", time.Now().Unix())
		mac := hmac.New(sha256.New, []byte(signingKey))
		mac.Write([]byte(ts))
		mac.Write([]byte("."))
		mac.Write(body)
		req.Header.Set("X-Lattice-Signature", "t="+ts+",v1="+hex.EncodeToString(mac.Sum(nil)))
	}
	resp, err := h.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	respBody, _ := io.ReadAll(resp.Body)
	if resp.StatusCode >= 200 && resp.StatusCode < 300 {
		return respBody, nil
	}
	if !shouldRetry(cfg, resp.StatusCode) {
		return nil, backoff.Permanent(fmt.Errorf("webhook returned %d: %s", resp.StatusCode, string(respBody)))
	}
	return nil, fmt.Errorf("webhook returned %d", resp.StatusCode)
}

func (h *WebhookHandler) resolveSigningKey(ctx context.Context, ref string) (string, error) {
	if ref == "" || h.secrets == nil {
		return "", nil
	}
	v, err := h.secrets.Lookup(ctx, ref)
	if err != nil {
		return "", fmt.Errorf("webhook signing key: %w", err)
	}
	return v, nil
}

type webhookBody struct {
	ActionType types.APIName  `json:"action_type"`
	Workspace  string         `json:"workspace_id"`
	Actor      webhookActor   `json:"actor"`
	Input      map[string]any `json:"input"`
	Subject    map[string]any `json:"subject,omitempty"`
}

type webhookActor struct {
	UserID string   `json:"user_id"`
	Roles  []string `json:"roles,omitempty"`
}

func buildWebhookBody(ev DispatchEvent) webhookBody {
	out := webhookBody{
		ActionType: ev.ActionType.APIName,
		Workspace:  string(ev.Workspace.ID),
		Actor:      webhookActor{UserID: ev.Actor.UserID, Roles: append([]string(nil), ev.Actor.Roles...)},
		Input:      ev.Input,
	}
	if ev.Subject != nil {
		s := make(map[string]any, len(ev.Subject.Values))
		for k, v := range ev.Subject.Values {
			s[string(k)] = v
		}
		out.Subject = s
	}
	return out
}

func buildBackoff(cfg *types.WebhookHandler) backoff.BackOff {
	initial := time.Duration(cfg.BackoffInitialMS) * time.Millisecond
	if initial <= 0 {
		initial = 200 * time.Millisecond
	}
	max := time.Duration(cfg.BackoffMaxMS) * time.Millisecond
	if max <= 0 {
		max = 30 * time.Second
	}
	jitter := cfg.BackoffJitter
	if jitter <= 0 {
		jitter = 0.2
	}
	bo := backoff.NewExponentialBackOff()
	bo.InitialInterval = initial
	bo.MaxInterval = max
	bo.RandomizationFactor = jitter
	return bo
}

func maxTries(cfg *types.WebhookHandler) int {
	if cfg.MaxRetries <= 0 {
		return 3
	}
	return cfg.MaxRetries + 1
}

func shouldRetry(cfg *types.WebhookHandler, status int) bool {
	if status >= 500 || status == http.StatusTooManyRequests {
		return true
	}
	for _, s := range cfg.RetryOnStatus {
		if s == status {
			return true
		}
	}
	return false
}
