// Snapshot diff and merge. Compares two ontology snapshots by api_name
// and produces a structural Diff. Merge is last-write-wins.

package branch

import (
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// DiffSnapshots returns the changes from `from` to `to`.
func DiffSnapshots(from, to types.Ontology) types.Diff {
	out := types.Diff{}
	out = diffObjectTypes(from, to, out)
	out = diffLinkTypes(from, to, out)
	out = diffActionTypes(from, to, out)
	out = diffPolicyRules(from, to, out)
	return out
}

func diffObjectTypes(from, to types.Ontology, d types.Diff) types.Diff {
	idx := indexObjectTypes(from)
	seen := make(map[types.APIName]bool, len(to.ObjectTypes))
	for _, ot := range to.ObjectTypes {
		seen[ot.APIName] = true
		if old, ok := idx[ot.APIName]; ok {
			if !objectTypeEqual(old, ot) {
				d.Updated = append(d.Updated, types.DiffEntry{
					Kind: types.KindObjectType, APIName: ot.APIName,
				})
			}
			continue
		}
		d.Created = append(d.Created, types.DiffEntry{
			Kind: types.KindObjectType, APIName: ot.APIName,
		})
	}
	for name := range idx {
		if !seen[name] {
			d.Deleted = append(d.Deleted, types.DiffEntry{
				Kind: types.KindObjectType, APIName: name,
			})
		}
	}
	return d
}

func diffLinkTypes(from, to types.Ontology, d types.Diff) types.Diff {
	idx := make(map[types.APIName]types.LinkType, len(from.LinkTypes))
	for _, lt := range from.LinkTypes {
		idx[lt.APIName] = lt
	}
	seen := make(map[types.APIName]bool, len(to.LinkTypes))
	for _, lt := range to.LinkTypes {
		seen[lt.APIName] = true
		if _, ok := idx[lt.APIName]; !ok {
			d.Created = append(d.Created, types.DiffEntry{
				Kind: types.KindLinkType, APIName: lt.APIName,
			})
		}
	}
	for name := range idx {
		if !seen[name] {
			d.Deleted = append(d.Deleted, types.DiffEntry{
				Kind: types.KindLinkType, APIName: name,
			})
		}
	}
	return d
}

func diffActionTypes(from, to types.Ontology, d types.Diff) types.Diff {
	idx := make(map[types.APIName]types.ActionType, len(from.ActionTypes))
	for _, at := range from.ActionTypes {
		idx[at.APIName] = at
	}
	seen := make(map[types.APIName]bool, len(to.ActionTypes))
	for _, at := range to.ActionTypes {
		seen[at.APIName] = true
		if _, ok := idx[at.APIName]; !ok {
			d.Created = append(d.Created, types.DiffEntry{
				Kind: types.KindActionType, APIName: at.APIName,
			})
		}
	}
	for name := range idx {
		if !seen[name] {
			d.Deleted = append(d.Deleted, types.DiffEntry{
				Kind: types.KindActionType, APIName: name,
			})
		}
	}
	return d
}

func diffPolicyRules(from, to types.Ontology, d types.Diff) types.Diff {
	idx := make(map[types.APIName]types.PolicyRule, len(from.PolicyRules))
	for _, p := range from.PolicyRules {
		idx[p.APIName] = p
	}
	seen := make(map[types.APIName]bool, len(to.PolicyRules))
	for _, p := range to.PolicyRules {
		seen[p.APIName] = true
		if _, ok := idx[p.APIName]; !ok {
			d.Created = append(d.Created, types.DiffEntry{
				Kind: types.KindPolicyRule, APIName: p.APIName,
			})
		}
	}
	for name := range idx {
		if !seen[name] {
			d.Deleted = append(d.Deleted, types.DiffEntry{
				Kind: types.KindPolicyRule, APIName: name,
			})
		}
	}
	return d
}

// MergeSnapshots applies last-write-wins of src onto dst. New types in src
// are appended; existing types are replaced.
func MergeSnapshots(dst, src types.Ontology) types.Ontology {
	dst.ObjectTypes = mergeOTs(dst.ObjectTypes, src.ObjectTypes)
	dst.LinkTypes = mergeLTs(dst.LinkTypes, src.LinkTypes)
	dst.ActionTypes = mergeATs(dst.ActionTypes, src.ActionTypes)
	dst.PolicyRules = mergePRs(dst.PolicyRules, src.PolicyRules)
	return dst
}

func mergeOTs(dst, src []types.ObjectType) []types.ObjectType {
	idx := make(map[types.APIName]int, len(dst))
	for i, ot := range dst {
		idx[ot.APIName] = i
	}
	for _, ot := range src {
		if i, ok := idx[ot.APIName]; ok {
			dst[i] = ot
		} else {
			dst = append(dst, ot)
			idx[ot.APIName] = len(dst) - 1
		}
	}
	return dst
}

func mergeLTs(dst, src []types.LinkType) []types.LinkType {
	idx := make(map[types.APIName]int, len(dst))
	for i, lt := range dst {
		idx[lt.APIName] = i
	}
	for _, lt := range src {
		if i, ok := idx[lt.APIName]; ok {
			dst[i] = lt
		} else {
			dst = append(dst, lt)
			idx[lt.APIName] = len(dst) - 1
		}
	}
	return dst
}

func mergeATs(dst, src []types.ActionType) []types.ActionType {
	idx := make(map[types.APIName]int, len(dst))
	for i, at := range dst {
		idx[at.APIName] = i
	}
	for _, at := range src {
		if i, ok := idx[at.APIName]; ok {
			dst[i] = at
		} else {
			dst = append(dst, at)
			idx[at.APIName] = len(dst) - 1
		}
	}
	return dst
}

func mergePRs(dst, src []types.PolicyRule) []types.PolicyRule {
	idx := make(map[types.APIName]int, len(dst))
	for i, p := range dst {
		idx[p.APIName] = i
	}
	for _, p := range src {
		if i, ok := idx[p.APIName]; ok {
			dst[i] = p
		} else {
			dst = append(dst, p)
			idx[p.APIName] = len(dst) - 1
		}
	}
	return dst
}

func indexObjectTypes(o types.Ontology) map[types.APIName]types.ObjectType {
	idx := make(map[types.APIName]types.ObjectType, len(o.ObjectTypes))
	for _, ot := range o.ObjectTypes {
		idx[ot.APIName] = ot
	}
	return idx
}

func objectTypeEqual(a, b types.ObjectType) bool {
	if a.APIName != b.APIName || a.PrimaryKey != b.PrimaryKey || len(a.Properties) != len(b.Properties) {
		return false
	}
	idx := make(map[types.APIName]types.Property, len(a.Properties))
	for _, p := range a.Properties {
		idx[p.APIName] = p
	}
	for _, p := range b.Properties {
		old, ok := idx[p.APIName]
		if !ok {
			return false
		}
		if old.DataType != p.DataType || old.Indexed != p.Indexed || old.Nullable != p.Nullable {
			return false
		}
	}
	return true
}
