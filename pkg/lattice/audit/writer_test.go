package audit

import (
	"context"
	"errors"
	"sync"
	"testing"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type capturingSink struct {
	mu      sync.Mutex
	batches [][]types.AuditRecord
	err     error
}

func (s *capturingSink) Write(_ context.Context, batch []types.AuditRecord) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.err != nil {
		return s.err
	}
	cp := append([]types.AuditRecord(nil), batch...)
	s.batches = append(s.batches, cp)
	return nil
}

func (s *capturingSink) total() int {
	s.mu.Lock()
	defer s.mu.Unlock()
	n := 0
	for _, b := range s.batches {
		n += len(b)
	}
	return n
}

func TestWriter_FlushOnBatchSize(t *testing.T) {
	sink := &capturingSink{}
	w := NewWriter(sink, Config{BatchSize: 3, FlushInterval: time.Hour, BufferSize: 16})
	defer func() { _ = w.Close(context.Background()) }()

	for i := 0; i < 3; i++ {
		if err := w.Write(context.Background(), types.AuditRecord{ActorUserID: "u"}); err != nil {
			t.Fatalf("write: %v", err)
		}
	}
	deadline := time.Now().Add(2 * time.Second)
	for sink.total() < 3 && time.Now().Before(deadline) {
		time.Sleep(10 * time.Millisecond)
	}
	if sink.total() != 3 {
		t.Fatalf("want 3 written, got %d", sink.total())
	}
}

func TestWriter_FlushOnInterval(t *testing.T) {
	sink := &capturingSink{}
	w := NewWriter(sink, Config{BatchSize: 100, FlushInterval: 30 * time.Millisecond, BufferSize: 16})
	defer func() { _ = w.Close(context.Background()) }()

	if err := w.Write(context.Background(), types.AuditRecord{ActorUserID: "u"}); err != nil {
		t.Fatal(err)
	}
	time.Sleep(120 * time.Millisecond)
	if sink.total() != 1 {
		t.Fatalf("want 1 record after interval, got %d", sink.total())
	}
}

func TestWriter_FlushExplicit(t *testing.T) {
	sink := &capturingSink{}
	w := NewWriter(sink, Config{BatchSize: 100, FlushInterval: time.Hour, BufferSize: 16})
	defer func() { _ = w.Close(context.Background()) }()

	for i := 0; i < 5; i++ {
		_ = w.Write(context.Background(), types.AuditRecord{ActorUserID: "u"})
	}
	if err := w.Flush(context.Background()); err != nil {
		t.Fatalf("flush: %v", err)
	}
	if sink.total() != 5 {
		t.Fatalf("want 5 after flush, got %d", sink.total())
	}
}

func TestWriter_DropsWhenBufferFull(t *testing.T) {
	sink := &capturingSink{}
	w := NewWriter(sink, Config{BatchSize: 100, FlushInterval: time.Hour, BufferSize: 1})
	defer func() { _ = w.Close(context.Background()) }()

	// Fill buffer; first write succeeds, the rest drop because the goroutine
	// is paused on FlushInterval.
	for i := 0; i < 50; i++ {
		_ = w.Write(context.Background(), types.AuditRecord{ActorUserID: "u"})
	}
	// Some drops should have happened.
	if w.Drops() == 0 {
		t.Fatal("expected some drops with tiny buffer")
	}
}

func TestWriter_SinkErrorIsLoggedNotPanicked(t *testing.T) {
	sink := &capturingSink{err: errors.New("boom")}
	w := NewWriter(sink, Config{BatchSize: 1, FlushInterval: time.Hour, BufferSize: 4})
	defer func() { _ = w.Close(context.Background()) }()

	if err := w.Write(context.Background(), types.AuditRecord{ActorUserID: "u"}); err != nil {
		t.Fatal(err)
	}
	// Flush triggers write; should not panic; should not deadlock.
	_ = w.Flush(context.Background())
}

func TestWriter_CloseDrains(t *testing.T) {
	sink := &capturingSink{}
	w := NewWriter(sink, Config{BatchSize: 100, FlushInterval: time.Hour, BufferSize: 16})
	for i := 0; i < 4; i++ {
		_ = w.Write(context.Background(), types.AuditRecord{ActorUserID: "u"})
	}
	if err := w.Close(context.Background()); err != nil {
		t.Fatalf("close: %v", err)
	}
	if sink.total() != 4 {
		t.Fatalf("want 4 drained, got %d", sink.total())
	}
}
