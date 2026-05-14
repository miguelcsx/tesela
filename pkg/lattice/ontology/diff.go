// Diff computes the structural delta between two Materialized ontologies.
// The diff is shallow: an updated object type is reported once, not per
// property. Detailed per-property diffs can be derived by callers from the
// Created/Updated/Deleted entries.

package ontology

import (
	"reflect"
	"sort"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// DiffMaterialized returns a types.Diff describing what changed going from old
// to new. old may be the zero value when the workspace is being initialized
// for the first time.
func DiffMaterialized(old, new Materialized) types.Diff {
	d := types.Diff{}
	addDiffEntries(types.KindDatasource, named(old.Datasources), named(new.Datasources), &d)
	addDiffEntries(types.KindObjectType, named(old.ObjectTypes), named(new.ObjectTypes), &d)
	addDiffEntries(types.KindLinkType, named(old.LinkTypes), named(new.LinkTypes), &d)
	addDiffEntries(types.KindActionType, named(old.ActionTypes), named(new.ActionTypes), &d)
	addDiffEntries(types.KindRole, named(old.Roles), named(new.Roles), &d)
	addDiffEntries(types.KindPolicyRule, named(old.PolicyRules), named(new.PolicyRules), &d)
	addDiffEntries(types.KindCustomTool, named(old.CustomTools), named(new.CustomTools), &d)
	addDiffEntries(types.KindAgent, named(old.Agents), named(new.Agents), &d)
	addDiffEntries(types.KindAsset, named(old.Assets), named(new.Assets), &d)
	return d
}

// named groups any slice of named entities into a name -> any map that the
// generic differ can compare. The values are stored as `any` because the
// differ only uses reflect.DeepEqual on them.
func named[T any](in []T) map[types.APIName]any {
	out := make(map[types.APIName]any, len(in))
	for _, e := range in {
		name := apiNameOf(e)
		out[name] = e
	}
	return out
}

func apiNameOf(v any) types.APIName {
	switch x := v.(type) {
	case types.Datasource:
		return x.APIName
	case types.ObjectType:
		return x.APIName
	case types.LinkType:
		return x.APIName
	case types.ActionType:
		return x.APIName
	case types.Role:
		return x.APIName
	case types.PolicyRule:
		return x.APIName
	case types.CustomTool:
		return x.APIName
	case types.Agent:
		return x.APIName
	case types.Asset:
		return x.APIName
	default:
		return ""
	}
}

func addDiffEntries(kind types.Kind, oldM, newM map[types.APIName]any, d *types.Diff) {
	allNames := make(map[types.APIName]struct{}, len(oldM)+len(newM))
	for n := range oldM {
		allNames[n] = struct{}{}
	}
	for n := range newM {
		allNames[n] = struct{}{}
	}
	names := sortedNames(allNames)
	for _, n := range names {
		oldVal, oldHas := oldM[n]
		newVal, newHas := newM[n]
		switch {
		case !oldHas && newHas:
			d.Created = append(d.Created, types.DiffEntry{Kind: kind, APIName: n})
		case oldHas && !newHas:
			d.Deleted = append(d.Deleted, types.DiffEntry{Kind: kind, APIName: n})
		case !equalIgnoringTimestamps(oldVal, newVal):
			d.Updated = append(d.Updated, types.DiffEntry{Kind: kind, APIName: n})
		}
	}
}

func sortedNames(set map[types.APIName]struct{}) []types.APIName {
	out := make([]types.APIName, 0, len(set))
	for n := range set {
		out = append(out, n)
	}
	sort.Slice(out, func(i, j int) bool { return out[i] < out[j] })
	return out
}

// equalIgnoringTimestamps treats two entities as equal when every field
// except IDs/CreatedAt/UpdatedAt/Version matches. We zero those fields on a
// copy to keep the comparison purely structural.
func equalIgnoringTimestamps(a, b any) bool {
	return reflect.DeepEqual(stripVolatile(a), stripVolatile(b))
}

func stripVolatile(v any) any {
	z := time.Time{}
	switch x := v.(type) {
	case types.Datasource:
		x.CreatedAt, x.UpdatedAt, x.ID, x.SealedCredentials = z, z, "", nil
		return x
	case types.ObjectType:
		x.CreatedAt, x.UpdatedAt, x.ID, x.Version = z, z, "", 0
		return x
	case types.LinkType:
		x.CreatedAt, x.UpdatedAt, x.ID, x.Version = z, z, "", 0
		return x
	case types.ActionType:
		x.CreatedAt, x.UpdatedAt, x.ID, x.Version = z, z, "", 0
		return x
	case types.Role:
		x.CreatedAt, x.UpdatedAt, x.ID = z, z, ""
		return x
	case types.PolicyRule:
		x.CreatedAt, x.UpdatedAt, x.ID = z, z, ""
		return x
	case types.CustomTool:
		x.CreatedAt, x.UpdatedAt, x.ID = z, z, ""
		return x
	case types.Agent:
		x.CreatedAt, x.UpdatedAt, x.ID = z, z, ""
		return x
	case types.Asset:
		x.CreatedAt, x.UpdatedAt, x.ID = z, z, ""
		return x
	default:
		return v
	}
}
