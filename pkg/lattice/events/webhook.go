package events

import (
	"bytes"
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"net/http"
	"time"
)

// WebhookSink POSTs JSON-encoded events to a URL. If Secret is set, each
// request gets a hex-encoded HMAC-SHA256 over the raw body in the
// X-Lattice-Signature header.
//
// The sink is fire-and-retry-with-bounded-attempts: on non-2xx or transport
// error it retries up to MaxRetries with exponential backoff, then gives up
// and logs.
type WebhookSink struct {
	URL        string
	Secret     string
	HTTPClient *http.Client
	MaxRetries int
	OnError    func(err error, e Event)
}

// AsHandler returns a Handler that ships e to the webhook.
func (w *WebhookSink) AsHandler() Handler {
	if w.HTTPClient == nil {
		w.HTTPClient = &http.Client{Timeout: 10 * time.Second}
	}
	if w.MaxRetries == 0 {
		w.MaxRetries = 3
	}
	return func(ctx context.Context, e Event) error {
		body, err := json.Marshal(e)
		if err != nil {
			return err
		}
		var lastErr error
		backoff := 200 * time.Millisecond
		for attempt := 0; attempt < w.MaxRetries; attempt++ {
			req, _ := http.NewRequestWithContext(ctx, http.MethodPost, w.URL, bytes.NewReader(body))
			req.Header.Set("Content-Type", "application/json")
			req.Header.Set("X-Lattice-Event", string(e.Kind))
			if w.Secret != "" {
				mac := hmac.New(sha256.New, []byte(w.Secret))
				_, _ = mac.Write(body)
				req.Header.Set("X-Lattice-Signature", "sha256="+hex.EncodeToString(mac.Sum(nil)))
			}
			resp, err := w.HTTPClient.Do(req)
			if err == nil {
				_ = resp.Body.Close()
				if resp.StatusCode/100 == 2 {
					return nil
				}
				lastErr = errors.New("webhook: non-2xx status: " + resp.Status)
			} else {
				lastErr = err
			}
			time.Sleep(backoff)
			backoff *= 2
		}
		if w.OnError != nil {
			w.OnError(lastErr, e)
		}
		return lastErr
	}
}
