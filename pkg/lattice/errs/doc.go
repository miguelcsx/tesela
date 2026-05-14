// Package errs defines Lattice's structured error model.
//
// Every error that crosses a package boundary inside Lattice is an *errs.Error
// carrying a stable Code, a human-readable Message, optional Details, and an
// optional cause that can be unwrapped via errors.Unwrap. The HTTP layer maps
// each Code to a concrete status code (see internal/api/middleware/errors.go),
// and the wire format is { "error": { "code", "message", "details", "request_id" } }.
//
// Use:
//
//	return errs.New(errs.CodeValidation, "invalid filter").
//	    WithDetail("field", "status")
//
//	if err := repo.Get(ctx); err != nil {
//	    return errs.Wrap(err, errs.CodeAdapter, "loading customer")
//	}
//
//	if errs.Is(err, errs.CodeNotFound) { ... }
package errs
