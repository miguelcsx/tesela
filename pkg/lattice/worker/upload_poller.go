// UploadPoller runs the async upload pipeline: discovery, AI mapping,
// validation, loading, and committing.

package worker

import (
	"context"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"strings"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/objectstore"
	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
	"github.com/miguelcsx/lattice/pkg/lattice/upload"
)

// UploadPollerConfig bundles the poller dependencies.
type UploadPollerConfig struct {
	Store                *storage.Store
	Ontology             *ontology.Registry
	ObjectStore          objectstore.Store
	Bucket               string
	Adapters             *backend.Registry
	MappingEngine        *upload.MappingEngine
	Logger               *slog.Logger
	Interval             time.Duration
	Batch                int
	ConfidenceThreshold  float64
}

// UploadPoller is the async upload runner.
type UploadPoller struct{ cfg UploadPollerConfig }

// NewUploadPoller constructs an UploadPoller.
func NewUploadPoller(cfg UploadPollerConfig) *UploadPoller {
	if cfg.Interval <= 0 {
		cfg.Interval = 5 * time.Second
	}
	if cfg.Batch <= 0 {
		cfg.Batch = 8
	}
	if cfg.Logger == nil {
		cfg.Logger = slog.Default()
	}
	if cfg.ConfidenceThreshold <= 0 {
		cfg.ConfidenceThreshold = 0.85
	}
	return &UploadPoller{cfg: cfg}
}

// Run blocks until ctx is cancelled, polling and driving upload transitions.
func (p *UploadPoller) Run(ctx context.Context) error {
	t := time.NewTicker(p.cfg.Interval)
	defer t.Stop()
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-t.C:
			if err := p.tick(ctx); err != nil && !errors.Is(err, context.Canceled) {
				p.cfg.Logger.Warn("upload poller tick", "err", err)
			}
		}
	}
}

func (p *UploadPoller) tick(ctx context.Context) error {
	activeStatuses := []types.UploadStatus{
		types.UploadStatusInitiated,
		types.UploadStatusPending,
		types.UploadStatusUploaded,
		types.UploadStatusReadyForMapping,
		types.UploadStatusMappingConfirmed,
		types.UploadStatusValidating,
		types.UploadStatusLoading,
		types.UploadStatusValidatingPost,
		types.UploadStatusCommitting,
	}
	for _, status := range activeStatuses {
		uploads, err := p.cfg.Store.Uploads().ListByStatus(ctx, status, p.cfg.Batch)
		if err != nil {
			return err
		}
		for _, u := range uploads {
			if err := p.handle(ctx, u); err != nil {
				p.cfg.Logger.Error("upload handle failed", "upload_id", u.ID, "status", u.Status, "err", err)
				_ = p.fail(ctx, u, err)
			}
		}
	}
	return nil
}

func (p *UploadPoller) handle(ctx context.Context, u types.Upload) error {
	switch u.Status {
	case types.UploadStatusInitiated:
		return p.handleInitiated(ctx, u)
	case types.UploadStatusPending:
		return p.handlePending(ctx, u)
	case types.UploadStatusUploaded:
		return p.handleUploaded(ctx, u)
	case types.UploadStatusReadyForMapping:
		return p.handleReadyForMapping(ctx, u)
	case types.UploadStatusMappingConfirmed:
		return p.transition(ctx, u, types.UploadStatusValidating)
	case types.UploadStatusValidating:
		return p.handleValidating(ctx, u)
	case types.UploadStatusLoading:
		return p.handleLoading(ctx, u)
	case types.UploadStatusValidatingPost:
		return p.handleValidatingPost(ctx, u)
	case types.UploadStatusCommitting:
		return p.handleCommitting(ctx, u)
	default:
		return nil
	}
}

func (p *UploadPoller) handleInitiated(ctx context.Context, u types.Upload) error {
	// The manager already transitions Initiated → Pending during Initiate,
	// but if something got stuck, advance it.
	return p.transition(ctx, u, types.UploadStatusPending)
}

func (p *UploadPoller) handlePending(ctx context.Context, u types.Upload) error {
	// Pending uploads are waiting for the client to finish uploading.
	// Nothing to do here until NotifyUploaded is called.
	return nil
}

func (p *UploadPoller) handleUploaded(ctx context.Context, u types.Upload) error {
	body, err := p.readObject(ctx, u.StorageURL)
	if err != nil {
		return fmt.Errorf("read object: %w", err)
	}
	defer body.Close()
	ds, err := upload.Detect(ctx, "csv", body)
	if err != nil {
		return fmt.Errorf("detect schema: %w", err)
	}
	u.DiscoveredSchema = ds
	u.Status = types.UploadStatusDiscovering
	u, err = p.cfg.Store.Uploads().Update(ctx, u)
	if err != nil {
		return err
	}
	return p.transition(ctx, u, types.UploadStatusReadyForMapping)
}

func (p *UploadPoller) handleReadyForMapping(ctx context.Context, u types.Upload) error {
	if u.ProposedColumnMapping != nil {
		return nil // Already proposed; wait for human approval or auto-approval
	}
	mgr := upload.New(p.buildManagerConfig())
	u, err := mgr.ProposeMapping(ctx, u.WorkspaceID, u.ID)
	if err != nil {
		return fmt.Errorf("propose mapping: %w", err)
	}
	_, wasApproved, err := mgr.AutoApproveIfThreshold(ctx, u, p.cfg.ConfidenceThreshold)
	if err != nil {
		return fmt.Errorf("auto approve: %w", err)
	}
	if wasApproved {
		p.cfg.Logger.Info("upload auto-approved", "upload_id", u.ID, "confidence", u.MappingConfidence)
	}
	return nil
}

func (p *UploadPoller) handleValidating(ctx context.Context, u types.Upload) error {
	snap, err := p.cfg.Ontology.Snapshot(ctx, u.WorkspaceID)
	if err != nil {
		return fmt.Errorf("ontology snapshot: %w", err)
	}
	asset, ok := findAssetInOntology(snap, u.Asset)
	if !ok {
		return errs.Newf(errs.CodeNotFound, "asset %q not found", u.Asset)
	}
	engine := upload.NewValidationEngine()
	result := engine.Validate(u, asset)
	if result.HasErrors() {
		u.ErrorMessage = formatValidationIssues(result.Errors)
		if _, err := p.cfg.Store.Uploads().Update(ctx, u); err != nil {
			return err
		}
		return p.transition(ctx, u, types.UploadStatusFailed)
	}
	return p.transition(ctx, u, types.UploadStatusLoading)
}

func (p *UploadPoller) handleLoading(ctx context.Context, u types.Upload) error {
	snap, err := p.cfg.Ontology.Snapshot(ctx, u.WorkspaceID)
	if err != nil {
		return fmt.Errorf("ontology snapshot: %w", err)
	}
	asset, ok := findAssetInOntology(snap, u.Asset)
	if !ok {
		return errs.Newf(errs.CodeNotFound, "asset %q not found", u.Asset)
	}

	// Resolve datasource
	ds, ok := findDatasourceInOntology(snap, asset.Sink.DatasourceAPIName)
	if !ok {
		return errs.Newf(errs.CodeNotFound, "datasource %q not found", asset.Sink.DatasourceAPIName)
	}

	conn, err := p.cfg.Adapters.Acquire(ctx, ds)
	if err != nil {
		return fmt.Errorf("acquire backend connection: %w", err)
	}

	loader, err := backend.AsBulkLoader(conn)
	if err != nil {
		return fmt.Errorf("backend capability: %w", err)
	}

	var format string
	if u.DiscoveredSchema != nil {
		format = u.DiscoveredSchema.Format
	}
	mapping := upload.ToBackendColumnMapping(u.ColumnMapping)

	src := types.SourceConfig{
		DatasourceAPIName: ds.APIName,
		Schema:            asset.Sink.Schema,
		Table:             asset.Sink.Table,
	}
	result, err := loader.BulkLoad(ctx, src, backend.ObjectStorageRef{
		URL:    u.StorageURL,
		Format: format,
	}, mapping)
	if err != nil {
		return fmt.Errorf("bulk load: %w", err)
	}

	u.Metadata = map[string]any{
		"rows_loaded": result.RowsLoaded,
		"staging_ref": result.StagingRef,
	}
	if _, err := p.cfg.Store.Uploads().Update(ctx, u); err != nil {
		return err
	}
	return p.transition(ctx, u, types.UploadStatusValidatingPost)
}

func (p *UploadPoller) handleValidatingPost(ctx context.Context, u types.Upload) error {
	snap, err := p.cfg.Ontology.Snapshot(ctx, u.WorkspaceID)
	if err != nil {
		return fmt.Errorf("ontology snapshot: %w", err)
	}
	asset, ok := findAssetInOntology(snap, u.Asset)
	if !ok {
		return errs.Newf(errs.CodeNotFound, "asset %q not found", u.Asset)
	}

	var result backend.BulkLoadResult
	if m, ok := u.Metadata["rows_loaded"]; ok {
		switch v := m.(type) {
		case int64:
			result.RowsLoaded = v
		case float64:
			result.RowsLoaded = int64(v)
		}
	}
	if ref, ok := u.Metadata["staging_ref"].(string); ok {
		result.StagingRef = ref
	}

	engine := upload.NewPostValidationEngine()
	vres := engine.Validate(u, asset, result)
	if vres.HasErrors() {
		u.ErrorMessage = formatValidationIssues(vres.Errors)
		if _, err := p.cfg.Store.Uploads().Update(ctx, u); err != nil {
			return err
		}
		_ = p.rollbackUpload(ctx, u, asset)
		return p.transition(ctx, u, types.UploadStatusFailed)
	}
	return p.transition(ctx, u, types.UploadStatusCommitting)
}

func (p *UploadPoller) handleCommitting(ctx context.Context, u types.Upload) error {
	snap, err := p.cfg.Ontology.Snapshot(ctx, u.WorkspaceID)
	if err != nil {
		return fmt.Errorf("ontology snapshot: %w", err)
	}
	asset, ok := findAssetInOntology(snap, u.Asset)
	if !ok {
		return errs.Newf(errs.CodeNotFound, "asset %q not found", u.Asset)
	}

	var rowCount int64
	if m, ok := u.Metadata["rows_loaded"]; ok {
		switch v := m.(type) {
		case int64:
			rowCount = v
		case float64:
			rowCount = int64(v)
		}
	}

	now := time.Now().UTC()
	version := types.AssetVersion{
		ID:        ids.NewULID(),
		WorkspaceID: u.WorkspaceID,
		AssetID:   asset.ID,
		UploadID:  u.ID,
		RowCount:  rowCount,
		Status:    "published",
		Lineage: map[string]any{
			"upload_id":      u.ID,
			"column_mapping": u.ColumnMapping,
			"actor_user_id":  u.ActorUserID,
		},
		Metadata:  map[string]any{},
		Committed: &now,
	}
	if _, err := p.cfg.Store.AssetVersions().Create(ctx, version); err != nil {
		return fmt.Errorf("create asset version: %w", err)
	}
	return p.transition(ctx, u, types.UploadStatusCompleted)
}

func (p *UploadPoller) transition(ctx context.Context, u types.Upload, target types.UploadStatus) error {
	u.Status = target
	_, err := p.cfg.Store.Uploads().Update(ctx, u)
	return err
}

func (p *UploadPoller) fail(ctx context.Context, u types.Upload, err error) error {
	if u.Status.IsTerminal() {
		return nil
	}
	u.Status = types.UploadStatusFailed
	u.ErrorMessage = err.Error()
	if _, err := p.cfg.Store.Uploads().Update(ctx, u); err != nil {
		return err
	}

	// Attempt rollback if we have a staging reference.
	snap, snapErr := p.cfg.Ontology.Snapshot(ctx, u.WorkspaceID)
	if snapErr != nil {
		return snapErr
	}
	asset, ok := findAssetInOntology(snap, u.Asset)
	if !ok {
		return nil
	}
	_ = p.rollbackUpload(ctx, u, asset)
	return nil
}

func (p *UploadPoller) rollbackUpload(ctx context.Context, u types.Upload, asset types.Asset) error {
	if ref, ok := u.Metadata["staging_ref"].(string); !ok || ref == "" {
		return nil
	}
	snap, err := p.cfg.Ontology.Snapshot(ctx, u.WorkspaceID)
	if err != nil {
		return err
	}
	ds, ok := findDatasourceInOntology(snap, asset.Sink.DatasourceAPIName)
	if !ok {
		return nil
	}
	conn, err := p.cfg.Adapters.Acquire(ctx, ds)
	if err != nil {
		return err
	}
	loader, err := backend.AsBulkLoader(conn)
	if err != nil {
		return err
	}
	return loader.RollbackUpload(ctx, types.SourceConfig{
		DatasourceAPIName: ds.APIName,
		Schema:            asset.Sink.Schema,
		Table:             asset.Sink.Table,
	}, string(u.ID))
}

func (p *UploadPoller) readObject(ctx context.Context, url string) (io.ReadCloser, error) {
	if p.cfg.ObjectStore == nil {
		return nil, errs.New(errs.CodeInternal, "object store not configured")
	}
	key := url
	if p.cfg.Bucket != "" && strings.HasPrefix(key, p.cfg.Bucket+"/") {
		key = strings.TrimPrefix(key, p.cfg.Bucket+"/")
	} else if strings.HasPrefix(key, "/") {
		key = strings.TrimPrefix(key, "/")
	}
	rc, _, err := p.cfg.ObjectStore.Get(ctx, key)
	return rc, err
}

func (p *UploadPoller) buildManagerConfig() upload.Config {
	return upload.Config{
		Store:       p.cfg.Store,
		Ontology:    p.cfg.Ontology,
		ObjectStore: p.cfg.ObjectStore,
		Bucket:      p.cfg.Bucket,
		Now:         time.Now,
	}
}

func findAssetInOntology(snap *types.Ontology, name types.APIName) (types.Asset, bool) {
	for _, a := range snap.Assets {
		if a.APIName == name {
			return a, true
		}
	}
	return types.Asset{}, false
}

func findDatasourceInOntology(snap *types.Ontology, name types.APIName) (types.Datasource, bool) {
	for _, d := range snap.Datasources {
		if d.APIName == name {
			return d, true
		}
	}
	return types.Datasource{}, false
}

func formatValidationIssues(issues []upload.ValidationIssue) string {
	var msgs []string
	for _, i := range issues {
		msgs = append(msgs, fmt.Sprintf("[%s] %s", i.RuleAPIName, i.Message))
	}
	return strings.Join(msgs, "; ")
}
