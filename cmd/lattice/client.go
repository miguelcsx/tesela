// httpClient is the thin REST client every CLI subcommand uses.

package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"time"

	"github.com/spf13/cobra"

	"github.com/miguelcsx/lattice/pkg/lattice/config"
)

type httpClient struct {
	baseURL   string
	token     string
	workspace string
	http      *http.Client
}

func newClient(cmd *cobra.Command) (*httpClient, error) {
	cfg, err := config.LoadCLI(config.LoadOptions{File: stringFlag(cmd, "config")})
	if err != nil {
		return nil, fmt.Errorf("load config: %w", err)
	}
	server := stringFlag(cmd, "server")
	if server == "" {
		server = cfg.Server.URL
	}
	if server == "" {
		return nil, errors.New("--server or config server.url is required")
	}
	token := stringFlag(cmd, "token")
	if token == "" {
		token = cfg.Server.Token
	}
	if token == "" {
		token = os.Getenv("LATTICE_TOKEN")
	}
	workspace := stringFlag(cmd, "workspace")
	return &httpClient{
		baseURL:   server,
		token:     token,
		workspace: workspace,
		http:      &http.Client{Timeout: 60 * time.Second},
	}, nil
}

func (c *httpClient) request(ctx context.Context, method, path string, body io.Reader, contentType string) (*http.Response, error) {
	req, err := http.NewRequestWithContext(ctx, method, c.baseURL+path, body)
	if err != nil {
		return nil, err
	}
	if c.token != "" {
		req.Header.Set("Authorization", "Bearer "+c.token)
	}
	if contentType != "" {
		req.Header.Set("Content-Type", contentType)
	}
	req.Header.Set("Accept", "application/json")
	return c.http.Do(req)
}

func (c *httpClient) postJSON(ctx context.Context, path string, body any) ([]byte, error) {
	raw, err := json.Marshal(body)
	if err != nil {
		return nil, err
	}
	return c.do(ctx, http.MethodPost, path, bytes.NewReader(raw), "application/json")
}

func (c *httpClient) get(ctx context.Context, path string) ([]byte, error) {
	return c.do(ctx, http.MethodGet, path, nil, "")
}

func (c *httpClient) postRaw(ctx context.Context, path string, body []byte, contentType string) ([]byte, error) {
	return c.do(ctx, http.MethodPost, path, bytes.NewReader(body), contentType)
}

func (c *httpClient) do(ctx context.Context, method, path string, body io.Reader, contentType string) ([]byte, error) {
	resp, err := c.request(ctx, method, path, body, contentType)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	raw, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}
	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("server %d: %s", resp.StatusCode, string(raw))
	}
	return raw, nil
}

func stringFlag(cmd *cobra.Command, name string) string {
	v, _ := cmd.Flags().GetString(name)
	if v != "" {
		return v
	}
	v, _ = cmd.PersistentFlags().GetString(name)
	if v != "" {
		return v
	}
	if root := cmd.Root(); root != cmd {
		v, _ = root.PersistentFlags().GetString(name)
	}
	return v
}
