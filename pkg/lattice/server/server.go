// Server is the HTTP entry point. NewServer wires every dependency the
// router needs and returns *http.Server ready to call ListenAndServe on.

package server

import (
	"errors"
	"log/slog"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/actions"
	"github.com/miguelcsx/lattice/pkg/lattice/agents"
	"github.com/miguelcsx/lattice/pkg/lattice/audit"
	"github.com/miguelcsx/lattice/pkg/lattice/auth"
	"github.com/miguelcsx/lattice/pkg/lattice/buildinfo"
	"github.com/miguelcsx/lattice/pkg/lattice/config"
	gqlpkg "github.com/miguelcsx/lattice/pkg/lattice/graphql"
	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/query"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/upload"
)

// ServerConfig is the dependency bundle NewServer consumes.
type ServerConfig struct {
	HTTP           config.HTTPConfig
	BuildInfo      buildinfo.Info
	Store          *storage.Store
	Authenticator  *auth.JWTAuthenticator
	Ontology       *ontology.Registry
	QueryPipeline  *query.Pipeline
	ActionPipeline *actions.Pipeline
	UploadManager  *upload.Manager
	AgentRuntime   *agents.Runtime
	GraphQL        *gqlpkg.SchemaCache
	Audit          *audit.Writer
	Logger         *slog.Logger
}

// NewServer builds the *http.Server. The handler is a chi router with the
// standard middleware chain.
func NewServer(cfg ServerConfig) (*http.Server, error) {
	if cfg.Logger == nil {
		cfg.Logger = slog.Default()
	}
	if err := validateServerConfig(cfg); err != nil {
		return nil, err
	}
	r := chi.NewRouter()
	installMiddleware(r, cfg)
	installRoutes(r, cfg)

	listen := cfg.HTTP.Listen
	if listen == "" {
		listen = ":8080"
	}
	return &http.Server{
		Addr:              listen,
		Handler:           r,
		ReadTimeout:       toDuration(cfg.HTTP.ReadTimeoutSeconds, 30),
		WriteTimeout:      toDuration(cfg.HTTP.WriteTimeoutSeconds, 60),
		IdleTimeout:       toDuration(cfg.HTTP.IdleTimeoutSeconds, 120),
		ReadHeaderTimeout: 10 * time.Second,
	}, nil
}

func validateServerConfig(cfg ServerConfig) error {
	if cfg.Store == nil {
		return errors.New("api: Store is required")
	}
	if cfg.Ontology == nil {
		return errors.New("api: Ontology is required")
	}
	if cfg.QueryPipeline == nil {
		return errors.New("api: QueryPipeline is required")
	}
	if cfg.Authenticator == nil {
		return errors.New("api: Authenticator is required")
	}
	return nil
}

func installMiddleware(r chi.Router, cfg ServerConfig) {
	r.Use(requestIDMiddleware)
	r.Use(recoverMiddleware(cfg.Logger))
	r.Use(loggingMiddleware(cfg.Logger))
	r.Use(timeoutMiddleware(toDuration(cfg.HTTP.ReadTimeoutSeconds, 30)))
}

func toDuration(secs, def int) time.Duration {
	if secs <= 0 {
		secs = def
	}
	return time.Duration(secs) * time.Second
}
