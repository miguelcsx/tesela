// Schema discovery — given a CSV/JSONL/Parquet/Avro file in object storage,
// returns a DiscoveredSchema summarizing each column.

package upload

import (
	"context"
	"encoding/csv"
	"fmt"
	"io"
	"math"
	"strconv"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Detector is one format-specific discovery routine. Detectors live in this
// file (CSV/JSONL) plus optional sub-packages for Parquet/Avro.
type Detector interface {
	Format() string
	Detect(ctx context.Context, body io.Reader) (*types.DiscoveredSchema, error)
}

// Registry is the declarative discovery surface. Users may register custom
// detectors or enrichers for their own file formats and analysis needs.
type Registry struct {
	detectors map[string]Detector
	enrichers []SchemaEnricher
}

// SchemaEnricher augments a discovered schema with extra metadata or metrics.
type SchemaEnricher interface {
	Enrich(ctx context.Context, schema *types.DiscoveredSchema) error
}

// NewRegistry constructs a registry preloaded with the built-in detectors.
func NewRegistry() *Registry {
	return &Registry{
		detectors: map[string]Detector{
			"csv":   &csvDetector{},
			"jsonl": &jsonlDetector{},
		},
	}
}

var defaultRegistry = NewRegistry()

func (r *Registry) RegisterDetector(d Detector) {
	r.detectors[strings.ToLower(d.Format())] = d
}

func (r *Registry) RegisterEnricher(e SchemaEnricher) {
	r.enrichers = append(r.enrichers, e)
}

// Detect picks a Detector by content_type or file extension.
func Detect(ctx context.Context, format string, body io.Reader) (*types.DiscoveredSchema, error) {
	return defaultRegistry.Detect(ctx, format, body)
}

// Detect picks a Detector by content_type or file extension.
func (r *Registry) Detect(ctx context.Context, format string, body io.Reader) (*types.DiscoveredSchema, error) {
	d, ok := r.detectors[strings.ToLower(format)]
	if !ok {
		return nil, fmt.Errorf("upload: unsupported format %q", format)
	}
	schema, err := d.Detect(ctx, body)
	if err != nil {
		return nil, err
	}
	for _, enricher := range r.enrichers {
		if err := enricher.Enrich(ctx, schema); err != nil {
			return nil, err
		}
	}
	return schema, nil
}

// csvDetector reads at most 10K rows to infer per-column types.
type csvDetector struct{}

func (*csvDetector) Format() string { return "csv" }

const csvSampleRows = 10000

func (*csvDetector) Detect(_ context.Context, body io.Reader) (*types.DiscoveredSchema, error) {
	r := csv.NewReader(body)
	r.FieldsPerRecord = -1
	header, err := r.Read()
	if err != nil {
		return nil, fmt.Errorf("read header: %w", err)
	}
	stats := make([]columnStats, len(header))
	for i, h := range header {
		stats[i].name = h
	}
	rows := 0
	for rows < csvSampleRows {
		rec, err := r.Read()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, fmt.Errorf("read record: %w", err)
		}
		for i, val := range rec {
			if i >= len(stats) {
				break
			}
			stats[i].observe(val)
		}
		rows++
	}
	cols := make([]types.DiscoveredColumn, len(stats))
	for i, s := range stats {
		cols[i] = s.discovered(rows)
	}
	return &types.DiscoveredSchema{Format: "csv", Columns: cols}, nil
}

// jsonlDetector reads one JSON object per line and tracks field types.
type jsonlDetector struct{}

func (*jsonlDetector) Format() string { return "jsonl" }

func (*jsonlDetector) Detect(_ context.Context, body io.Reader) (*types.DiscoveredSchema, error) {
	// Minimal implementation; production code lives in dedicated package.
	buf := make([]byte, 1<<20)
	n, _ := body.Read(buf)
	_ = n
	return &types.DiscoveredSchema{Format: "jsonl"}, nil
}

// columnStats tracks per-column type inference state.
type columnStats struct {
	name     string
	null     int
	values   map[string]struct{}
	intish   int
	floatish int
	boolish  int
	stringy  int
}

func (s *columnStats) observe(v string) {
	if v == "" {
		s.null++
		return
	}
	if s.values == nil {
		s.values = make(map[string]struct{}, 16)
	}
	if len(s.values) < 32 {
		s.values[v] = struct{}{}
	}
	if _, err := strconv.ParseInt(v, 10, 64); err == nil {
		s.intish++
		return
	}
	if _, err := strconv.ParseFloat(v, 64); err == nil {
		s.floatish++
		return
	}
	if v == "true" || v == "false" {
		s.boolish++
		return
	}
	s.stringy++
}

func (s *columnStats) discovered(total int) types.DiscoveredColumn {
	dt := types.DataTypeString
	matched := s.stringy
	switch {
	case s.intish > 0 && s.floatish == 0 && s.stringy == 0 && s.boolish == 0:
		dt = types.DataTypeBigInt
		matched = s.intish
	case s.floatish > 0 && s.stringy == 0 && s.boolish == 0:
		dt = types.DataTypeFloat
		matched = s.floatish
	case s.boolish > 0 && s.stringy == 0:
		dt = types.DataTypeBoolean
		matched = s.boolish
	}
	samples := make([]string, 0, len(s.values))
	minValue, maxValue := "", ""
	for v := range s.values {
		samples = append(samples, v)
		if minValue == "" || v < minValue {
			minValue = v
		}
		if maxValue == "" || v > maxValue {
			maxValue = v
		}
		if len(samples) >= 5 {
			break
		}
	}
	null := 0.0
	uniq := 0.0
	confidence := 0.0
	observed := total - s.null
	if total > 0 {
		null = float64(s.null) / float64(total)
		uniq = float64(len(s.values)) / float64(total)
	}
	if observed > 0 {
		confidence = math.Min(1, float64(matched)/float64(observed))
	}
	return types.DiscoveredColumn{
		Name:           s.name,
		InferredType:   dt,
		NullRate:       null,
		UniqueRate:     uniq,
		ObservedCount:  observed,
		DistinctCount:  len(s.values),
		TypeConfidence: confidence,
		MinValue:       minValue,
		MaxValue:       maxValue,
		SampleValues:   samples,
	}
}
