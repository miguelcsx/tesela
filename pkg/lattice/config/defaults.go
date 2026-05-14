// Built-in defaults for each binary's config. Defaults are applied first in
// the layered loader; YAML files and env variables override them.

package config

// apiDefaults returns the baseline APIConfig.
func apiDefaults() map[string]any {
	return map[string]any{
		"service": map[string]any{
			"name":        "lattice-api",
			"environment": "development",
		},
		"metadata_db": map[string]any{
			"max_open_conns":       25,
			"max_idle_conns":       5,
			"conn_lifetime_ms":     30 * 60 * 1000, // 30 minutes
			"statement_timeout_ms": 30000,
			"migrate_on_start":     false,
		},
		"secrets": map[string]any{
			"provider": "env",
		},
		"telemetry": map[string]any{
			"enabled":        false,
			"sampling_ratio": 1.0,
			"log_level":      "info",
			"log_format":     "json",
		},
		"http": map[string]any{
			"listen":                 ":8080",
			"read_timeout_seconds":   15,
			"write_timeout_seconds":  60,
			"idle_timeout_seconds":   120,
			"max_body_bytes":         int64(16 * 1024 * 1024),
			"shutdown_grace_seconds": 30,
		},
		"auth": map[string]any{
			"accepted_algorithms": []string{"RS256", "ES256"},
			"clock_skew_seconds":  60,
			"roles_claim":         "roles",
			"user_id_claim":       "sub",
		},
	}
}

// workerDefaults returns the baseline WorkerConfig.
func workerDefaults() map[string]any {
	return map[string]any{
		"service": map[string]any{
			"name":        "lattice-worker",
			"environment": "development",
		},
		"metadata_db": map[string]any{
			"max_open_conns":       10,
			"max_idle_conns":       2,
			"conn_lifetime_ms":     30 * 60 * 1000,
			"statement_timeout_ms": 60000,
			"migrate_on_start":     false,
		},
		"secrets": map[string]any{
			"provider": "env",
		},
		"telemetry": map[string]any{
			"enabled":        false,
			"sampling_ratio": 1.0,
			"log_level":      "info",
			"log_format":     "json",
		},
		"worker": map[string]any{
			"max_concurrent_jobs":    20,
			"health_listen":          ":8081",
			"shutdown_grace_seconds": 60,
			"queue_name":             "default",
		},
	}
}

// cliDefaults returns the baseline CLIConfig.
func cliDefaults() map[string]any {
	return map[string]any{
		"service": map[string]any{
			"name":        "lattice-cli",
			"environment": "development",
		},
		"telemetry": map[string]any{
			"enabled":    false,
			"log_level":  "info",
			"log_format": "text",
		},
		"server": map[string]any{
			"url": "http://localhost:8080",
		},
	}
}
