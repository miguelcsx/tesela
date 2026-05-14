// Writer is the buffered, append-only audit log front-end.

package audit

import (
	"context"
	"errors"
	"log/slog"
	"sync"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Sink persists batches of audit records. The default sink wraps the metadata
// store; alternative sinks (Kafka, file, dual-write) can be composed.
type Sink interface {
	Write(ctx context.Context, batch []types.AuditRecord) error
}

// Config controls Writer behavior.
type Config struct {
	BatchSize     int
	FlushInterval time.Duration
	BufferSize    int
	Logger        *slog.Logger
}

// Writer is the buffered audit log front-end.
type Writer struct {
	cfg     Config
	sink    Sink
	now     func() time.Time
	queue   chan types.AuditRecord
	flush   chan chan struct{}
	stop    chan struct{}
	stopped chan struct{}
	once    sync.Once
	logger  *slog.Logger

	dropsMu sync.Mutex
	drops   int64
}

// NewWriter constructs a writer and starts the background flusher. Caller
// must call Close to drain.
func NewWriter(sink Sink, cfg Config) *Writer {
	if cfg.BatchSize <= 0 {
		cfg.BatchSize = 100
	}
	if cfg.FlushInterval <= 0 {
		cfg.FlushInterval = time.Second
	}
	if cfg.BufferSize <= 0 {
		cfg.BufferSize = 1024
	}
	if cfg.Logger == nil {
		cfg.Logger = slog.Default()
	}
	w := &Writer{
		cfg:     cfg,
		sink:    sink,
		now:     time.Now,
		queue:   make(chan types.AuditRecord, cfg.BufferSize),
		flush:   make(chan chan struct{}, 1),
		stop:    make(chan struct{}),
		stopped: make(chan struct{}),
		logger:  cfg.Logger,
	}
	go w.run()
	return w
}

// Write enqueues a record. Non-blocking: if the buffer is full, the record is
// dropped and the drops counter is incremented. Returns nil except for the
// degenerate case of a closed writer.
func (w *Writer) Write(_ context.Context, rec types.AuditRecord) error {
	if rec.ID == "" {
		rec.ID = ids.NewULID()
	}
	if rec.OccurredAt.IsZero() {
		rec.OccurredAt = w.now().UTC()
	}
	select {
	case <-w.stop:
		return errors.New("audit writer is closed")
	default:
	}
	select {
	case w.queue <- rec:
		return nil
	default:
		w.dropsMu.Lock()
		w.drops++
		w.dropsMu.Unlock()
		w.logger.Warn("audit buffer full; record dropped",
			"workspace_id", rec.WorkspaceID,
			"operation", rec.Operation,
			"actor_user_id", rec.ActorUserID,
		)
		return nil
	}
}

// Drops returns the cumulative number of dropped audit records.
func (w *Writer) Drops() int64 {
	w.dropsMu.Lock()
	defer w.dropsMu.Unlock()
	return w.drops
}

// Flush forces an immediate batch write. Blocks until done.
func (w *Writer) Flush(ctx context.Context) error {
	done := make(chan struct{})
	select {
	case w.flush <- done:
	case <-ctx.Done():
		return ctx.Err()
	case <-w.stop:
		return nil
	}
	select {
	case <-done:
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

// Close drains the queue, flushes, and stops the background goroutine. Safe
// to call multiple times.
func (w *Writer) Close(ctx context.Context) error {
	w.once.Do(func() { close(w.stop) })
	select {
	case <-w.stopped:
	case <-ctx.Done():
		return ctx.Err()
	}
	return nil
}

func (w *Writer) run() {
	defer close(w.stopped)
	ticker := time.NewTicker(w.cfg.FlushInterval)
	defer ticker.Stop()

	batch := make([]types.AuditRecord, 0, w.cfg.BatchSize)
	flushNow := func(reason string) {
		if len(batch) == 0 {
			return
		}
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		if err := w.sink.Write(ctx, batch); err != nil {
			w.logger.Error("audit sink write failed",
				"reason", reason,
				"batch_size", len(batch),
				"err", err,
			)
		}
		cancel()
		batch = batch[:0]
	}

	for {
		select {
		case <-w.stop:
			w.drainQueue(&batch)
			flushNow("close")
			return
		case rec := <-w.queue:
			batch = append(batch, rec)
			if len(batch) >= w.cfg.BatchSize {
				flushNow("batch_full")
			}
		case <-ticker.C:
			flushNow("interval")
		case done := <-w.flush:
			w.drainQueue(&batch)
			flushNow("flush_request")
			close(done)
		}
	}
}

func (w *Writer) drainQueue(batch *[]types.AuditRecord) {
	for {
		select {
		case rec := <-w.queue:
			*batch = append(*batch, rec)
		default:
			return
		}
	}
}
