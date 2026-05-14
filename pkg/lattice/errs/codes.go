// Codes are the closed set of error categories Lattice produces. Adding a new
// Code requires updating the HTTP mapper and any downstream classifiers.

package errs

// Code is a stable, machine-readable error category.
type Code string

// The canonical set of error codes. These strings appear on the wire and in
// telemetry; they are part of the public contract and must not be renamed.
const (
	// CodeNotFound — the requested resource does not exist.
	CodeNotFound Code = "not_found"
	// CodeForbidden — the actor is authenticated but the operation is denied
	// by an explicit policy or capability check.
	CodeForbidden Code = "forbidden"
	// CodeUnauthenticated — no actor could be resolved (missing or invalid token).
	CodeUnauthenticated Code = "unauthenticated"
	// CodeValidation — the request payload failed schema or semantic validation.
	CodeValidation Code = "validation_error"
	// CodeConflict — the request conflicts with current state (duplicate id,
	// idempotency-key collision with a different payload, write-write race).
	CodeConflict Code = "conflict"
	// CodeRateLimited — the actor or workspace exceeded a quota.
	CodeRateLimited Code = "rate_limited"
	// CodeInternal — unexpected server-side failure; retryable from the client's
	// perspective only if explicitly indicated.
	CodeInternal Code = "internal_error"
	// CodeAdapter — a downstream data store or external dependency failed.
	CodeAdapter Code = "adapter_error"
	// CodePolicyDenied — the policy engine produced a deny decision. Distinct from
	// CodeForbidden so audit and observability can separate "no rule allowed" from
	// "an explicit deny rule matched" — both surface as HTTP 403.
	CodePolicyDenied Code = "policy_denied"
)

// String returns the wire representation of the code.
func (c Code) String() string { return string(c) }
