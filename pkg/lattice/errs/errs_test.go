package errs_test

import (
	"errors"
	"fmt"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
)

func TestNew_PopulatesCodeAndMessage(t *testing.T) {
	t.Parallel()

	err := errs.New(errs.CodeNotFound, "user not found")
	if err == nil {
		t.Fatal("New must not return nil")
	}
	if err.Code != errs.CodeNotFound {
		t.Fatalf("Code: want %q, got %q", errs.CodeNotFound, err.Code)
	}
	if err.Message != "user not found" {
		t.Fatalf("Message: want %q, got %q", "user not found", err.Message)
	}
	if err.Error() == "" {
		t.Fatal("Error() must not be empty")
	}
}

func TestNewf_FormatsMessage(t *testing.T) {
	t.Parallel()

	err := errs.Newf(errs.CodeValidation, "field %q is required", "email")
	want := `field "email" is required`
	if err.Message != want {
		t.Fatalf("Message: want %q, got %q", want, err.Message)
	}
}

func TestWrap_PreservesCauseAndCode(t *testing.T) {
	t.Parallel()

	cause := errors.New("io: closed")
	wrapped := errs.Wrap(cause, errs.CodeAdapter, "executing query")

	if wrapped.Code != errs.CodeAdapter {
		t.Fatalf("Code: want %q, got %q", errs.CodeAdapter, wrapped.Code)
	}
	if !errors.Is(wrapped, cause) {
		t.Fatal("errors.Is must find the original cause through Unwrap")
	}
	if wrapped.Message != "executing query" {
		t.Fatalf("Message: want %q, got %q", "executing query", wrapped.Message)
	}
}

func TestWrap_NilReturnsNil(t *testing.T) {
	t.Parallel()

	if got := errs.Wrap(nil, errs.CodeInternal, "noop"); got != nil {
		t.Fatalf("Wrap(nil, ...) must return nil, got %v", got)
	}
}

func TestWrapf_FormatsMessage(t *testing.T) {
	t.Parallel()

	cause := errors.New("boom")
	wrapped := errs.Wrapf(cause, errs.CodeInternal, "while processing %d items", 42)
	if wrapped.Message != "while processing 42 items" {
		t.Fatalf("Message: got %q", wrapped.Message)
	}
}

func TestError_FormatIncludesCodeAndCause(t *testing.T) {
	t.Parallel()

	cause := errors.New("disk full")
	err := errs.Wrap(cause, errs.CodeInternal, "saving snapshot")
	got := err.Error()
	want := "internal_error: saving snapshot: disk full"
	if got != want {
		t.Fatalf("Error(): want %q, got %q", want, got)
	}
}

func TestError_FormatWithoutCause(t *testing.T) {
	t.Parallel()

	got := errs.New(errs.CodeNotFound, "missing").Error()
	want := "not_found: missing"
	if got != want {
		t.Fatalf("Error(): want %q, got %q", want, got)
	}
}

func TestIs_MatchesByCode(t *testing.T) {
	t.Parallel()

	err := errs.New(errs.CodeForbidden, "no access")
	if !errs.Is(err, errs.CodeForbidden) {
		t.Fatal("Is must report true when codes match")
	}
	if errs.Is(err, errs.CodeNotFound) {
		t.Fatal("Is must report false when codes differ")
	}
	if errs.Is(nil, errs.CodeForbidden) {
		t.Fatal("Is(nil, ...) must be false")
	}
}

func TestIs_FindsThroughErrorsWrap(t *testing.T) {
	t.Parallel()

	inner := errs.New(errs.CodePolicyDenied, "policy denied")
	outer := fmt.Errorf("outer: %w", inner)

	if !errs.Is(outer, errs.CodePolicyDenied) {
		t.Fatal("Is must traverse fmt.Errorf-wrapped errors")
	}
}

func TestAs_ReturnsTypedErrorWhenPresent(t *testing.T) {
	t.Parallel()

	original := errs.New(errs.CodeConflict, "duplicate id")
	wrapped := fmt.Errorf("repo: %w", original)

	got, ok := errs.As(wrapped)
	if !ok {
		t.Fatal("As must return ok=true for an errs.Error in the chain")
	}
	if got.Code != errs.CodeConflict {
		t.Fatalf("As returned wrong error: %v", got)
	}
}

func TestAs_ReturnsFalseForUnrelatedError(t *testing.T) {
	t.Parallel()

	if _, ok := errs.As(errors.New("not ours")); ok {
		t.Fatal("As must return false for an error that is not an errs.Error")
	}
	if _, ok := errs.As(nil); ok {
		t.Fatal("As(nil) must return false")
	}
}

func TestWithDetails_AttachesAndRoundtrips(t *testing.T) {
	t.Parallel()

	err := errs.New(errs.CodeValidation, "bad input").WithDetails(map[string]any{
		"field": "email",
		"rule":  "format",
	})

	if got := err.Details["field"]; got != "email" {
		t.Fatalf("Details[field]: got %v", got)
	}
	if got := err.Details["rule"]; got != "format" {
		t.Fatalf("Details[rule]: got %v", got)
	}
}

func TestWithDetails_MergesPreviousEntries(t *testing.T) {
	t.Parallel()

	err := errs.New(errs.CodeValidation, "x").
		WithDetails(map[string]any{"a": 1}).
		WithDetails(map[string]any{"b": 2})

	if err.Details["a"] != 1 || err.Details["b"] != 2 {
		t.Fatalf("WithDetails must merge: %+v", err.Details)
	}
}

func TestWithDetail_AddsSingleKey(t *testing.T) {
	t.Parallel()

	err := errs.New(errs.CodeRateLimited, "slow down").
		WithDetail("retry_after_seconds", 60)

	if err.Details["retry_after_seconds"] != 60 {
		t.Fatalf("WithDetail did not store value: %+v", err.Details)
	}
}

func TestCode_IsString(t *testing.T) {
	t.Parallel()

	codes := []errs.Code{
		errs.CodeNotFound, errs.CodeForbidden, errs.CodeUnauthenticated,
		errs.CodeValidation, errs.CodeConflict, errs.CodeRateLimited,
		errs.CodeInternal, errs.CodeAdapter, errs.CodePolicyDenied,
	}
	seen := make(map[errs.Code]struct{}, len(codes))
	for _, c := range codes {
		if string(c) == "" {
			t.Fatalf("code %v is empty", c)
		}
		if _, dup := seen[c]; dup {
			t.Fatalf("duplicate code %q", c)
		}
		seen[c] = struct{}{}
	}
}

func TestUnwrap_ReturnsCause(t *testing.T) {
	t.Parallel()

	cause := errors.New("root")
	err := errs.Wrap(cause, errs.CodeInternal, "ctx")

	if got := errors.Unwrap(err); got != cause {
		t.Fatalf("Unwrap: want %v, got %v", cause, got)
	}
}
