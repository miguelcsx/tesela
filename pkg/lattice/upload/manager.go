// Manager is the upload lifecycle orchestrator. It exposes the API surface
// (initiate, notify, mapping, approve) and drives the per-state transitions.

package upload

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/audit"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/objectstore"
	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/policy"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Config bundles the manager dependencies.
type Config struct {
	Store                *storage.Store
	Ontology             *ontology.Registry
	ObjectStore          objectstore.Store
	Bucket               string
	URLTTL               time.Duration
	Now                  func() time.Time
	MappingEngine        *MappingEngine
	HeuristicMappingEngine *HeuristicMappingEngine
	Policy               *policy.Evaluator
	Audit                *audit.Writer
}

// Manager is the upload orchestrator.
type Manager struct{ cfg Config }

// New constructs a Manager.
func New(cfg Config) *Manager {
	if cfg.URLTTL <= 0 {
		cfg.URLTTL = 6 * time.Hour
	}
	if cfg.Now == nil {
		cfg.Now = time.Now
	}
	if cfg.HeuristicMappingEngine == nil {
		cfg.HeuristicMappingEngine = NewHeuristicMappingEngine()
	}
	return &Manager{cfg: cfg}
}

func (m *Manager) checkPolicy(ctx context.Context, actor types.Actor, op types.Operation, resourceKind types.Kind, resourceName types.APIName) error {
	if m.cfg.Policy == nil {
		return nil
	}
	dec := m.cfg.Policy.Evaluate(policy.Request{
		Actor:        actor,
		Operation:    op,
		ResourceKind: resourceKind,
		ResourceName: resourceName,
	})
	if !dec.Allow {
		return errs.Newf(errs.CodePolicyDenied, "policy denied: %s", dec.Reason)
	}
	return nil
}

func (m *Manager) writeAudit(ctx context.Context, rec types.AuditRecord) {
	if m.cfg.Audit == nil {
		return
	}
	_ = m.cfg.Audit.Write(ctx, rec)
}

// InitiateRequest is the input to Initiate.
type InitiateRequest struct {
	WorkspaceID types.WorkspaceID
	Asset       types.APIName
	Actor       types.Actor
	ContentType string
	MaxBytes    int64
}

// InitiateResult is what the caller returns to the client.
type InitiateResult struct {
	UploadID   types.UploadID `json:"upload_id"`
	SignedURL  string         `json:"signed_url"`
	ExpiresAt  time.Time      `json:"expires_at"`
	StorageKey string         `json:"storage_key"`
}

// Initiate creates a new Upload row and returns a presigned PUT URL.
func (m *Manager) Initiate(ctx context.Context, req InitiateRequest) (InitiateResult, error) {
	if err := m.checkPolicy(ctx, req.Actor, types.OperationUpload, types.KindAsset, req.Asset); err != nil {
		return InitiateResult{}, err
	}
	snap, err := m.cfg.Ontology.Snapshot(ctx, req.WorkspaceID)
	if err != nil {
		return InitiateResult{}, errs.Wrap(err, errs.CodeInternal, "ontology")
	}
	if !assetExists(snap, req.Asset) {
		return InitiateResult{}, errs.Newf(errs.CodeNotFound, "asset %q not found", req.Asset)
	}
	uploadID := types.UploadID(ids.NewULID())
	key := fmt.Sprintf("uploads/%s/%s/%s", req.WorkspaceID, req.Asset, uploadID)
	expires := m.cfg.Now().Add(m.cfg.URLTTL).UTC()
	signedURL, err := m.cfg.ObjectStore.SignedPutURL(ctx, key, objectstore.SignedOptions{
		Expires: m.cfg.URLTTL, MaxBytes: req.MaxBytes, ContentType: req.ContentType,
	})
	if err != nil {
		return InitiateResult{}, errs.Wrap(err, errs.CodeInternal, "signed url")
	}
	storageURL := fmt.Sprintf("%s/%s", m.cfg.Bucket, key)
	rec := types.Upload{
		ID:               uploadID,
		WorkspaceID:      req.WorkspaceID,
		Asset:            req.Asset,
		Status:           types.UploadStatusInitiated,
		StorageURL:       storageURL,
		SignedURL:        signedURL,
		SignedURLExpires: &expires,
		ActorUserID:      req.Actor.UserID,
	}
	if _, err := m.cfg.Store.Uploads().Create(ctx, rec); err != nil {
		return InitiateResult{}, errs.Wrap(err, errs.CodeInternal, "create upload")
	}
	if _, err := m.transition(ctx, req.WorkspaceID, uploadID, types.UploadStatusPending); err != nil {
		return InitiateResult{}, errs.Wrap(err, errs.CodeInternal, "transition to pending")
	}
	m.writeAudit(ctx, types.AuditRecord{
		WorkspaceID:    req.WorkspaceID,
		ActorUserID:    req.Actor.UserID,
		ActorRoles:     req.Actor.Roles,
		Operation:      types.OperationUpload,
		ResourceKind:   string(types.KindAsset),
		ResourceAPIName: req.Asset,
		PolicyDecision: types.AuditDecisionAllow,
		Metadata:       map[string]any{"upload_id": uploadID, "event": "upload_initiated"},
	})
	return InitiateResult{
		UploadID: uploadID, SignedURL: signedURL, ExpiresAt: expires, StorageKey: key,
	}, nil
}

// NotifyUploaded marks the upload as having received its bytes; the next
// stage (discovery) is triggered from the worker.
func (m *Manager) NotifyUploaded(ctx context.Context, ws types.WorkspaceID, id types.UploadID, actor types.Actor) (types.Upload, error) {
	if err := m.checkPolicy(ctx, actor, types.OperationUpload, types.KindAsset, types.APIName("")); err != nil {
		return types.Upload{}, err
	}
	u, err := m.transition(ctx, ws, id, types.UploadStatusUploaded)
	if err != nil {
		return u, err
	}
	m.writeAudit(ctx, types.AuditRecord{
		WorkspaceID:    ws,
		ActorUserID:    actor.UserID,
		ActorRoles:     actor.Roles,
		Operation:      types.OperationUpload,
		ResourceKind:   string(types.KindAsset),
		ResourceAPIName: u.Asset,
		UploadID:       u.ID,
		PolicyDecision: types.AuditDecisionAllow,
		Metadata:       map[string]any{"event": "upload_received"},
	})
	return u, nil
}

// SetMapping persists the column mapping the client confirmed.
func (m *Manager) SetMapping(ctx context.Context, ws types.WorkspaceID, id types.UploadID, actor types.Actor, mapping []types.ColumnMapping) (types.Upload, error) {
	if err := m.checkPolicy(ctx, actor, types.OperationApproveUpload, types.KindAsset, types.APIName("")); err != nil {
		return types.Upload{}, err
	}
	u, err := m.cfg.Store.Uploads().GetByID(ctx, ws, id)
	if err != nil {
		return types.Upload{}, err
	}
	if u.Status != types.UploadStatusReadyForMapping {
		return types.Upload{}, errs.Newf(errs.CodeValidation, "upload status %q does not accept mapping", u.Status)
	}
	snap, err := m.cfg.Ontology.Snapshot(ctx, ws)
	if err != nil {
		return types.Upload{}, errs.Wrap(err, errs.CodeInternal, "ontology")
	}
	asset, ok := findAsset(snap, u.Asset)
	if !ok {
		return types.Upload{}, errs.Newf(errs.CodeNotFound, "asset %q not found", u.Asset)
	}
	engine := NewHeuristicMappingEngine()
	_, blocking := engine.ValidateMapping(mapping, asset, asset.UnmappedColumnPolicy)
	if len(blocking) > 0 {
		return types.Upload{}, errs.Newf(errs.CodeValidation, "mapping validation failed: %v", blocking)
	}
	u.ColumnMapping = mapping
	u.Status = types.UploadStatusMappingConfirmed
	updated, err := m.cfg.Store.Uploads().Update(ctx, u)
	if err != nil {
		return types.Upload{}, err
	}
	m.writeAudit(ctx, types.AuditRecord{
		WorkspaceID:    ws,
		ActorUserID:    actor.UserID,
		ActorRoles:     actor.Roles,
		Operation:      types.OperationApproveUpload,
		ResourceKind:   string(types.KindAsset),
		ResourceAPIName: u.Asset,
		UploadID:       u.ID,
		PolicyDecision: types.AuditDecisionAllow,
		Metadata:       map[string]any{"event": "upload_mapping_approved", "mapping": mapping},
	})
	return updated, nil
}

// Approve advances a mapping_confirmed upload through validation/loading.
// Returns the latest upload row; the caller may poll for completion.
func (m *Manager) Approve(ctx context.Context, ws types.WorkspaceID, id types.UploadID, actor types.Actor) (types.Upload, error) {
	if err := m.checkPolicy(ctx, actor, types.OperationApproveUpload, types.KindAsset, types.APIName("")); err != nil {
		return types.Upload{}, err
	}
	u, err := m.transition(ctx, ws, id, types.UploadStatusValidating)
	if err != nil {
		return u, err
	}
	m.writeAudit(ctx, types.AuditRecord{
		WorkspaceID:    ws,
		ActorUserID:    actor.UserID,
		ActorRoles:     actor.Roles,
		Operation:      types.OperationApproveUpload,
		ResourceKind:   string(types.KindAsset),
		ResourceAPIName: u.Asset,
		UploadID:       u.ID,
		PolicyDecision: types.AuditDecisionAllow,
		Metadata:       map[string]any{"event": "upload_mapping_approved"},
	})
	return u, nil
}

// ProposeMapping uses the mapping engine to generate a column mapping proposal.
func (m *Manager) ProposeMapping(ctx context.Context, ws types.WorkspaceID, id types.UploadID) (types.Upload, error) {
	u, err := m.cfg.Store.Uploads().GetByID(ctx, ws, id)
	if err != nil {
		return types.Upload{}, err
	}
	if u.Status != types.UploadStatusReadyForMapping {
		return types.Upload{}, errs.Newf(errs.CodeValidation, "upload status %q does not accept mapping proposal", u.Status)
	}
	snap, err := m.cfg.Ontology.Snapshot(ctx, ws)
	if err != nil {
		return types.Upload{}, errs.Wrap(err, errs.CodeInternal, "ontology")
	}
	asset, ok := findAsset(snap, u.Asset)
	if !ok {
		return types.Upload{}, errs.Newf(errs.CodeNotFound, "asset %q not found", u.Asset)
	}
	if m.cfg.MappingEngine == nil {
		return types.Upload{}, errs.New(errs.CodeInternal, "mapping engine not configured")
	}
	u, err = m.cfg.MappingEngine.ProposeMapping(ctx, u, asset)
	if err != nil {
		return types.Upload{}, errs.Wrap(err, errs.CodeInternal, "propose mapping")
	}
	return m.cfg.Store.Uploads().Update(ctx, u)
}

// AutoApproveIfThreshold transitions the upload to MappingConfirmed if confidence is high enough.
func (m *Manager) AutoApproveIfThreshold(ctx context.Context, u types.Upload, threshold float64) (types.Upload, bool, error) {
	if u.MappingConfidence >= threshold {
		u, err := m.transition(ctx, u.WorkspaceID, u.ID, types.UploadStatusMappingConfirmed)
		if err != nil {
			return u, false, err
		}
		return u, true, nil
	}
	return u, false, nil
}

// Cancel marks the upload failed.
func (m *Manager) Cancel(ctx context.Context, ws types.WorkspaceID, id types.UploadID, actor types.Actor, reason string) (types.Upload, error) {
	if err := m.checkPolicy(ctx, actor, types.OperationDeleteUpload, types.KindAsset, types.APIName("")); err != nil {
		return types.Upload{}, err
	}
	u, err := m.cfg.Store.Uploads().GetByID(ctx, ws, id)
	if err != nil {
		return types.Upload{}, err
	}
	if u.Status.IsTerminal() {
		return u, nil
	}
	u.Status = types.UploadStatusFailed
	u.ErrorMessage = reason
	updated, err := m.cfg.Store.Uploads().Update(ctx, u)
	if err != nil {
		return types.Upload{}, err
	}
	m.writeAudit(ctx, types.AuditRecord{
		WorkspaceID:    ws,
		ActorUserID:    actor.UserID,
		ActorRoles:     actor.Roles,
		Operation:      types.OperationDeleteUpload,
		ResourceKind:   string(types.KindAsset),
		ResourceAPIName: u.Asset,
		UploadID:       u.ID,
		PolicyDecision: types.AuditDecisionAllow,
		Metadata:       map[string]any{"event": "upload_cancelled", "reason": reason},
	})
	return updated, nil
}

// Commit finalizes a validated upload into a published asset version.
func (m *Manager) Commit(ctx context.Context, ws types.WorkspaceID, id types.UploadID, actor types.Actor) (types.AssetVersion, error) {
	if err := m.checkPolicy(ctx, actor, types.OperationApproveUpload, types.KindAsset, types.APIName("")); err != nil {
		return types.AssetVersion{}, err
	}
	u, err := m.cfg.Store.Uploads().GetByID(ctx, ws, id)
	if err != nil {
		return types.AssetVersion{}, err
	}
	snap, err := m.cfg.Ontology.Snapshot(ctx, ws)
	if err != nil {
		return types.AssetVersion{}, errs.Wrap(err, errs.CodeInternal, "ontology")
	}
	asset, ok := findAsset(snap, u.Asset)
	if !ok {
		return types.AssetVersion{}, errs.Newf(errs.CodeNotFound, "asset %q not found", u.Asset)
	}
	var rowCount int64
	if v, ok := u.Metadata["rows_loaded"]; ok {
		switch val := v.(type) {
		case int64:
			rowCount = val
		case float64:
			rowCount = int64(val)
		}
	}
	now := time.Now().UTC()
	version := types.AssetVersion{
		ID:          ids.NewULID(),
		WorkspaceID: ws,
		AssetID:     asset.ID,
		UploadID:    u.ID,
		RowCount:    rowCount,
		Status:      "published",
		Lineage: map[string]any{
			"upload_id":      u.ID,
			"column_mapping": u.ColumnMapping,
			"actor_user_id":  u.ActorUserID,
		},
		Metadata:  map[string]any{},
		Committed: &now,
	}
	v, err := m.cfg.Store.AssetVersions().Create(ctx, version)
	if err != nil {
		return types.AssetVersion{}, fmt.Errorf("create asset version: %w", err)
	}
	m.writeAudit(ctx, types.AuditRecord{
		WorkspaceID:    ws,
		ActorUserID:    actor.UserID,
		ActorRoles:     actor.Roles,
		Operation:      types.OperationApproveUpload,
		ResourceKind:   string(types.KindAsset),
		ResourceAPIName: u.Asset,
		UploadID:       u.ID,
		PolicyDecision: types.AuditDecisionAllow,
		Metadata:       map[string]any{"event": "upload_committed", "version_id": v.ID},
	})
	return v, nil
}

// Rollback invalidates an upload and attempts to clean up staged data.
func (m *Manager) Rollback(ctx context.Context, ws types.WorkspaceID, id types.UploadID, actor types.Actor, reason string) error {
	if err := m.checkPolicy(ctx, actor, types.OperationDeleteUpload, types.KindAsset, types.APIName("")); err != nil {
		return err
	}
	u, err := m.cfg.Store.Uploads().GetByID(ctx, ws, id)
	if err != nil {
		return err
	}
	if u.Status.IsTerminal() {
		return nil
	}
	u.Status = types.UploadStatusFailed
	u.ErrorMessage = reason
	if _, err := m.cfg.Store.Uploads().Update(ctx, u); err != nil {
		return err
	}
	m.writeAudit(ctx, types.AuditRecord{
		WorkspaceID:    ws,
		ActorUserID:    actor.UserID,
		ActorRoles:     actor.Roles,
		Operation:      types.OperationDeleteUpload,
		ResourceKind:   string(types.KindAsset),
		ResourceAPIName: u.Asset,
		UploadID:       u.ID,
		PolicyDecision: types.AuditDecisionAllow,
		Metadata:       map[string]any{"event": "upload_rolled_back", "reason": reason},
	})
	return nil
}

// transition is a small helper that loads, validates the new state, and saves.
func (m *Manager) transition(ctx context.Context, ws types.WorkspaceID, id types.UploadID, target types.UploadStatus) (types.Upload, error) {
	u, err := m.cfg.Store.Uploads().GetByID(ctx, ws, id)
	if err != nil {
		return types.Upload{}, err
	}
	if !canTransition(u.Status, target) {
		return types.Upload{}, errs.Newf(errs.CodeValidation,
			"upload %s: cannot transition %s → %s", id, u.Status, target)
	}
	u.Status = target
	updated, err := m.cfg.Store.Uploads().Update(ctx, u)
	if err != nil {
		return types.Upload{}, err
	}
	return updated, nil
}

// canTransition encodes the legal state machine. Anything not listed is
// rejected.
var transitions = map[types.UploadStatus]map[types.UploadStatus]struct{}{
	types.UploadStatusInitiated:        set(types.UploadStatusPending, types.UploadStatusFailed),
	types.UploadStatusPending:          set(types.UploadStatusUploaded, types.UploadStatusFailed),
	types.UploadStatusUploaded:         set(types.UploadStatusDiscovering, types.UploadStatusFailed),
	types.UploadStatusDiscovering:      set(types.UploadStatusReadyForMapping, types.UploadStatusFailed),
	types.UploadStatusReadyForMapping:  set(types.UploadStatusMappingConfirmed, types.UploadStatusFailed),
	types.UploadStatusMappingConfirmed: set(types.UploadStatusValidating, types.UploadStatusFailed),
	types.UploadStatusValidating:       set(types.UploadStatusLoading, types.UploadStatusFailed),
	types.UploadStatusLoading:          set(types.UploadStatusValidatingPost, types.UploadStatusFailed),
	types.UploadStatusValidatingPost:   set(types.UploadStatusCommitting, types.UploadStatusFailed),
	types.UploadStatusCommitting:       set(types.UploadStatusCompleted, types.UploadStatusFailed),
}

func canTransition(from, to types.UploadStatus) bool {
	allowed, ok := transitions[from]
	if !ok {
		return false
	}
	_, ok = allowed[to]
	return ok
}

func set(items ...types.UploadStatus) map[types.UploadStatus]struct{} {
	out := make(map[types.UploadStatus]struct{}, len(items))
	for _, i := range items {
		out[i] = struct{}{}
	}
	return out
}

func assetExists(snap *types.Ontology, name types.APIName) bool {
	for _, a := range snap.Assets {
		if a.APIName == name {
			return true
		}
	}
	return false
}

func findAsset(snap *types.Ontology, name types.APIName) (types.Asset, bool) {
	for _, a := range snap.Assets {
		if a.APIName == name {
			return a, true
		}
	}
	return types.Asset{}, false
}

// errMissingAsset is exported only to reduce noise in tests.
var errMissingAsset = errors.New("upload: asset missing in ontology")
