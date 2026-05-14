// Upload handlers — initiate, notify, set mapping, approve, cancel, list.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
	"github.com/miguelcsx/lattice/pkg/lattice/upload"
)

type uploadHandlers struct {
	store   *storage.Store
	manager *upload.Manager
}

func newUploadHandlers(cfg ServerConfig) *uploadHandlers {
	return &uploadHandlers{store: cfg.Store, manager: cfg.UploadManager}
}

type uploadCreateReq struct {
	ContentType string `json:"content_type,omitempty"`
	MaxBytes    int64  `json:"max_bytes,omitempty"`
}

func (h *uploadHandlers) Create(w http.ResponseWriter, r *http.Request) {
	if h.manager == nil {
		writeError(w, r, errs.New(errs.CodeInternal, "upload manager not configured"))
		return
	}
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var body uploadCreateReq
	if r.ContentLength > 0 {
		if err := decodeJSON(r, &body); err != nil {
			writeError(w, r, err)
			return
		}
	}
	res, err := h.manager.Initiate(r.Context(), upload.InitiateRequest{
		WorkspaceID: ws.ID,
		Asset:       types.APIName(chi.URLParam(r, "asset")),
		Actor:       actor,
		ContentType: body.ContentType,
		MaxBytes:    body.MaxBytes,
	})
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusCreated, res)
}

func (h *uploadHandlers) Notify(w http.ResponseWriter, r *http.Request) {
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	id := types.UploadID(chi.URLParam(r, "upload_id"))
	out, err := h.manager.NotifyUploaded(r.Context(), ws.ID, id, actor)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, out)
}

type mappingReq struct {
	Mapping []types.ColumnMapping `json:"mapping"`
}

func (h *uploadHandlers) SetMapping(w http.ResponseWriter, r *http.Request) {
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var body mappingReq
	if err := decodeJSON(r, &body); err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.manager.SetMapping(r.Context(), ws.ID, types.UploadID(chi.URLParam(r, "upload_id")), actor, body.Mapping)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, out)
}

func (h *uploadHandlers) Approve(w http.ResponseWriter, r *http.Request) {
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.manager.Approve(r.Context(), ws.ID, types.UploadID(chi.URLParam(r, "upload_id")), actor)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, out)
}

func (h *uploadHandlers) Cancel(w http.ResponseWriter, r *http.Request) {
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.manager.Cancel(r.Context(), ws.ID, types.UploadID(chi.URLParam(r, "upload_id")), actor, r.URL.Query().Get("reason"))
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, out)
}

func (h *uploadHandlers) Get(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.store.Uploads().GetByID(r.Context(), ws.ID, types.UploadID(chi.URLParam(r, "upload_id")))
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, out)
}

func (h *uploadHandlers) ProposedMapping(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	u, err := h.store.Uploads().GetByID(r.Context(), ws.ID, types.UploadID(chi.URLParam(r, "upload_id")))
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{
		"proposed_mapping": u.ProposedColumnMapping,
		"confidence":       u.MappingConfidence,
		"reasoning":        "", // persisted separately if needed
	})
}

func (h *uploadHandlers) List(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	asset := chi.URLParam(r, "asset")
	uploads, err := h.store.Uploads().List(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var out []types.Upload
	for _, u := range uploads {
		if string(u.Asset) == asset {
			out = append(out, u)
		}
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"uploads": out})
}

func (h *uploadHandlers) Retry(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	id := types.UploadID(chi.URLParam(r, "upload_id"))
	u, err := h.store.Uploads().GetByID(r.Context(), ws.ID, id)
	if err != nil {
		writeError(w, r, err)
		return
	}
	if !u.Status.IsTerminal() {
		writeError(w, r, errs.Newf(errs.CodeValidation, "upload %q is not in a terminal state", id))
		return
	}
	// Reset to Initiated and re-run the pipeline
	u.Status = types.UploadStatusInitiated
	u.ErrorMessage = ""
	u.Metadata = nil
	updated, err := h.store.Uploads().Update(r.Context(), u)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, updated)
}

func (h *uploadHandlers) Errors(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	u, err := h.store.Uploads().GetByID(r.Context(), ws.ID, types.UploadID(chi.URLParam(r, "upload_id")))
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{
		"status":       u.Status,
		"error_message": u.ErrorMessage,
		"metadata":     u.Metadata,
	})
}

func (h *uploadHandlers) ListVersions(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	asset := types.APIName(chi.URLParam(r, "asset"))
	a, err := h.store.Assets().GetByAPIName(r.Context(), ws.ID, asset)
	if err != nil {
		writeError(w, r, err)
		return
	}
	versions, err := h.store.AssetVersions().ListByAsset(r.Context(), ws.ID, a.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"versions": versions})
}

func (h *uploadHandlers) LatestVersion(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	asset := types.APIName(chi.URLParam(r, "asset"))
	a, err := h.store.Assets().GetByAPIName(r.Context(), ws.ID, asset)
	if err != nil {
		writeError(w, r, err)
		return
	}
	version, err := h.store.AssetVersions().GetLatestByAsset(r.Context(), ws.ID, a.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, version)
}

func (h *uploadHandlers) context(r *http.Request) (types.Workspace, types.Actor, error) {
	name := chi.URLParam(r, "workspace")
	ws, err := h.store.Workspaces().GetByAPIName(r.Context(), types.APIName(name))
	if err != nil {
		return types.Workspace{}, types.Actor{}, err
	}
	actor, err := actorFromContext(r.Context())
	if err != nil {
		return types.Workspace{}, types.Actor{}, errs.Wrap(err, errs.CodeUnauthenticated, "actor")
	}
	return ws, actor, nil
}
