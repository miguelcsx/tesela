// Configuration schema. Sub-schemas are reused across the three binary
// configs (APIConfig, WorkerConfig, CLIConfig).

package config

// ServiceConfig identifies the running process to telemetry and audit.
type ServiceConfig struct {
	Name        string `koanf:"name"`
	Environment string `koanf:"environment"`
}

// MetadataDBConfig describes the Lattice metadata Postgres connection.
type MetadataDBConfig struct {
	DSN              string `koanf:"dsn"`
	MaxOpenConns     int    `koanf:"max_open_conns"`
	MaxIdleConns     int    `koanf:"max_idle_conns"`
	ConnLifetimeMS   int    `koanf:"conn_lifetime_ms"`
	StatementTimeout int    `koanf:"statement_timeout_ms"`
	MigrateOnStart   bool   `koanf:"migrate_on_start"`
}

// SecretsConfig selects which SecretProvider implementation is active.
type SecretsConfig struct {
	Provider string                 `koanf:"provider"` // env, vault, awssm, gcpsm, k8s, static
	Options  map[string]interface{} `koanf:"options"`
}

// CryptoConfig holds the KEK reference used by internal/crypto.Sealer.
type CryptoConfig struct {
	KEKReference string `koanf:"kek_reference"`
}

// AuthConfig describes how JWT bearer tokens are validated.
type AuthConfig struct {
	Issuer        string   `koanf:"issuer"`
	Audience      string   `koanf:"audience"`
	JWKSURL       string   `koanf:"jwks_url"`
	AcceptedAlgs  []string `koanf:"accepted_algorithms"`
	ClockSkewSecs int      `koanf:"clock_skew_seconds"`
	RolesClaim    string   `koanf:"roles_claim"`
	UserIDClaim   string   `koanf:"user_id_claim"`
}

// TelemetryConfig configures OpenTelemetry export.
type TelemetryConfig struct {
	Enabled       bool    `koanf:"enabled"`
	OTLPEndpoint  string  `koanf:"otlp_endpoint"`
	SamplingRatio float64 `koanf:"sampling_ratio"`
	LogLevel      string  `koanf:"log_level"`
	LogFormat     string  `koanf:"log_format"` // text | json
}

// HTTPConfig is the lattice-api HTTP server configuration.
type HTTPConfig struct {
	Listen              string `koanf:"listen"`
	ReadTimeoutSeconds  int    `koanf:"read_timeout_seconds"`
	WriteTimeoutSeconds int    `koanf:"write_timeout_seconds"`
	IdleTimeoutSeconds  int    `koanf:"idle_timeout_seconds"`
	MaxBodyBytes        int64  `koanf:"max_body_bytes"`
	ShutdownGraceSecs   int    `koanf:"shutdown_grace_seconds"`
}

// WorkerSettings configures the lattice-worker River runtime.
type WorkerSettings struct {
	MaxConcurrentJobs int    `koanf:"max_concurrent_jobs"`
	HealthListen      string `koanf:"health_listen"`
	ShutdownGraceSecs int    `koanf:"shutdown_grace_seconds"`
	QueueName         string `koanf:"queue_name"`
}

// APIConfig is the configuration consumed by cmd/lattice-api.
type APIConfig struct {
	Service    ServiceConfig    `koanf:"service"`
	MetadataDB MetadataDBConfig `koanf:"metadata_db"`
	Secrets    SecretsConfig    `koanf:"secrets"`
	Crypto     CryptoConfig     `koanf:"crypto"`
	Auth       AuthConfig       `koanf:"auth"`
	Telemetry  TelemetryConfig  `koanf:"telemetry"`
	HTTP       HTTPConfig       `koanf:"http"`
}

// WorkerConfig is the configuration consumed by cmd/lattice-worker.
type WorkerConfig struct {
	Service    ServiceConfig    `koanf:"service"`
	MetadataDB MetadataDBConfig `koanf:"metadata_db"`
	Secrets    SecretsConfig    `koanf:"secrets"`
	Crypto     CryptoConfig     `koanf:"crypto"`
	Telemetry  TelemetryConfig  `koanf:"telemetry"`
	Worker     WorkerSettings   `koanf:"worker"`
}

// CLIConfig is the configuration consumed by cmd/lattice. Most fields are
// optional and used only by subcommands that touch the database directly
// (e.g., `lattice db migrate`).
type CLIConfig struct {
	Service    ServiceConfig    `koanf:"service"`
	MetadataDB MetadataDBConfig `koanf:"metadata_db"`
	Secrets    SecretsConfig    `koanf:"secrets"`
	Crypto     CryptoConfig     `koanf:"crypto"`
	Telemetry  TelemetryConfig  `koanf:"telemetry"`
	Server     CLIServerConfig  `koanf:"server"`
}

// CLIServerConfig points the CLI at a running lattice-api.
type CLIServerConfig struct {
	URL   string `koanf:"url"`
	Token string `koanf:"token"`
}
