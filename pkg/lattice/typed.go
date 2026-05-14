// Generic typed registration and typed helpers.
//
// Register[T]      — declare object type from a Go struct
// Action[In,Subj]  — declare action from input/subject structs
// Link[From,To]    — declare link from type parameters
// RegisterAgent    — declare an agent
// OnChange[T]      — subscribe to object lifecycle events
// OnAudit            — subscribe to audit events

package lattice

import (
	"context"
	"encoding/json"
	"fmt"
	"reflect"

	"github.com/miguelcsx/lattice/pkg/lattice/events"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// ---------------------------------------------------------------------
// Type[T] — typed object registration
// ---------------------------------------------------------------------

// Type[T] is the typed handle returned by Register. Methods on it accept
// callbacks that work on T directly (no manual marshaling).
type Type[T any] struct {
	app     *App
	builder *ObjectTypeBuilder
	specs   []fieldSpec
	pkField string // Go field name of the primary key
}

// Register declares an object type from a Go struct. The struct's `lattice`
// tags drive property metadata; field names default to snake_case.
func Register[T any](app *App) *Type[T] {
	var zero T
	t := reflect.TypeOf(zero)
	if t == nil {
		app.addError(fmt.Errorf("Register: T must be a concrete type"))
		return &Type[T]{app: app}
	}
	for t.Kind() == reflect.Ptr {
		t = t.Elem()
	}
	if t.Kind() != reflect.Struct {
		app.addError(fmt.Errorf("Register[%s]: T must be a struct", t.String()))
		return &Type[T]{app: app}
	}

	specs, err := describeStruct(t)
	if err != nil {
		app.addError(err)
		return &Type[T]{app: app}
	}

	name := t.Name()
	b := app.ObjectType(name)
	pkGoField := ""
	for i, spec := range specs {
		b.Property(spec.APIName, spec.DataType)
		if spec.PrimaryKey {
			b.PrimaryKey()
			pkGoField = t.Field(specs[i].fieldIndex(t)).Name
		}
		if spec.Indexed {
			b.Indexed()
		}
		if spec.Nullable {
			b.Nullable()
		}
		for _, tag := range spec.Tags {
			b.Tag(tag)
		}
		if len(spec.AllowedValues) > 0 {
			b.AllowedValues(spec.AllowedValues...)
		}
		if len(spec.Markings) > 0 {
			b.Marking(spec.Markings...)
		}
	}
	return &Type[T]{app: app, builder: b, specs: specs, pkField: pkGoField}
}

// Search registers a typed search backend.
func (h *Type[T]) Search(fn func(ctx context.Context, q Query) ([]T, error)) *Type[T] {
	if h.builder == nil {
		return h
	}
	h.builder.Search(func(ctx context.Context, q Query) (Page, error) {
		items, err := fn(ctx, q)
		if err != nil {
			return Page{}, err
		}
		return marshalSlice(items, h.specs)
	})
	return h
}

// Get registers a typed primary-key lookup.
func (h *Type[T]) Get(fn func(ctx context.Context, pk string) (T, error)) *Type[T] {
	if h.builder == nil {
		return h
	}
	h.builder.Get(func(ctx context.Context, pk any) (Record, error) {
		key := fmt.Sprint(pk)
		item, err := fn(ctx, key)
		if err != nil {
			return Record{}, err
		}
		return marshalOne(item, h.specs)
	})
	return h
}

// Mutate registers a typed write callback.
func (h *Type[T]) Mutate(fn func(ctx context.Context, mut Mutation) (T, error)) *Type[T] {
	if h.builder == nil {
		return h
	}
	h.builder.Mutate(func(ctx context.Context, mut Mutation) (MutationResult, error) {
		item, err := fn(ctx, mut)
		if err != nil {
			return MutationResult{}, err
		}
		rec, err := marshalOne(item, h.specs)
		if err != nil {
			return MutationResult{}, err
		}
		out := MutationResult{AffectedRows: 1}
		out.Returned = make(map[types.APIName]any, len(rec.Values))
		for k, v := range rec.Values {
			out.Returned[k] = v
		}
		return out, nil
	})
	return h
}

// DisplayName overrides the human-readable name.
func (h *Type[T]) DisplayName(s string) *Type[T] {
	if h.builder != nil {
		h.builder.DisplayName(s)
	}
	return h
}

// Description sets the docs string.
func (h *Type[T]) Description(s string) *Type[T] {
	if h.builder != nil {
		h.builder.Description(s)
	}
	return h
}

// Source binds this type to a registered datasource and table.
func (h *Type[T]) Source(datasource, table string) *Type[T] {
	if h.builder != nil {
		h.builder.Source(datasource, table)
	}
	return h
}

// fieldIndex finds the StructField index for spec.APIName by matching the
// snake_case-converted Go name.
func (s fieldSpec) fieldIndex(t reflect.Type) int {
	for i := 0; i < t.NumField(); i++ {
		f := t.Field(i)
		if defaultAPIName(f.Name) == s.APIName {
			return i
		}
		if name, _, _ := splitKV(f.Tag.Get("lattice")); name == s.APIName {
			return i
		}
	}
	return -1
}

// marshalSlice converts []T into a Page by JSON round-tripping each item.
func marshalSlice[T any](items []T, _ []fieldSpec) (Page, error) {
	out := Page{Records: make([]Record, 0, len(items))}
	for _, it := range items {
		rec, err := marshalOne(it, nil)
		if err != nil {
			return Page{}, err
		}
		out.Records = append(out.Records, rec)
	}
	return out, nil
}

// marshalOne converts T to a Record. Uses json round-trip.
func marshalOne(item any, _ []fieldSpec) (Record, error) {
	raw, err := json.Marshal(item)
	if err != nil {
		return Record{}, fmt.Errorf("lattice: marshal item: %w", err)
	}
	var m map[string]any
	if err := json.Unmarshal(raw, &m); err != nil {
		return Record{}, fmt.Errorf("lattice: unmarshal to map: %w", err)
	}
	values := make(map[types.APIName]any, len(m))
	for k, v := range m {
		values[types.APIName(snakeJSON(k))] = v
	}
	return Record{Values: values}, nil
}

func snakeJSON(s string) string { return defaultAPIName(s) }

// ---------------------------------------------------------------------
// Typed action declaration
// ---------------------------------------------------------------------

// Action declares a typed action. Input is the input struct; Subject is the
// object type acted on (use a placeholder type if the action has no subject).
func Action[Input, Subject any](app *App, apiName string) *ActionBuilder {
	var s Subject
	subjectType := reflect.TypeOf(s)
	for subjectType != nil && subjectType.Kind() == reflect.Ptr {
		subjectType = subjectType.Elem()
	}
	b := app.Action(apiName)
	if subjectType != nil && subjectType.Name() != "" {
		b.Subject(subjectType.Name())
	}
	if schema := schemaFromType[Input](); schema != nil {
		b.InputSchema(schema)
	}
	return b
}

// schemaFromType produces a minimal JSON-Schema map from T's struct tags.
func schemaFromType[T any]() map[string]any {
	var z T
	t := reflect.TypeOf(z)
	for t != nil && t.Kind() == reflect.Ptr {
		t = t.Elem()
	}
	if t == nil || t.Kind() != reflect.Struct {
		return nil
	}
	props := map[string]any{}
	required := []string{}
	for i := 0; i < t.NumField(); i++ {
		f := t.Field(i)
		spec := parseFieldSpec(f)
		if spec.Skip {
			continue
		}
		props[spec.APIName] = map[string]any{"type": jsonSchemaType(spec.DataType)}
		if !spec.Nullable {
			required = append(required, spec.APIName)
		}
	}
	out := map[string]any{"type": "object", "properties": props}
	if len(required) > 0 {
		out["required"] = required
	}
	return out
}

func jsonSchemaType(dt any) string {
	switch dt {
	case "integer", "bigint":
		return "integer"
	case "float", "decimal":
		return "number"
	case "boolean":
		return "boolean"
	case "json":
		return "object"
	default:
		return "string"
	}
}

// ---------------------------------------------------------------------
// Typed agent registration
// ---------------------------------------------------------------------

// AgentConfig is the typed declaration of an agent.
type AgentConfig struct {
	Name                      string
	DisplayName, Description  string
	Provider, Model          string
	SystemPrompt              string
	Temperature               float64
	MaxTokens                 int
	From                      []any
	AllowedRoles              []string
	Limits                    AgentLimits
	RequireApprovalForActions bool
}

// AgentLimits is re-exported here for ergonomic agent declarations.
type AgentLimits = types.AgentLimits

// LinkRef references an existing link type by api_name.
type LinkRef string

// ActionRef references an existing action type by api_name.
type ActionRef string

// CustomToolRef references a registered custom tool by api_name.
type CustomToolRef string

// RegisterAgent declares an agent on app and returns the materialized type.
func RegisterAgent(app *App, cfg AgentConfig) types.Agent {
	a := types.Agent{
		ID:                        types.AgentID(ids.NewULID()),
		WorkspaceID:               app.workspace.ID,
		APIName:                   types.APIName(cfg.Name),
		DisplayName:               cfg.DisplayName,
		Description:               cfg.Description,
		SystemPrompt:              cfg.SystemPrompt,
		Model:                     types.ModelConfig{Provider: cfg.Provider, Model: cfg.Model, Temperature: cfg.Temperature, MaxTokens: cfg.MaxTokens},
		Limits:                    cfg.Limits,
		RequireApprovalForActions: cfg.RequireApprovalForActions,
	}
	for _, role := range cfg.AllowedRoles {
		a.AllowedRoles = append(a.AllowedRoles, types.APIName(role))
	}
	for _, src := range cfg.From {
		switch v := src.(type) {
		case LinkRef:
			a.FromLinkTypes = append(a.FromLinkTypes, types.APIName(v))
		case ActionRef:
			a.FromActions = append(a.FromActions, types.APIName(v))
		case CustomToolRef:
			a.CustomTools = append(a.CustomTools, types.APIName(v))
		default:
			t := reflect.TypeOf(src)
			for t != nil && t.Kind() == reflect.Ptr {
				t = t.Elem()
			}
			if t != nil && t.Name() != "" {
				a.FromObjectTypes = append(a.FromObjectTypes, types.APIName(t.Name()))
			}
		}
	}
	app.addAgent(a)
	return a
}

// ---------------------------------------------------------------------
// Typed link declaration
// ---------------------------------------------------------------------

// Link declares a directed link between two registered object types.
func Link[From, To any](app *App, apiName string) *LinkTypeBuilder {
	var z1 From
	var z2 To
	t1 := reflect.TypeOf(z1)
	t2 := reflect.TypeOf(z2)
	for t1 != nil && t1.Kind() == reflect.Ptr {
		t1 = t1.Elem()
	}
	for t2 != nil && t2.Kind() == reflect.Ptr {
		t2 = t2.Elem()
	}
	from, to := "", ""
	if t1 != nil {
		from = t1.Name()
	}
	if t2 != nil {
		to = t2.Name()
	}
	return app.LinkType(apiName).From(from).To(to)
}

// ---------------------------------------------------------------------
// Typed subscription helpers
// ---------------------------------------------------------------------

// Change is the typed payload delivered to OnChange handlers.
type Change[T any] struct {
	Kind       events.Kind
	PrimaryKey string
	Actor      string
	Before     *T
	After      *T
}

// OnChange subscribes to lifecycle events for the object type whose Go type is T.
func OnChange[T any](a *App, fn func(ctx context.Context, c Change[T]) error) (events.Subscription, error) {
	var z T
	tt := reflect.TypeOf(z)
	for tt != nil && tt.Kind() == reflect.Ptr {
		tt = tt.Elem()
	}
	apiName := defaultAPIName(tt.Name())
	filter := events.Filter{
		Kinds: []events.Kind{
			events.KindObjectCreated,
			events.KindObjectUpdated,
			events.KindObjectDeleted,
		},
	}
	if tt.Name() != "" {
		filter.ObjectTypes = nil
	}
	return a.bus.Subscribe(filter, func(ctx context.Context, e events.Event) error {
		if string(e.ObjectType) != apiName {
			return nil
		}
		c := Change[T]{Kind: e.Kind, PrimaryKey: e.PrimaryKey, Actor: e.Actor}
		if len(e.Body) > 0 {
			var raw struct {
				Before json.RawMessage `json:"before"`
				After  json.RawMessage `json:"after"`
			}
			_ = json.Unmarshal(e.Body, &raw)
			if len(raw.Before) > 0 {
				var b T
				if err := json.Unmarshal(raw.Before, &b); err == nil {
					c.Before = &b
				}
			}
			if len(raw.After) > 0 {
				var b T
				if err := json.Unmarshal(raw.After, &b); err == nil {
					c.After = &b
				}
			}
		}
		return fn(ctx, c)
	})
}

// OnAudit subscribes to every audit emission.
func (a *App) OnAudit(fn func(ctx context.Context, e events.Event) error) (events.Subscription, error) {
	return a.bus.Subscribe(events.Filter{Kinds: []events.Kind{events.KindAuditEmitted}}, fn)
}

// OnEvent is the lowest-level subscription helper — pass-through to the bus.
func (a *App) OnEvent(filter events.Filter, fn func(ctx context.Context, e events.Event) error) (events.Subscription, error) {
	return a.bus.Subscribe(filter, fn)
}

// Webhook registers an outbound HTTP webhook that receives every event
// matching filter. Returns the Subscription handle (close to disable).
func (a *App) Webhook(url, secret string, filter events.Filter) (events.Subscription, error) {
	w := &events.WebhookSink{URL: url, Secret: secret}
	return a.bus.Subscribe(filter, w.AsHandler())
}
