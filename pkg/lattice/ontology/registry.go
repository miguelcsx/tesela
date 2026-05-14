// Registry stitches the metadata store, the validator, the diff engine, and
// the in-memory cache together. It is the only entry point the rest of the
// runtime uses.

package ontology

import (
	"context"
	"fmt"
	"time"

	"github.com/jackc/pgx/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/crypto"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Registry is the canonical surface for ontology operations.
type Registry struct {
	store  *storage.Store
	cache  *Cache
	sealer crypto.Sealer
	now    func() time.Time
}

// NewRegistry constructs a Registry. Sealer may be nil when no datasource has
// credentials; in that case Apply rejects datasources with a credentials
// section.
func NewRegistry(s *storage.Store, sealer crypto.Sealer) *Registry {
	return &Registry{store: s, cache: NewCache(), sealer: sealer, now: time.Now}
}

// Cache returns the internal cache (used by listeners that need Subscribe).
func (r *Registry) Cache() *Cache { return r.cache }

// Snapshot returns the current ontology for ws, hydrating the cache from the
// metadata store on a miss.
func (r *Registry) Snapshot(ctx context.Context, ws types.WorkspaceID) (*types.Ontology, error) {
	if snap := r.cache.Load(ws); snap != nil {
		return snap, nil
	}
	return r.reload(ctx, ws)
}

// Reload forces the cache to refresh from the metadata store. Useful after
// out-of-band changes (e.g., direct DB writes during migration).
func (r *Registry) Reload(ctx context.Context, ws types.WorkspaceID) (*types.Ontology, error) {
	return r.reload(ctx, ws)
}

func (r *Registry) reload(ctx context.Context, ws types.WorkspaceID) (*types.Ontology, error) {
	snap, err := r.loadFromStore(ctx, ws)
	if err != nil {
		return nil, err
	}
	r.cache.Store(ws, snap, types.Change{
		WorkspaceID: ws, NewVersion: snap.Version, OccurredAt: r.now().UTC(),
	})
	return snap, nil
}

func (r *Registry) loadFromStore(ctx context.Context, ws types.WorkspaceID) (*types.Ontology, error) {
	wsRow, err := r.store.Workspaces().GetByID(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("load workspace: %w", err)
	}
	dss, err := r.store.Datasources().List(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("list datasources: %w", err)
	}
	ots, err := r.store.ObjectTypes().List(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("list object_types: %w", err)
	}
	lts, err := r.store.LinkTypes().List(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("list link_types: %w", err)
	}
	ats, err := r.store.ActionTypes().List(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("list action_types: %w", err)
	}
	roles, err := r.store.Roles().List(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("list roles: %w", err)
	}
	prs, err := r.store.PolicyRules().List(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("list policy_rules: %w", err)
	}
	cts, err := r.store.CustomTools().List(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("list custom_tools: %w", err)
	}
	agents, err := r.store.Agents().List(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("list agents: %w", err)
	}
	assets, err := r.store.Assets().List(ctx, ws)
	if err != nil {
		return nil, fmt.Errorf("list assets: %w", err)
	}
	version := maxVersion(ots, lts, ats)
	return &types.Ontology{
		Workspace:   wsRow,
		Version:     version,
		GeneratedAt: r.now().UTC(),
		Datasources: dss,
		ObjectTypes: ots,
		LinkTypes:   lts,
		ActionTypes: ats,
		Roles:       roles,
		PolicyRules: prs,
		CustomTools: cts,
		Agents:      agents,
		Assets:      assets,
	}, nil
}

// Apply parses, validates, persists, and installs a new ontology snapshot for
// the workspace named in the declarative document. Returns the diff
// against the previous snapshot.
func (r *Registry) Apply(ctx context.Context, raw []byte) (types.Diff, error) {
	doc, err := ParseDocument(raw)
	if err != nil {
		return types.Diff{}, err
	}
	wsRow, err := r.ensureWorkspace(ctx, doc.Workspace)
	if err != nil {
		return types.Diff{}, err
	}
	mat, err := doc.Materialize(wsRow.ID)
	if err != nil {
		return types.Diff{}, err
	}
	mat.Workspace = wsRow
	if err := r.sealCredentials(&mat, doc); err != nil {
		return types.Diff{}, err
	}
	if err := Validate(mat); err != nil {
		return types.Diff{}, err
	}
	old, err := r.snapshotOrEmpty(ctx, wsRow.ID)
	if err != nil {
		return types.Diff{}, err
	}
	diff := DiffMaterialized(asMaterialized(old), mat)
	if err := r.persist(ctx, mat, diff); err != nil {
		return types.Diff{}, err
	}
	snap, err := r.reload(ctx, wsRow.ID)
	if err != nil {
		return types.Diff{}, err
	}
	r.cache.Store(wsRow.ID, snap, types.Change{
		WorkspaceID: wsRow.ID, NewVersion: snap.Version, OccurredAt: r.now().UTC(), Diff: diff,
	})
	return diff, nil
}

// ensureWorkspace upserts the workspace row described by the document.
func (r *Registry) ensureWorkspace(ctx context.Context, w workspaceDoc) (types.Workspace, error) {
	existing, err := r.store.Workspaces().GetByAPIName(ctx, types.APIName(w.APIName))
	if err == nil {
		existing.DisplayName = w.DisplayName
		existing.Description = w.Description
		existing.Settings = w.Settings
		return r.store.Workspaces().Update(ctx, existing)
	}
	if !storage.IsNotFound(err) {
		return types.Workspace{}, fmt.Errorf("lookup workspace: %w", err)
	}
	return r.store.Workspaces().Create(ctx, types.Workspace{
		ID:          types.WorkspaceID(ids.NewULID()),
		APIName:     types.APIName(w.APIName),
		DisplayName: w.DisplayName,
		Description: w.Description,
		Settings:    w.Settings,
	})
}

func (r *Registry) sealCredentials(m *Materialized, doc *Document) error {
	if len(doc.Datasources) == 0 {
		return nil
	}
	for i, ds := range doc.Datasources {
		if len(ds.Credentials) == 0 {
			continue
		}
		if r.sealer == nil {
			return fmt.Errorf("datasource %q has credentials but no sealer is configured", ds.APIName)
		}
		raw, err := encodeJSON(ds.Credentials)
		if err != nil {
			return fmt.Errorf("datasource %q: encode credentials: %w", ds.APIName, err)
		}
		blob, err := r.sealer.Seal(raw)
		if err != nil {
			return fmt.Errorf("datasource %q: seal: %w", ds.APIName, err)
		}
		m.Datasources[i].SealedCredentials = blob
	}
	return nil
}

func (r *Registry) snapshotOrEmpty(ctx context.Context, ws types.WorkspaceID) (*types.Ontology, error) {
	snap, err := r.Snapshot(ctx, ws)
	if err == nil {
		return snap, nil
	}
	if storage.IsNotFound(err) {
		return &types.Ontology{Workspace: types.Workspace{ID: ws}}, nil
	}
	return nil, err
}

// persist inserts/updates every entity. Each upsert is assigned a fresh ID
// when the row is new; existing rows keep their ID.
func (r *Registry) persist(ctx context.Context, mat Materialized, diff types.Diff) error {
	return r.store.WithTx(ctx, func(ctx context.Context, tx pgx.Tx) error {
		s := r.store
		for _, ds := range mat.Datasources {
			if ds.ID == "" {
				ds.ID = types.DatasourceID(ids.NewULID())
			}
			if _, err := s.Datasources().Upsert(ctx, ds); err != nil {
				return fmt.Errorf("upsert datasource %s: %w", ds.APIName, err)
			}
		}
		for _, ot := range mat.ObjectTypes {
			if ot.ID == "" {
				ot.ID = types.ObjectTypeID(ids.NewULID())
			}
			if _, err := s.ObjectTypes().Upsert(ctx, ot); err != nil {
				return fmt.Errorf("upsert object_type %s: %w", ot.APIName, err)
			}
		}
		for _, lt := range mat.LinkTypes {
			if lt.ID == "" {
				lt.ID = types.LinkTypeID(ids.NewULID())
			}
			if _, err := s.LinkTypes().Upsert(ctx, lt); err != nil {
				return fmt.Errorf("upsert link_type %s: %w", lt.APIName, err)
			}
		}
		for _, at := range mat.ActionTypes {
			if at.ID == "" {
				at.ID = types.ActionTypeID(ids.NewULID())
			}
			if _, err := s.ActionTypes().Upsert(ctx, at); err != nil {
				return fmt.Errorf("upsert action_type %s: %w", at.APIName, err)
			}
		}
		for _, role := range mat.Roles {
			if role.ID == "" {
				role.ID = types.RoleID(ids.NewULID())
			}
			if _, err := s.Roles().Upsert(ctx, role); err != nil {
				return fmt.Errorf("upsert role %s: %w", role.APIName, err)
			}
		}
		for _, pr := range mat.PolicyRules {
			if pr.ID == "" {
				pr.ID = types.PolicyRuleID(ids.NewULID())
			}
			if _, err := s.PolicyRules().Upsert(ctx, pr); err != nil {
				return fmt.Errorf("upsert policy_rule %s: %w", pr.APIName, err)
			}
		}
		for _, ct := range mat.CustomTools {
			if ct.ID == "" {
				ct.ID = types.CustomToolID(ids.NewULID())
			}
			if _, err := s.CustomTools().Upsert(ctx, ct); err != nil {
				return fmt.Errorf("upsert custom_tool %s: %w", ct.APIName, err)
			}
		}
		for _, a := range mat.Agents {
			if a.ID == "" {
				a.ID = types.AgentID(ids.NewULID())
			}
			if _, err := s.Agents().Upsert(ctx, a); err != nil {
				return fmt.Errorf("upsert agent %s: %w", a.APIName, err)
			}
		}
		for _, as := range mat.Assets {
			if as.ID == "" {
				as.ID = types.AssetID(ids.NewULID())
			}
			if _, err := s.Assets().Upsert(ctx, as); err != nil {
				return fmt.Errorf("upsert asset %s: %w", as.APIName, err)
			}
		}
		for _, e := range diff.Deleted {
			if err := r.deleteEntity(ctx, mat.Workspace.ID, e); err != nil {
				return err
			}
		}
		return nil
	})
}

func (r *Registry) deleteEntity(ctx context.Context, ws types.WorkspaceID, e types.DiffEntry) error {
	switch e.Kind {
	case types.KindDatasource:
		return r.store.Datasources().Delete(ctx, ws, e.APIName)
	case types.KindObjectType:
		return r.store.ObjectTypes().Delete(ctx, ws, e.APIName)
	case types.KindLinkType:
		return r.store.LinkTypes().Delete(ctx, ws, e.APIName)
	case types.KindActionType:
		return r.store.ActionTypes().Delete(ctx, ws, e.APIName)
	case types.KindRole:
		return r.store.Roles().Delete(ctx, ws, e.APIName)
	case types.KindPolicyRule:
		return r.store.PolicyRules().Delete(ctx, ws, e.APIName)
	case types.KindCustomTool:
		return r.store.CustomTools().Delete(ctx, ws, e.APIName)
	case types.KindAgent:
		return r.store.Agents().Delete(ctx, ws, e.APIName)
	case types.KindAsset:
		return r.store.Assets().Delete(ctx, ws, e.APIName)
	default:
		return fmt.Errorf("unknown kind %s", e.Kind)
	}
}

// ExportDocument renders the current snapshot to the declarative document
// format.
func (r *Registry) ExportDocument(ctx context.Context, ws types.WorkspaceID) ([]byte, error) {
	snap, err := r.Snapshot(ctx, ws)
	if err != nil {
		return nil, err
	}
	return SerializeDocument(snap)
}

// Diff compares two named published versions. Used by the API endpoint.
func (r *Registry) Diff(ctx context.Context, ws types.WorkspaceID, fromName, toName string) (types.Diff, error) {
	from, err := r.store.OntologyVersions().GetByName(ctx, ws, fromName)
	if err != nil {
		return types.Diff{}, fmt.Errorf("from version %q: %w", fromName, err)
	}
	to, err := r.store.OntologyVersions().GetByName(ctx, ws, toName)
	if err != nil {
		return types.Diff{}, fmt.Errorf("to version %q: %w", toName, err)
	}
	return DiffMaterialized(asMaterialized(&from.Snapshot), asMaterialized(&to.Snapshot)), nil
}

// Publish freezes the current snapshot under name. Idempotent: returns
// storage.ErrConflict if the name exists.
func (r *Registry) Publish(ctx context.Context, ws types.WorkspaceID, name, createdBy, notes string) (types.OntologyVersion, error) {
	snap, err := r.Snapshot(ctx, ws)
	if err != nil {
		return types.OntologyVersion{}, err
	}
	v := types.OntologyVersion{
		ID:          ids.NewULID(),
		WorkspaceID: ws,
		Name:        name,
		Snapshot:    *snap,
		CreatedBy:   createdBy,
		CreatedAt:   r.now().UTC(),
		Notes:       notes,
	}
	return r.store.OntologyVersions().Create(ctx, v)
}

func asMaterialized(o *types.Ontology) Materialized {
	return Materialized{
		Workspace:   o.Workspace,
		Datasources: o.Datasources,
		ObjectTypes: o.ObjectTypes,
		LinkTypes:   o.LinkTypes,
		ActionTypes: o.ActionTypes,
		Roles:       o.Roles,
		PolicyRules: o.PolicyRules,
		CustomTools: o.CustomTools,
		Agents:      o.Agents,
		Assets:      o.Assets,
	}
}

func maxVersion(ots []types.ObjectType, lts []types.LinkType, ats []types.ActionType) int {
	max := 0
	for _, o := range ots {
		if o.Version > max {
			max = o.Version
		}
	}
	for _, l := range lts {
		if l.Version > max {
			max = l.Version
		}
	}
	for _, a := range ats {
		if a.Version > max {
			max = a.Version
		}
	}
	return max
}
