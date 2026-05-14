// Runtime owns the OpenTelemetry providers; Shutdown drains them in reverse
// order of registration. Currently the package only sets up tracing; metrics
// and logs export are added when the Phase 1 observability requirements are
// implemented (Prometheus exporter for metrics is wired separately).

package telemetry

import (
	"context"
	"errors"
	"fmt"
	"sync"

	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/exporters/otlp/otlptrace"
	"go.opentelemetry.io/otel/exporters/otlp/otlptrace/otlptracegrpc"
	"go.opentelemetry.io/otel/sdk/resource"
	tracesdk "go.opentelemetry.io/otel/sdk/trace"
	semconv "go.opentelemetry.io/otel/semconv/v1.27.0"

	"github.com/miguelcsx/lattice/pkg/lattice/buildinfo"
)

// ServiceInfo identifies the service for resource attributes.
type ServiceInfo struct {
	Name        string
	Environment string
}

// Config controls Bootstrap.
type Config struct {
	Enabled       bool
	Service       ServiceInfo
	OTLPEndpoint  string
	SamplingRatio float64
	BuildInfo     buildinfo.Info
}

// Runtime is the handle returned by Bootstrap. Shutdown is idempotent and
// safe to call from any goroutine.
type Runtime struct {
	closers []func(context.Context) error
	once    sync.Once
}

// Bootstrap initializes the global OpenTelemetry providers and returns a
// runtime handle. When cfg.Enabled is false, Bootstrap returns a no-op runtime
// that still satisfies the same contract.
func Bootstrap(ctx context.Context, cfg Config) (*Runtime, error) {
	if !cfg.Enabled {
		return &Runtime{}, nil
	}
	if cfg.OTLPEndpoint == "" {
		return nil, errors.New("telemetry: enabled but OTLPEndpoint is empty")
	}

	res, err := buildResource(cfg)
	if err != nil {
		return nil, err
	}

	tp, err := buildTracerProvider(ctx, cfg, res)
	if err != nil {
		return nil, err
	}
	otel.SetTracerProvider(tp)

	rt := &Runtime{}
	rt.register(tp.Shutdown)
	return rt, nil
}

func buildResource(cfg Config) (*resource.Resource, error) {
	attrs := []any{
		semconv.ServiceName(cfg.Service.Name),
		semconv.ServiceVersion(cfg.BuildInfo.Version),
	}
	if cfg.Service.Environment != "" {
		attrs = append(attrs, semconv.DeploymentEnvironmentName(cfg.Service.Environment))
	}
	if cfg.BuildInfo.Commit != "" {
		attrs = append(attrs, semconv.ServiceInstanceID(cfg.BuildInfo.Commit))
	}
	res, err := resource.New(context.Background(), resource.WithAttributes(toAttrs(attrs)...))
	if err != nil {
		return nil, fmt.Errorf("telemetry: build resource: %w", err)
	}
	return res, nil
}

func buildTracerProvider(ctx context.Context, cfg Config, res *resource.Resource) (*tracesdk.TracerProvider, error) {
	exporter, err := otlptrace.New(ctx, otlptracegrpc.NewClient(
		otlptracegrpc.WithEndpoint(cfg.OTLPEndpoint),
		otlptracegrpc.WithInsecure(),
	))
	if err != nil {
		return nil, fmt.Errorf("telemetry: otlp trace exporter: %w", err)
	}
	sampler := tracesdk.ParentBased(tracesdk.TraceIDRatioBased(samplingRatioOrDefault(cfg.SamplingRatio)))
	tp := tracesdk.NewTracerProvider(
		tracesdk.WithBatcher(exporter),
		tracesdk.WithResource(res),
		tracesdk.WithSampler(sampler),
	)
	return tp, nil
}

func samplingRatioOrDefault(r float64) float64 {
	if r <= 0 {
		return 1.0
	}
	if r > 1 {
		return 1.0
	}
	return r
}

// register adds c to the close set. Run order is FIFO at Shutdown time.
func (r *Runtime) register(c func(context.Context) error) {
	r.closers = append(r.closers, c)
}

// Shutdown drains every registered closer. Safe to call multiple times.
func (r *Runtime) Shutdown(ctx context.Context) error {
	var first error
	r.once.Do(func() {
		for _, c := range r.closers {
			if err := c(ctx); err != nil && first == nil {
				first = err
			}
		}
	})
	return first
}
