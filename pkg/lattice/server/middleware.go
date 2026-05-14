// HTTP middleware: request id, structured logging, panic recovery, JWT auth,
// timeout. Each middleware is a single function that wraps an http.Handler.

package server

import (
	"context"
	"errors"
	"log/slog"
	"net/http"
	"runtime/debug"
	"time"

	"github.com/go-chi/chi/v5/middleware"

	"github.com/miguelcsx/lattice/pkg/lattice/auth"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type ctxKey string

const (
	ctxKeyRequestID ctxKey = "request_id"
	ctxKeyActor     ctxKey = "actor"
)

// requestIDMiddleware ensures each request has an X-Request-ID — accepting
// the inbound header when present, generating one otherwise.
func requestIDMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		id := r.Header.Get("X-Request-ID")
		if id == "" {
			id = ids.NewULID()
		}
		w.Header().Set("X-Request-ID", id)
		ctx := context.WithValue(r.Context(), ctxKeyRequestID, id)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

// loggingMiddleware emits one structured log line per request.
func loggingMiddleware(logger *slog.Logger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			start := time.Now()
			ww := middleware.NewWrapResponseWriter(w, r.ProtoMajor)
			next.ServeHTTP(ww, r)
			logger.Info("http request",
				"method", r.Method,
				"path", r.URL.Path,
				"status", ww.Status(),
				"bytes", ww.BytesWritten(),
				"duration_ms", time.Since(start).Milliseconds(),
				"request_id", requestIDFromContext(r.Context()),
			)
		})
	}
}

// recoverMiddleware turns a panic into a 500 with a logged stack.
func recoverMiddleware(logger *slog.Logger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			defer func() {
				if v := recover(); v != nil {
					logger.Error("panic recovered",
						"panic", v,
						"path", r.URL.Path,
						"stack", string(debug.Stack()),
						"request_id", requestIDFromContext(r.Context()),
					)
					writeError(w, r, errs.New(errs.CodeInternal, "internal error"))
				}
			}()
			next.ServeHTTP(w, r)
		})
	}
}

// timeoutMiddleware applies an absolute per-request deadline.
func timeoutMiddleware(d time.Duration) func(http.Handler) http.Handler {
	if d <= 0 {
		d = 30 * time.Second
	}
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ctx, cancel := context.WithTimeout(r.Context(), d)
			defer cancel()
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// authMiddleware verifies the bearer token and stores the actor in context.
// Routes that should be public must be declared before this middleware in
// the chain (the router uses sub-routers for that).
func authMiddleware(authn *auth.JWTAuthenticator) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			actor, err := authn.Authenticate(r.Context(), r.Header.Get("Authorization"))
			if err != nil {
				writeError(w, r, err)
				return
			}
			ctx := context.WithValue(r.Context(), ctxKeyActor, actor)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

func actorFromContext(ctx context.Context) (types.Actor, error) {
	v, ok := ctx.Value(ctxKeyActor).(types.Actor)
	if !ok {
		return types.Actor{}, errors.New("actor missing from context")
	}
	return v, nil
}

func requestIDFromContext(ctx context.Context) string {
	v, _ := ctx.Value(ctxKeyRequestID).(string)
	return v
}
