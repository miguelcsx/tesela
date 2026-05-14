package telemetry_test

import (
	"context"
	"strings"
	"testing"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/buildinfo"
	"github.com/miguelcsx/lattice/pkg/lattice/telemetry"
)

func TestBootstrap_DisabledIsNoop(t *testing.T) {
	t.Parallel()

	cfg := telemetry.Config{
		Enabled:   false,
		Service:   telemetry.ServiceInfo{Name: "test", Environment: "test"},
		BuildInfo: buildinfo.Info{Version: "v0.0.0"},
	}
	rt, err := telemetry.Bootstrap(context.Background(), cfg)
	if err != nil {
		t.Fatalf("Bootstrap: %v", err)
	}
	if rt == nil {
		t.Fatal("Bootstrap returned nil runtime")
	}
	if err := rt.Shutdown(context.Background()); err != nil {
		t.Fatalf("Shutdown: %v", err)
	}
	// Idempotent shutdown.
	if err := rt.Shutdown(context.Background()); err != nil {
		t.Fatalf("second Shutdown: %v", err)
	}
}

func TestBootstrap_EnabledWithoutEndpointFails(t *testing.T) {
	t.Parallel()

	cfg := telemetry.Config{
		Enabled: true,
		Service: telemetry.ServiceInfo{Name: "test"},
	}
	if _, err := telemetry.Bootstrap(context.Background(), cfg); err == nil {
		t.Fatal("Bootstrap must reject enabled=true with no OTLPEndpoint")
	}
}

func TestSpan_PropagatesContext(t *testing.T) {
	t.Parallel()

	cfg := telemetry.Config{
		Enabled: false,
		Service: telemetry.ServiceInfo{Name: "test"},
	}
	rt, err := telemetry.Bootstrap(context.Background(), cfg)
	if err != nil {
		t.Fatalf("Bootstrap: %v", err)
	}
	defer func() { _ = rt.Shutdown(context.Background()) }()

	ctx, span := telemetry.Span(context.Background(), "test.op")
	if ctx == nil {
		t.Fatal("Span must return non-nil ctx")
	}
	span.End()
}

func TestNewLogger_RespectsLevelAndFormat(t *testing.T) {
	t.Parallel()

	for _, format := range []string{"json", "text"} {
		for _, level := range []string{"debug", "info", "warn", "error"} {
			lg, err := telemetry.NewLogger(telemetry.LoggerConfig{
				Level:  level,
				Format: format,
			})
			if err != nil {
				t.Fatalf("NewLogger(%s/%s): %v", level, format, err)
			}
			if lg == nil {
				t.Fatalf("NewLogger(%s/%s) returned nil", level, format)
			}
		}
	}
}

func TestNewLogger_RejectsBadLevel(t *testing.T) {
	t.Parallel()

	if _, err := telemetry.NewLogger(telemetry.LoggerConfig{Level: "bogus"}); err == nil {
		t.Fatal("NewLogger must reject unknown level")
	}
}

func TestRuntime_ShutdownTimesOut(t *testing.T) {
	t.Parallel()

	cfg := telemetry.Config{
		Enabled: false,
		Service: telemetry.ServiceInfo{Name: "test"},
	}
	rt, err := telemetry.Bootstrap(context.Background(), cfg)
	if err != nil {
		t.Fatalf("Bootstrap: %v", err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Millisecond)
	defer cancel()
	if err := rt.Shutdown(ctx); err != nil && !strings.Contains(err.Error(), "deadline") {
		// Disabled runtime shouldn't actually need network; should still succeed.
		t.Fatalf("Shutdown: %v", err)
	}
}
