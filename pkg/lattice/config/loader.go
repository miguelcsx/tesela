// Layered loader: defaults → file → env → overrides. Returns a typed config
// struct or a precise error pointing at the missing field.

package config

import (
	"errors"
	"fmt"
	"os"
	"strings"

	"github.com/knadh/koanf/parsers/yaml"
	"github.com/knadh/koanf/providers/confmap"
	"github.com/knadh/koanf/providers/env"
	"github.com/knadh/koanf/providers/file"
	"github.com/knadh/koanf/v2"
)

// envPrefix is stripped from environment variable names before mapping.
const envPrefix = "LATTICE_"

// LoadOptions controls a single load. File is optional; Overrides are merged
// last and intended primarily for tests.
type LoadOptions struct {
	File      string
	Overrides map[string]any
}

// LoadAPI hydrates an APIConfig from the layered sources.
func LoadAPI(opts LoadOptions) (*APIConfig, error) {
	var cfg APIConfig
	if err := loadInto(opts, apiDefaults(), &cfg); err != nil {
		return nil, err
	}
	if err := validateAPI(&cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}

// LoadWorker hydrates a WorkerConfig from the layered sources.
func LoadWorker(opts LoadOptions) (*WorkerConfig, error) {
	var cfg WorkerConfig
	if err := loadInto(opts, workerDefaults(), &cfg); err != nil {
		return nil, err
	}
	if err := validateWorker(&cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}

// LoadCLI hydrates a CLIConfig from the layered sources. CLI loading is more
// permissive: most fields are optional.
func LoadCLI(opts LoadOptions) (*CLIConfig, error) {
	var cfg CLIConfig
	if err := loadInto(opts, cliDefaults(), &cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}

// loadInto runs the four-layer merge into target.
func loadInto(opts LoadOptions, defaults map[string]any, target any) error {
	k := koanf.New(".")

	if err := k.Load(confmap.Provider(defaults, "."), nil); err != nil {
		return fmt.Errorf("load defaults: %w", err)
	}

	if opts.File != "" {
		if _, err := os.Stat(opts.File); err != nil {
			return fmt.Errorf("config file: %w", err)
		}
		if err := k.Load(file.Provider(opts.File), yaml.Parser()); err != nil {
			return fmt.Errorf("parse %s: %w", opts.File, err)
		}
	}

	if err := k.Load(env.Provider(envPrefix, ".", envKeyTransform), nil); err != nil {
		return fmt.Errorf("load env: %w", err)
	}

	if len(opts.Overrides) > 0 {
		if err := k.Load(confmap.Provider(opts.Overrides, "."), nil); err != nil {
			return fmt.Errorf("load overrides: %w", err)
		}
	}

	if err := k.Unmarshal("", target); err != nil {
		return fmt.Errorf("unmarshal: %w", err)
	}
	return nil
}

// envKeyTransform maps an environment variable name onto a koanf path.
//
// Sections are separated by double underscore (__); single underscores stay
// inside section names so multi-word sections like metadata_db remain intact.
//
// Examples:
//
//	LATTICE_HTTP__LISTEN              → http.listen
//	LATTICE_METADATA_DB__DSN          → metadata_db.dsn
//	LATTICE_AUTH__JWKS_URL            → auth.jwks_url
//	LATTICE_AUTH__ACCEPTED_ALGORITHMS → auth.accepted_algorithms
//
// Single-section keys (LATTICE_HTTP_LISTEN with one underscore) are also
// supported for ergonomic reasons but only resolve when the schema has no
// multi-word section sharing that prefix.
func envKeyTransform(key string) string {
	trimmed := strings.TrimPrefix(key, envPrefix)
	lower := strings.ToLower(trimmed)
	if strings.Contains(lower, "__") {
		parts := strings.Split(lower, "__")
		return strings.Join(parts, ".")
	}
	return strings.ReplaceAll(lower, "_", ".")
}

// validateAPI enforces presence of the fields that have no safe default.
func validateAPI(cfg *APIConfig) error {
	return errors.Join(
		requireString("metadata_db.dsn", cfg.MetadataDB.DSN),
		requireString("crypto.kek_reference", cfg.Crypto.KEKReference),
		requireString("auth.issuer", cfg.Auth.Issuer),
		requireString("auth.audience", cfg.Auth.Audience),
	)
}

// validateWorker enforces presence of fields with no safe default.
func validateWorker(cfg *WorkerConfig) error {
	return errors.Join(
		requireString("metadata_db.dsn", cfg.MetadataDB.DSN),
		requireString("crypto.kek_reference", cfg.Crypto.KEKReference),
	)
}

// requireString returns a precise error when value is empty.
func requireString(field, value string) error {
	if value == "" {
		return fmt.Errorf("config: %s is required", field)
	}
	return nil
}
