// Builder constructs a graphql.Schema from a types.Ontology snapshot.

package graphql

import (
	"context"
	"fmt"

	"github.com/graphql-go/graphql"

	"github.com/miguelcsx/lattice/pkg/lattice/actions"
	"github.com/miguelcsx/lattice/pkg/lattice/query"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Builder constructs schemas from snapshots.
type Builder struct {
	queryPipeline  *query.Pipeline
	actionPipeline *actions.Pipeline
}

// NewBuilder returns a Builder.
func NewBuilder(q *query.Pipeline, a *actions.Pipeline) *Builder {
	return &Builder{queryPipeline: q, actionPipeline: a}
}

// Build returns the *graphql.Schema for snap.
func (b *Builder) Build(snap *types.Ontology) (graphql.Schema, error) {
	queryFields := graphql.Fields{}
	for _, ot := range snap.ObjectTypes {
		ot := ot
		objType := buildObjectType(ot)
		queryFields[fmt.Sprintf("search_%s", ot.APIName)] = &graphql.Field{
			Type: graphql.NewList(objType),
			Args: graphql.FieldConfigArgument{
				"limit": &graphql.ArgumentConfig{Type: graphql.Int},
			},
			Resolve: b.searchResolver(ot),
		}
		queryFields[fmt.Sprintf("get_%s", ot.APIName)] = &graphql.Field{
			Type: objType,
			Args: graphql.FieldConfigArgument{
				"primary_key": &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.String)},
			},
			Resolve: b.getResolver(ot),
		}
	}
	mutationFields := graphql.Fields{}
	for _, at := range snap.ActionTypes {
		at := at
		mutationFields[fmt.Sprintf("execute_%s", at.APIName)] = &graphql.Field{
			Type: graphql.NewObject(graphql.ObjectConfig{
				Name: fmt.Sprintf("ActionResult_%s", at.APIName),
				Fields: graphql.Fields{
					"run_id": &graphql.Field{Type: graphql.String},
					"status": &graphql.Field{Type: graphql.String},
				},
			}),
			Args:    graphql.FieldConfigArgument{"input": &graphql.ArgumentConfig{Type: graphql.String}},
			Resolve: b.executeResolver(at),
		}
	}
	return graphql.NewSchema(graphql.SchemaConfig{
		Query: graphql.NewObject(graphql.ObjectConfig{
			Name: "Query", Fields: queryFields,
		}),
		Mutation: graphql.NewObject(graphql.ObjectConfig{
			Name: "Mutation", Fields: mutationFields,
		}),
	})
}

func buildObjectType(ot types.ObjectType) *graphql.Object {
	fields := graphql.Fields{}
	for _, p := range ot.Properties {
		fields[string(p.APIName)] = &graphql.Field{Type: scalarFor(p.DataType)}
	}
	return graphql.NewObject(graphql.ObjectConfig{
		Name:        sanitizeName(string(ot.APIName)),
		Description: ot.Description,
		Fields:      fields,
	})
}

func scalarFor(dt types.DataType) graphql.Output {
	switch dt {
	case types.DataTypeInteger, types.DataTypeBigInt:
		return graphql.Int
	case types.DataTypeFloat, types.DataTypeDecimal:
		return graphql.Float
	case types.DataTypeBoolean:
		return graphql.Boolean
	default:
		return graphql.String
	}
}

func sanitizeName(s string) string {
	out := make([]byte, 0, len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		switch {
		case c == '.':
			out = append(out, '_')
		case c == '_' || (c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'):
			out = append(out, c)
		}
	}
	return string(out)
}

func (b *Builder) searchResolver(ot types.ObjectType) graphql.FieldResolveFn {
	return func(p graphql.ResolveParams) (any, error) {
		actor, ws, err := contextFromGraphQL(p.Context)
		if err != nil {
			return nil, err
		}
		limit := 50
		if v, ok := p.Args["limit"].(int); ok {
			limit = v
		}
		page, err := b.queryPipeline.Search(p.Context, query.SearchRequest{
			Actor: actor, WorkspaceID: ws, ObjectType: ot.APIName,
			Spec: types.QuerySpec{Page: types.PageSpec{Limit: limit}},
		})
		if err != nil {
			return nil, err
		}
		return recordsToMaps(page.Records), nil
	}
}

func (b *Builder) getResolver(ot types.ObjectType) graphql.FieldResolveFn {
	return func(p graphql.ResolveParams) (any, error) {
		actor, ws, err := contextFromGraphQL(p.Context)
		if err != nil {
			return nil, err
		}
		pk, _ := p.Args["primary_key"].(string)
		rec, err := b.queryPipeline.Get(p.Context, query.GetRequest{
			Actor: actor, WorkspaceID: ws, ObjectType: ot.APIName, PrimaryKey: pk,
		})
		if err != nil {
			return nil, err
		}
		return recordToMap(rec), nil
	}
}

func (b *Builder) executeResolver(at types.ActionType) graphql.FieldResolveFn {
	return func(p graphql.ResolveParams) (any, error) {
		actor, ws, err := contextFromGraphQL(p.Context)
		if err != nil {
			return nil, err
		}
		var input map[string]any
		if v, ok := p.Args["input"].(string); ok && v != "" {
			input = map[string]any{"raw": v}
		}
		res, err := b.actionPipeline.Execute(p.Context, actions.ExecuteRequest{
			Actor: actor, WorkspaceID: ws, ActionTypeName: at.APIName, Input: input,
		})
		if err != nil {
			return nil, err
		}
		return map[string]any{"run_id": string(res.RunID), "status": string(res.Status)}, nil
	}
}

// ContextKey is the key for the actor + workspace passed via context.
type ContextKey string

const (
	CtxKeyActor       ContextKey = "graphql.actor"
	CtxKeyWorkspaceID ContextKey = "graphql.workspace_id"
)

func contextFromGraphQL(ctx context.Context) (types.Actor, types.WorkspaceID, error) {
	actor, ok := ctx.Value(CtxKeyActor).(types.Actor)
	if !ok {
		return types.Actor{}, "", fmt.Errorf("graphql: actor missing from context")
	}
	ws, ok := ctx.Value(CtxKeyWorkspaceID).(types.WorkspaceID)
	if !ok {
		return types.Actor{}, "", fmt.Errorf("graphql: workspace missing from context")
	}
	return actor, ws, nil
}

func recordToMap(r types.Record) map[string]any {
	out := make(map[string]any, len(r.Values))
	for k, v := range r.Values {
		out[string(k)] = v
	}
	return out
}

func recordsToMaps(in []types.Record) []map[string]any {
	out := make([]map[string]any, len(in))
	for i, r := range in {
		out[i] = recordToMap(r)
	}
	return out
}
