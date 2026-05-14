// HTTP error mapping. Translates internal errors (errs.Code) and store
// sentinels into JSON error responses with the right status code.

package server

import (
	"encoding/json"
	"errors"
	"log/slog"
	"net/http"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
)

// codeStatus is the canonical mapping from internal error codes to HTTP
// status codes. Adding a new errs.Code requires one entry here.
var codeStatus = map[errs.Code]int{
	errs.CodeNotFound:        http.StatusNotFound,
	errs.CodeForbidden:       http.StatusForbidden,
	errs.CodePolicyDenied:    http.StatusForbidden,
	errs.CodeUnauthenticated: http.StatusUnauthorized,
	errs.CodeValidation:      http.StatusBadRequest,
	errs.CodeConflict:        http.StatusConflict,
	errs.CodeRateLimited:     http.StatusTooManyRequests,
	errs.CodeAdapter:         http.StatusBadGateway,
	errs.CodeInternal:        http.StatusInternalServerError,
}

// errorBody is the wire form of the error response.
type errorBody struct {
	Error errorPayload `json:"error"`
}

type errorPayload struct {
	Code      string         `json:"code"`
	Message   string         `json:"message"`
	Details   map[string]any `json:"details,omitempty"`
	RequestID string         `json:"request_id,omitempty"`
}

// writeError serializes err as JSON with the appropriate status code.
func writeError(w http.ResponseWriter, r *http.Request, err error) {
	if err == nil {
		return
	}
	requestID := requestIDFromContext(r.Context())
	if le, ok := errs.As(err); ok {
		writeStructured(w, le, requestID)
		return
	}
	if errors.Is(err, storage.ErrNotFound) {
		writeStructured(w, errs.New(errs.CodeNotFound, err.Error()), requestID)
		return
	}
	if errors.Is(err, storage.ErrConflict) {
		writeStructured(w, errs.New(errs.CodeConflict, err.Error()), requestID)
		return
	}
	slog.Error("unhandled api error", "err", err, "request_id", requestID)
	writeStructured(w, errs.New(errs.CodeInternal, "internal error"), requestID)
}

func writeStructured(w http.ResponseWriter, e *errs.Error, requestID string) {
	status, ok := codeStatus[e.Code]
	if !ok {
		status = http.StatusInternalServerError
	}
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	body := errorBody{Error: errorPayload{
		Code:      string(e.Code),
		Message:   e.Message,
		Details:   e.Details,
		RequestID: requestID,
	}}
	_ = json.NewEncoder(w).Encode(body)
}
