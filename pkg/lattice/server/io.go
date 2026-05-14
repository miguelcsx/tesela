// Request/response (de)serialization helpers.

package server

import (
	"encoding/json"
	"errors"
	"io"
	"net/http"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
)

// maxBody caps the inbound body size. The HTTPConfig override is honored
// inside applyMaxBody.
const defaultMaxBody = 4 << 20 // 4 MiB

func writeJSON(w http.ResponseWriter, _ *http.Request, status int, body any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	if body == nil {
		return
	}
	_ = json.NewEncoder(w).Encode(body)
}

func decodeJSON(r *http.Request, dst any) error {
	max := defaultMaxBody
	body := http.MaxBytesReader(nil, r.Body, int64(max))
	defer body.Close()
	dec := json.NewDecoder(body)
	dec.DisallowUnknownFields()
	if err := dec.Decode(dst); err != nil {
		if errors.Is(err, io.EOF) {
			return errs.New(errs.CodeValidation, "request body is empty")
		}
		return errs.Wrap(err, errs.CodeValidation, "decode body")
	}
	return nil
}

func readBody(r *http.Request) ([]byte, error) {
	body := http.MaxBytesReader(nil, r.Body, int64(defaultMaxBody))
	defer body.Close()
	return io.ReadAll(body)
}
