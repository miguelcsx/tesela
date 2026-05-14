// Postgres-polling consumer. The poller scans the action_runs table for
// rows still in status='pending' and dispatches them through the action
// pipeline. It is deliberately minimal — Phase 6 ports this to River.

package worker

import (
	"context"
	"encoding/json"
	"errors"
	"log/slog"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/actions"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Config controls the poller.
type Config struct {
	Store    *storage.Store
	Pipeline *actions.Pipeline
	Logger   *slog.Logger
	Interval time.Duration
	Batch    int
}

// Poller is the async action runner.
type Poller struct{ cfg Config }

// NewPoller constructs a Poller.
func NewPoller(cfg Config) *Poller {
	if cfg.Interval <= 0 {
		cfg.Interval = 2 * time.Second
	}
	if cfg.Batch <= 0 {
		cfg.Batch = 16
	}
	if cfg.Logger == nil {
		cfg.Logger = slog.Default()
	}
	return &Poller{cfg: cfg}
}

// Run blocks until ctx is cancelled, polling and dispatching pending runs.
func (p *Poller) Run(ctx context.Context) error {
	t := time.NewTicker(p.cfg.Interval)
	defer t.Stop()
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-t.C:
			if err := p.tick(ctx); err != nil && !errors.Is(err, context.Canceled) {
				p.cfg.Logger.Warn("poller tick", "err", err)
			}
		}
	}
}

func (p *Poller) tick(ctx context.Context) error {
	pending, err := p.cfg.Store.ActionRuns().ListPending(ctx, p.cfg.Batch)
	if err != nil {
		return err
	}
	for _, run := range pending {
		p.dispatch(ctx, run)
	}
	return nil
}

func (p *Poller) dispatch(ctx context.Context, run types.ActionRun) {
	var input map[string]any
	if len(run.Input) > 0 {
		_ = json.Unmarshal(run.Input, &input)
	}
	_, err := p.cfg.Pipeline.Execute(ctx, actions.ExecuteRequest{
		Actor:          types.Actor{UserID: run.ActorUserID, WorkspaceID: string(run.WorkspaceID), Roles: run.ActorRoles},
		WorkspaceID:    run.WorkspaceID,
		ActionTypeName: run.ActionType,
		Input:          input,
		IdempotencyKey: run.IdempotencyKey,
		SubjectKey:     run.Subject,
	})
	if err != nil {
		p.cfg.Logger.Error("action dispatch failed", "run_id", run.ID, "err", err)
	}
}
