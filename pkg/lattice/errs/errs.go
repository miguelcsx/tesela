// Error is the typed error value that flows through Lattice. Constructors
// (New, Newf, Wrap, Wrapf) and inspection helpers (Is, As) below form the
// only sanctioned way to produce or examine one.

package errs

import (
	"errors"
	"fmt"
)

// Error is the canonical Lattice error. Construct with New / Newf / Wrap / Wrapf.
type Error struct {
	Code    Code
	Message string
	Details map[string]any

	cause error
}

// New returns a new error with no cause.
func New(code Code, message string) *Error {
	return &Error{Code: code, Message: message}
}

// Newf returns a new error whose Message is formatted via fmt.Sprintf.
func Newf(code Code, format string, args ...any) *Error {
	return &Error{Code: code, Message: fmt.Sprintf(format, args...)}
}

// Wrap returns an error tagged with code and a message, preserving cause for
// errors.Is / errors.Unwrap traversal. Wrap(nil, ...) returns nil so callers
// can write `return errs.Wrap(repo.Get())` without a separate nil check.
func Wrap(cause error, code Code, message string) *Error {
	if cause == nil {
		return nil
	}
	return &Error{Code: code, Message: message, cause: cause}
}

// Wrapf is the formatted-message form of Wrap.
func Wrapf(cause error, code Code, format string, args ...any) *Error {
	if cause == nil {
		return nil
	}
	return &Error{Code: code, Message: fmt.Sprintf(format, args...), cause: cause}
}

// Error implements the error interface. Format: "<code>: <message>[: <cause>]".
func (e *Error) Error() string {
	if e.cause == nil {
		return string(e.Code) + ": " + e.Message
	}
	return string(e.Code) + ": " + e.Message + ": " + e.cause.Error()
}

// Unwrap exposes the underlying cause for errors.Is / errors.As traversal.
func (e *Error) Unwrap() error { return e.cause }

// WithDetails merges additional context into Details. Nil entries are ignored.
// Returns the receiver for fluent construction.
func (e *Error) WithDetails(details map[string]any) *Error {
	if len(details) == 0 {
		return e
	}
	if e.Details == nil {
		e.Details = make(map[string]any, len(details))
	}
	for k, v := range details {
		e.Details[k] = v
	}
	return e
}

// WithDetail attaches a single key/value pair to Details.
func (e *Error) WithDetail(key string, value any) *Error {
	if e.Details == nil {
		e.Details = make(map[string]any, 1)
	}
	e.Details[key] = value
	return e
}

// Is reports whether err (or any error in its chain) is an *Error with the
// given Code. Returns false for nil err.
func Is(err error, code Code) bool {
	if err == nil {
		return false
	}
	var le *Error
	if errors.As(err, &le) {
		return le.Code == code
	}
	return false
}

// As extracts the first *Error in err's chain. Returns ok=false if err is nil
// or contains no *Error.
func As(err error) (*Error, bool) {
	if err == nil {
		return nil, false
	}
	var le *Error
	if errors.As(err, &le) {
		return le, true
	}
	return nil, false
}
