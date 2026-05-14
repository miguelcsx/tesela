package config_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/config"
)

const minimalAPIYAML = `
service:
  name: lattice-api
metadata_db:
  dsn: postgres://app:pwd@localhost:5432/lattice
secrets:
  provider: env
crypto:
  kek_reference: LATTICE_KEK
auth:
  issuer: https://example.com/
  audience: lattice
`

func writeFile(t *testing.T, dir, name, content string) string {
	t.Helper()
	path := filepath.Join(dir, name)
	if err := os.WriteFile(path, []byte(content), 0o600); err != nil {
		t.Fatalf("WriteFile: %v", err)
	}
	return path
}

func TestLoadAPI_AppliesDefaults(t *testing.T) {
	dir := t.TempDir()
	path := writeFile(t, dir, "lattice.yaml", minimalAPIYAML)

	cfg, err := config.LoadAPI(config.LoadOptions{File: path})
	if err != nil {
		t.Fatalf("LoadAPI: %v", err)
	}
	if cfg.Service.Name != "lattice-api" {
		t.Fatalf("Service.Name: %q", cfg.Service.Name)
	}
	if cfg.HTTP.Listen == "" {
		t.Fatal("default HTTP.Listen must not be empty")
	}
	if cfg.HTTP.ReadTimeoutSeconds == 0 {
		t.Fatal("default HTTP.ReadTimeoutSeconds must be > 0")
	}
}

func TestLoadAPI_FileOverridesDefaults(t *testing.T) {
	yaml := minimalAPIYAML + `
http:
  listen: ":9090"
  read_timeout_seconds: 30
`
	dir := t.TempDir()
	path := writeFile(t, dir, "lattice.yaml", yaml)

	cfg, err := config.LoadAPI(config.LoadOptions{File: path})
	if err != nil {
		t.Fatalf("LoadAPI: %v", err)
	}
	if cfg.HTTP.Listen != ":9090" {
		t.Fatalf("Listen: %q", cfg.HTTP.Listen)
	}
	if cfg.HTTP.ReadTimeoutSeconds != 30 {
		t.Fatalf("ReadTimeoutSeconds: %d", cfg.HTTP.ReadTimeoutSeconds)
	}
}

func TestLoadAPI_EnvOverridesFile(t *testing.T) {
	dir := t.TempDir()
	path := writeFile(t, dir, "lattice.yaml", minimalAPIYAML)

	t.Setenv("LATTICE_HTTP__LISTEN", ":7777")
	cfg, err := config.LoadAPI(config.LoadOptions{File: path})
	if err != nil {
		t.Fatalf("LoadAPI: %v", err)
	}
	if cfg.HTTP.Listen != ":7777" {
		t.Fatalf("Env override: got %q", cfg.HTTP.Listen)
	}
}

func TestLoadAPI_RejectsMissingDSN(t *testing.T) {
	yaml := `
service:
  name: lattice-api
secrets:
  provider: env
crypto:
  kek_reference: LATTICE_KEK
auth:
  issuer: https://x/
  audience: a
`
	dir := t.TempDir()
	path := writeFile(t, dir, "lattice.yaml", yaml)

	if _, err := config.LoadAPI(config.LoadOptions{File: path}); err == nil {
		t.Fatal("LoadAPI must reject missing metadata_db.dsn")
	}
}

func TestLoadWorker_AppliesDefaults(t *testing.T) {
	yaml := `
service:
  name: lattice-worker
metadata_db:
  dsn: postgres://app@localhost/lattice
secrets:
  provider: env
crypto:
  kek_reference: LATTICE_KEK
`
	dir := t.TempDir()
	path := writeFile(t, dir, "lattice.yaml", yaml)

	cfg, err := config.LoadWorker(config.LoadOptions{File: path})
	if err != nil {
		t.Fatalf("LoadWorker: %v", err)
	}
	if cfg.Worker.MaxConcurrentJobs == 0 {
		t.Fatal("default MaxConcurrentJobs must be > 0")
	}
}

func TestLoadAPI_MissingFileIsError(t *testing.T) {
	if _, err := config.LoadAPI(config.LoadOptions{File: "/does/not/exist.yaml"}); err == nil {
		t.Fatal("LoadAPI must error on missing file")
	}
}

func TestLoadAPI_NoFileFallsBackToDefaultsAndEnv(t *testing.T) {
	t.Setenv("LATTICE_METADATA_DB__DSN", "postgres://x@y/z")
	t.Setenv("LATTICE_CRYPTO__KEK_REFERENCE", "K")
	t.Setenv("LATTICE_AUTH__ISSUER", "https://issuer/")
	t.Setenv("LATTICE_AUTH__AUDIENCE", "aud")

	cfg, err := config.LoadAPI(config.LoadOptions{})
	if err != nil {
		t.Fatalf("LoadAPI without file: %v", err)
	}
	if cfg.MetadataDB.DSN == "" {
		t.Fatal("DSN should have been read from env")
	}
}
