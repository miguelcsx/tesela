// Structured logger backed by stdlib log/slog. Format and level are read from
// configuration; the resulting *slog.Logger is the canonical logger used by
// every binary.

package telemetry

import (
	"fmt"
	"io"
	"log/slog"
	"os"
	"strings"
)

// LoggerConfig drives NewLogger.
type LoggerConfig struct {
	Level  string // debug | info | warn | error
	Format string // json | text
	Writer io.Writer
}

// validLevels maps configuration strings to slog.Level values. Keeping this
// declarative avoids spreading switch statements across the codebase.
var validLevels = map[string]slog.Level{
	"debug": slog.LevelDebug,
	"info":  slog.LevelInfo,
	"warn":  slog.LevelWarn,
	"error": slog.LevelError,
}

// NewLogger builds a logger conforming to cfg. Writer defaults to os.Stderr.
func NewLogger(cfg LoggerConfig) (*slog.Logger, error) {
	level, ok := validLevels[strings.ToLower(cfg.Level)]
	if !ok {
		return nil, fmt.Errorf("telemetry: unknown log level %q", cfg.Level)
	}
	w := cfg.Writer
	if w == nil {
		w = os.Stderr
	}
	handler := newHandler(strings.ToLower(cfg.Format), w, &slog.HandlerOptions{Level: level})
	return slog.New(handler), nil
}

func newHandler(format string, w io.Writer, opts *slog.HandlerOptions) slog.Handler {
	if format == "text" {
		return slog.NewTextHandler(w, opts)
	}
	// Default to JSON; "json" or any other value (including empty) lands here.
	return slog.NewJSONHandler(w, opts)
}
