// ActionType is a typed mutation: input schema, optional output schema, a
// permission key, and a handler configuration that decides how it executes.

package types

import (
	"encoding/json"
	"time"
)

// ActionTypeID is the canonical handle for an action type.
type ActionTypeID string

// ExecutionMode selects synchronous or asynchronous dispatch.
type ExecutionMode string

const (
	ExecutionModeSync  ExecutionMode = "sync"
	ExecutionModeAsync ExecutionMode = "async"
)

// HandlerKind selects how an action is dispatched.
type HandlerKind string

const (
	HandlerKindCRUDCreate HandlerKind = "crud_create"
	HandlerKindCRUDUpdate HandlerKind = "crud_update"
	HandlerKindCRUDDelete HandlerKind = "crud_delete"
	HandlerKindWebhook    HandlerKind = "webhook"
	HandlerKindComposite  HandlerKind = "composite"
	HandlerKindCallback   HandlerKind = "callback"
)

// ActionType describes a typed mutation that a client may execute against a
// subject object type (or no subject for global actions).
type ActionType struct {
	ID                     ActionTypeID    `json:"id"`
	WorkspaceID            WorkspaceID     `json:"workspace_id"`
	APIName                APIName         `json:"api_name"`
	DisplayName            string          `json:"display_name,omitempty"`
	Description            string          `json:"description,omitempty"`
	Subject                APIName         `json:"subject,omitempty"`
	InputSchema            json.RawMessage `json:"input_schema"`
	OutputSchema           json.RawMessage `json:"output_schema,omitempty"`
	PermissionKey          string          `json:"permission_key"`
	IdempotencyKeyTemplate string          `json:"idempotency_key_template,omitempty"`
	ExecutionMode          ExecutionMode   `json:"execution_mode"`
	Handler                HandlerConfig   `json:"handler"`
	Version                int             `json:"version"`
	DeprecatedAt           *time.Time      `json:"deprecated_at,omitempty"`
	CreatedAt              time.Time       `json:"created_at"`
	UpdatedAt              time.Time       `json:"updated_at"`
}

// HandlerConfig is the dispatch configuration for an action. Only one of the
// embedded sections is populated, selected by Kind.
type HandlerConfig struct {
	Kind      HandlerKind       `json:"kind"`
	CRUD      *CRUDHandler      `json:"crud,omitempty"`
	Webhook   *WebhookHandler   `json:"webhook,omitempty"`
	Composite *CompositeHandler `json:"composite,omitempty"`
}

// CRUDHandler describes a declarative mutation against the subject's adapter.
type CRUDHandler struct {
	// Mappings is a declarative input→property map. Each entry assigns the
	// named action input field (or a CEL expression over the input/subject)
	// to a target property on the subject object type.
	Mappings []CRUDMapping `json:"mappings"`
}

// CRUDMapping pairs an action input field with a target property on the subject.
type CRUDMapping struct {
	TargetProperty APIName `json:"target_property"`
	Expression     string  `json:"expression"`
}

// WebhookHandler describes an external HTTP handler.
type WebhookHandler struct {
	URL              string        `json:"url"`
	TimeoutSeconds   int           `json:"timeout_seconds"`
	MaxRetries       int           `json:"max_retries"`
	SigningSecretRef string        `json:"signing_secret_ref,omitempty"`
	RetryOnStatus    []int         `json:"retry_on_status,omitempty"`
	HeaderForwards   []string      `json:"header_forwards,omitempty"`
	BackoffInitialMS int           `json:"backoff_initial_ms,omitempty"`
	BackoffMaxMS     int           `json:"backoff_max_ms,omitempty"`
	BackoffJitter    float64       `json:"backoff_jitter,omitempty"`
	_unused          time.Duration // reserved
}

// CompositeHandler describes a sequence of steps executed in order.
type CompositeHandler struct {
	Steps []CompositeStep `json:"steps"`
}

// CompositeStep is one step in a composite handler.
type CompositeStep struct {
	Name      string             `json:"name"`
	ActionRef APIName            `json:"action_ref"`
	InputExpr map[string]string  `json:"input_expr,omitempty"`
	OnFailure CompositeOnFailure `json:"on_failure"`
}

// CompositeOnFailure controls whether a composite proceeds after a step fails.
type CompositeOnFailure string

const (
	CompositeOnFailureAbort CompositeOnFailure = "abort"
	CompositeOnFailureSkip  CompositeOnFailure = "skip"
)
