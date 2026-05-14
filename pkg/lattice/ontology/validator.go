// Validator runs every structural and semantic check that must pass before a
// Materialized ontology can be persisted.
//
// Validators are organized as a list of independent checks; each accumulates
// errors in a shared accumulator. The caller (the registry) reports the full
// set of errors at once so users can fix many issues per round-trip.

package ontology

import (
	"errors"
	"fmt"

	"github.com/google/cel-go/cel"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// ValidationError aggregates per-entity validation messages.
type ValidationError struct {
	Issues []Issue
}

// Issue is a single validator finding.
type Issue struct {
	Kind    types.Kind `json:"kind"`
	APIName string     `json:"api_name,omitempty"`
	Field   string     `json:"field,omitempty"`
	Message string     `json:"message"`
}

// Error implements error.
func (v *ValidationError) Error() string {
	if len(v.Issues) == 0 {
		return "ontology: no validation issues"
	}
	if len(v.Issues) == 1 {
		return formatIssue(v.Issues[0])
	}
	return fmt.Sprintf("ontology: %d validation issues", len(v.Issues))
}

func formatIssue(i Issue) string {
	if i.APIName == "" {
		return fmt.Sprintf("[%s] %s", i.Kind, i.Message)
	}
	return fmt.Sprintf("[%s/%s] %s", i.Kind, i.APIName, i.Message)
}

// Validate runs every check against m. Returns nil on success, or a
// *ValidationError when any check failed.
func Validate(m Materialized) error {
	acc := &accumulator{}
	for _, check := range checks {
		check(m, acc)
	}
	if len(acc.issues) == 0 {
		return nil
	}
	return &ValidationError{Issues: acc.issues}
}

// accumulator gathers validation issues into a single slice.
type accumulator struct{ issues []Issue }

func (a *accumulator) add(kind types.Kind, name, field, msg string) {
	a.issues = append(a.issues, Issue{Kind: kind, APIName: name, Field: field, Message: msg})
}

// checks is the ordered list of validators run by Validate. Adding a new
// validator means appending one entry here.
var checks = []func(Materialized, *accumulator){
	checkAPINames,
	checkObjectTypes,
	checkLinkTypes,
	checkActionTypes,
	checkRolesAcyclic,
	checkPolicyRules,
	checkCustomTools,
	checkAgents,
	checkAssets,
}

func checkAPINames(m Materialized, acc *accumulator) {
	check := func(kind types.Kind, name types.APIName) {
		if err := name.Validate(); err != nil {
			acc.add(kind, string(name), "api_name", err.Error())
		}
	}
	if err := m.Workspace.APIName.Validate(); err != nil {
		acc.add(types.KindWorkspace, string(m.Workspace.APIName), "api_name", err.Error())
	}
	for _, ds := range m.Datasources {
		check(types.KindDatasource, ds.APIName)
	}
	for _, ot := range m.ObjectTypes {
		check(types.KindObjectType, ot.APIName)
	}
	for _, lt := range m.LinkTypes {
		check(types.KindLinkType, lt.APIName)
	}
	for _, at := range m.ActionTypes {
		check(types.KindActionType, at.APIName)
	}
	for _, r := range m.Roles {
		check(types.KindRole, r.APIName)
	}
	for _, pr := range m.PolicyRules {
		check(types.KindPolicyRule, pr.APIName)
	}
}

func checkObjectTypes(m Materialized, acc *accumulator) {
	dsIndex := indexDatasources(m.Datasources)
	for _, ot := range m.ObjectTypes {
		name := string(ot.APIName)
		if ot.Source.Table == "" {
			acc.add(types.KindObjectType, name, "source.table", "table is required")
		}
		if ot.Source.DatasourceAPIName == "" {
			acc.add(types.KindObjectType, name, "source.datasource", "datasource is required")
		} else if _, ok := dsIndex[ot.Source.DatasourceAPIName]; !ok {
			acc.add(types.KindObjectType, name, "source.datasource",
				fmt.Sprintf("references unknown datasource %q", ot.Source.DatasourceAPIName))
		}
		if ot.PrimaryKey == "" {
			acc.add(types.KindObjectType, name, "primary_key", "primary_key is required")
		}
		seen := make(map[types.APIName]struct{})
		for _, p := range ot.Properties {
			if _, dup := seen[p.APIName]; dup {
				acc.add(types.KindObjectType, name, "properties",
					fmt.Sprintf("duplicate property %q", p.APIName))
			}
			seen[p.APIName] = struct{}{}
			if err := p.DataType.Validate(); err != nil {
				acc.add(types.KindObjectType, name, fmt.Sprintf("properties.%s", p.APIName), err.Error())
			}
			if p.IsComputed() {
				if _, err := compileCEL(p.Computed.Expression); err != nil {
					acc.add(types.KindObjectType, name, fmt.Sprintf("properties.%s.computed", p.APIName), err.Error())
				}
				for _, dep := range p.Computed.DependsOn {
					if dep == p.APIName {
						acc.add(types.KindObjectType, name, fmt.Sprintf("properties.%s.computed.depends_on", p.APIName),
							"computed property cannot depend on itself")
						continue
					}
					if _, ok := ot.PropertyByName(dep); !ok {
						acc.add(types.KindObjectType, name, fmt.Sprintf("properties.%s.computed.depends_on", p.APIName),
							fmt.Sprintf("depends_on %q is not declared as a property", dep))
					}
				}
			}
		}
		if ot.PrimaryKey != "" {
			if _, ok := ot.PropertyByName(ot.PrimaryKey); !ok {
				acc.add(types.KindObjectType, name, "primary_key",
					fmt.Sprintf("primary_key %q is not declared as a property", ot.PrimaryKey))
			}
		}
	}
}

func checkLinkTypes(m Materialized, acc *accumulator) {
	otIndex := indexObjectTypes(m.ObjectTypes)
	for _, lt := range m.LinkTypes {
		name := string(lt.APIName)
		if err := lt.Cardinality.Validate(); err != nil {
			acc.add(types.KindLinkType, name, "cardinality", err.Error())
		}
		from, fromOK := otIndex[lt.FromObjectType]
		to, toOK := otIndex[lt.ToObjectType]
		if !fromOK {
			acc.add(types.KindLinkType, name, "from_object_type",
				fmt.Sprintf("unknown object type %q", lt.FromObjectType))
		}
		if !toOK {
			acc.add(types.KindLinkType, name, "to_object_type",
				fmt.Sprintf("unknown object type %q", lt.ToObjectType))
		}
		if fromOK && toOK {
			for _, mp := range lt.PropertyMappings {
				if _, ok := from.PropertyByName(mp.FromProperty); !ok {
					acc.add(types.KindLinkType, name, "property_mappings",
						fmt.Sprintf("from_property %q not on %s", mp.FromProperty, lt.FromObjectType))
				}
				if _, ok := to.PropertyByName(mp.ToProperty); !ok {
					acc.add(types.KindLinkType, name, "property_mappings",
						fmt.Sprintf("to_property %q not on %s", mp.ToProperty, lt.ToObjectType))
				}
			}
		}
		if lt.Cardinality.RequiresJunction() && lt.Junction == nil {
			acc.add(types.KindLinkType, name, "junction", "many_to_many link requires a junction")
		}
	}
}

func checkActionTypes(m Materialized, acc *accumulator) {
	otIndex := indexObjectTypes(m.ObjectTypes)
	atIndex := indexActionTypes(m.ActionTypes)
	for _, at := range m.ActionTypes {
		name := string(at.APIName)
		if at.PermissionKey == "" {
			acc.add(types.KindActionType, name, "permission_key", "permission_key is required")
		}
		if at.Subject != "" {
			if _, ok := otIndex[at.Subject]; !ok {
				acc.add(types.KindActionType, name, "subject",
					fmt.Sprintf("subject %q references unknown object type", at.Subject))
			}
		}
		if at.ExecutionMode != "" && at.ExecutionMode != types.ExecutionModeSync && at.ExecutionMode != types.ExecutionModeAsync {
			acc.add(types.KindActionType, name, "execution_mode",
				fmt.Sprintf("invalid execution_mode %q", at.ExecutionMode))
		}
		validateHandler(at, otIndex, atIndex, acc)
	}
}

func validateHandler(at types.ActionType, otIndex map[types.APIName]types.ObjectType, atIndex map[types.APIName]types.ActionType, acc *accumulator) {
	name := string(at.APIName)
	switch at.Handler.Kind {
	case types.HandlerKindCRUDCreate, types.HandlerKindCRUDUpdate, types.HandlerKindCRUDDelete:
		if at.Handler.CRUD == nil {
			acc.add(types.KindActionType, name, "handler.crud", "crud config is required")
			return
		}
		if at.Subject == "" {
			acc.add(types.KindActionType, name, "subject", "crud actions require a subject")
			return
		}
		ot, ok := otIndex[at.Subject]
		if !ok {
			return
		}
		for _, m := range at.Handler.CRUD.Mappings {
			if _, ok := ot.PropertyByName(m.TargetProperty); !ok {
				acc.add(types.KindActionType, name, "handler.crud.mappings",
					fmt.Sprintf("target_property %q not on subject", m.TargetProperty))
			}
		}
	case types.HandlerKindWebhook:
		if at.Handler.Webhook == nil || at.Handler.Webhook.URL == "" {
			acc.add(types.KindActionType, name, "handler.webhook", "webhook url is required")
		}
	case types.HandlerKindComposite:
		if at.Handler.Composite == nil || len(at.Handler.Composite.Steps) == 0 {
			acc.add(types.KindActionType, name, "handler.composite", "composite requires steps")
			return
		}
		for _, step := range at.Handler.Composite.Steps {
			if _, ok := atIndex[step.ActionRef]; !ok {
				acc.add(types.KindActionType, name, "handler.composite.steps",
					fmt.Sprintf("step %q references unknown action %q", step.Name, step.ActionRef))
			}
		}
	default:
		acc.add(types.KindActionType, name, "handler.kind",
			fmt.Sprintf("unknown handler kind %q", at.Handler.Kind))
	}
}

func checkRolesAcyclic(m Materialized, acc *accumulator) {
	roles := indexRoles(m.Roles)
	for _, r := range m.Roles {
		visited := make(map[types.APIName]bool)
		stack := make(map[types.APIName]bool)
		if err := walkRole(r.APIName, roles, visited, stack); err != nil {
			acc.add(types.KindRole, string(r.APIName), "inherits", err.Error())
		}
	}
}

func walkRole(name types.APIName, idx map[types.APIName]types.Role, visited, stack map[types.APIName]bool) error {
	if stack[name] {
		return fmt.Errorf("role inheritance cycle through %q", name)
	}
	if visited[name] {
		return nil
	}
	stack[name] = true
	r, ok := idx[name]
	if !ok {
		stack[name] = false
		visited[name] = true
		return fmt.Errorf("inherits unknown role %q", name)
	}
	for _, parent := range r.Inherits {
		if err := walkRole(parent, idx, visited, stack); err != nil {
			return err
		}
	}
	stack[name] = false
	visited[name] = true
	return nil
}

func checkPolicyRules(m Materialized, acc *accumulator) {
	roles := indexRoles(m.Roles)
	otIndex := indexObjectTypes(m.ObjectTypes)
	atIndex := indexActionTypes(m.ActionTypes)
	for _, pr := range m.PolicyRules {
		name := string(pr.APIName)
		if pr.Effect != types.PolicyEffectAllow && pr.Effect != types.PolicyEffectDeny {
			acc.add(types.KindPolicyRule, name, "effect",
				fmt.Sprintf("invalid effect %q", pr.Effect))
		}
		if len(pr.Operations) == 0 {
			acc.add(types.KindPolicyRule, name, "operations", "operations is required")
		}
		for _, op := range pr.Operations {
			if err := op.Validate(); err != nil {
				acc.add(types.KindPolicyRule, name, "operations", err.Error())
			}
		}
		for _, role := range pr.Roles {
			if _, ok := roles[role]; !ok {
				acc.add(types.KindPolicyRule, name, "roles",
					fmt.Sprintf("unknown role %q", role))
			}
		}
		if pr.ObjectType != "" {
			if _, ok := otIndex[pr.ObjectType]; !ok {
				acc.add(types.KindPolicyRule, name, "object_type",
					fmt.Sprintf("unknown object type %q", pr.ObjectType))
			}
		}
		if pr.ActionType != "" {
			if _, ok := atIndex[pr.ActionType]; !ok {
				acc.add(types.KindPolicyRule, name, "action_type",
					fmt.Sprintf("unknown action type %q", pr.ActionType))
			}
		}
		if !pr.RowFilter.IsZero() {
			if err := pr.RowFilter.Validate(); err != nil {
				acc.add(types.KindPolicyRule, name, "row_filter", err.Error())
			}
		}
		for _, c := range pr.Conditions {
			if c.Kind == types.ConditionKindCEL {
				if _, err := compileCEL(c.Expression); err != nil {
					acc.add(types.KindPolicyRule, name, "conditions", err.Error())
				}
			}
		}
	}
}

func checkCustomTools(m Materialized, acc *accumulator) {
	for _, ct := range m.CustomTools {
		if len(ct.InputSchema) == 0 {
			acc.add(types.KindCustomTool, string(ct.APIName), "input_schema", "input_schema is required")
		}
		switch ct.Kind {
		case types.CustomToolKindSQL:
			if ct.SQL == nil || ct.SQL.Statement == "" {
				acc.add(types.KindCustomTool, string(ct.APIName), "sql", "sql.statement is required")
			}
		case types.CustomToolKindWebhook:
			if ct.Webhook == nil || ct.Webhook.URL == "" {
				acc.add(types.KindCustomTool, string(ct.APIName), "webhook", "webhook.url is required")
			}
		case types.CustomToolKindComposite:
			if ct.Composite == nil || len(ct.Composite.Steps) == 0 {
				acc.add(types.KindCustomTool, string(ct.APIName), "composite", "composite requires steps")
			}
		default:
			acc.add(types.KindCustomTool, string(ct.APIName), "kind",
				fmt.Sprintf("invalid custom tool kind %q", ct.Kind))
		}
	}
}

func checkAgents(m Materialized, acc *accumulator) {
	otIndex := indexObjectTypes(m.ObjectTypes)
	ltIndex := indexLinkTypes(m.LinkTypes)
	atIndex := indexActionTypes(m.ActionTypes)
	ctIndex := indexCustomTools(m.CustomTools)
	for _, a := range m.Agents {
		name := string(a.APIName)
		if a.Model.Provider == "" || a.Model.Model == "" {
			acc.add(types.KindAgent, name, "model", "model.provider and model.model are required")
		}
		for _, ot := range a.FromObjectTypes {
			if _, ok := otIndex[ot]; !ok {
				acc.add(types.KindAgent, name, "from_object_types",
					fmt.Sprintf("unknown object type %q", ot))
			}
		}
		for _, lt := range a.FromLinkTypes {
			if _, ok := ltIndex[lt]; !ok {
				acc.add(types.KindAgent, name, "from_link_types",
					fmt.Sprintf("unknown link type %q", lt))
			}
		}
		for _, at := range a.FromActions {
			if _, ok := atIndex[at]; !ok {
				acc.add(types.KindAgent, name, "from_actions",
					fmt.Sprintf("unknown action type %q", at))
			}
		}
		for _, ct := range a.CustomTools {
			if _, ok := ctIndex[ct]; !ok {
				acc.add(types.KindAgent, name, "custom_tools",
					fmt.Sprintf("unknown custom tool %q", ct))
			}
		}
	}
}

func checkAssets(m Materialized, acc *accumulator) {
	dsIndex := indexDatasources(m.Datasources)
	for _, as := range m.Assets {
		if as.Sink.DatasourceAPIName == "" {
			acc.add(types.KindAsset, string(as.APIName), "sink.datasource", "sink.datasource is required")
			continue
		}
		if _, ok := dsIndex[as.Sink.DatasourceAPIName]; !ok {
			acc.add(types.KindAsset, string(as.APIName), "sink.datasource",
				fmt.Sprintf("unknown datasource %q", as.Sink.DatasourceAPIName))
		}
		if as.Sink.Table == "" {
			acc.add(types.KindAsset, string(as.APIName), "sink.table", "sink.table is required")
		}
	}
}

// celEnv is the singleton CEL environment used by validators. It defines the
// minimal symbol set (actor, resource, input) so expressions can compile.
var celEnv = mustCELEnv()

func mustCELEnv() *cel.Env {
	env, err := cel.NewEnv(
		cel.Variable("actor", cel.DynType),
		cel.Variable("resource", cel.DynType),
		cel.Variable("input", cel.DynType),
		cel.Variable("subject", cel.DynType),
		cel.Variable("now", cel.DynType),
	)
	if err != nil {
		panic(fmt.Sprintf("ontology: cel env: %v", err))
	}
	return env
}

func compileCEL(expr string) (cel.Program, error) {
	if expr == "" {
		return nil, errors.New("expression is empty")
	}
	ast, iss := celEnv.Compile(expr)
	if iss != nil && iss.Err() != nil {
		return nil, fmt.Errorf("cel compile %q: %w", expr, iss.Err())
	}
	prg, err := celEnv.Program(ast)
	if err != nil {
		return nil, fmt.Errorf("cel program: %w", err)
	}
	return prg, nil
}
