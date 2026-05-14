// Indexers turn entity slices into name->entity maps. They keep the
// validator readable and avoid quadratic loops.

package ontology

import "github.com/miguelcsx/lattice/pkg/lattice/types"

func indexDatasources(in []types.Datasource) map[types.APIName]types.Datasource {
	out := make(map[types.APIName]types.Datasource, len(in))
	for _, d := range in {
		out[d.APIName] = d
	}
	return out
}

func indexObjectTypes(in []types.ObjectType) map[types.APIName]types.ObjectType {
	out := make(map[types.APIName]types.ObjectType, len(in))
	for _, o := range in {
		out[o.APIName] = o
	}
	return out
}

func indexLinkTypes(in []types.LinkType) map[types.APIName]types.LinkType {
	out := make(map[types.APIName]types.LinkType, len(in))
	for _, l := range in {
		out[l.APIName] = l
	}
	return out
}

func indexActionTypes(in []types.ActionType) map[types.APIName]types.ActionType {
	out := make(map[types.APIName]types.ActionType, len(in))
	for _, a := range in {
		out[a.APIName] = a
	}
	return out
}

func indexRoles(in []types.Role) map[types.APIName]types.Role {
	out := make(map[types.APIName]types.Role, len(in))
	for _, r := range in {
		out[r.APIName] = r
	}
	return out
}

func indexCustomTools(in []types.CustomTool) map[types.APIName]types.CustomTool {
	out := make(map[types.APIName]types.CustomTool, len(in))
	for _, c := range in {
		out[c.APIName] = c
	}
	return out
}
