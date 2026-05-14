package graph

import (
	"fmt"
	"slices"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// SchemaGraph is the ontology-level graph extracted from object and link
// definitions. It is purely declarative and does not depend on any runtime
// backend implementation.
type SchemaGraph struct {
	ontology types.Ontology
	outgoing map[types.APIName][]types.LinkType
	incoming map[types.APIName][]types.LinkType
}

// BuildSchemaGraph constructs the ontology graph index once so callers can run
// path finding, cycle detection, and impact analysis efficiently.
func BuildSchemaGraph(o types.Ontology) *SchemaGraph {
	g := &SchemaGraph{
		ontology: o,
		outgoing: make(map[types.APIName][]types.LinkType, len(o.ObjectTypes)),
		incoming: make(map[types.APIName][]types.LinkType, len(o.ObjectTypes)),
	}
	for _, lt := range o.LinkTypes {
		g.outgoing[lt.FromObjectType] = append(g.outgoing[lt.FromObjectType], lt)
		g.incoming[lt.ToObjectType] = append(g.incoming[lt.ToObjectType], lt)
	}
	return g
}

// Path is a multi-hop relationship path over the schema graph.
type Path struct {
	From  types.APIName    `json:"from"`
	To    types.APIName    `json:"to"`
	Links []types.LinkType `json:"links"`
	Hops  []types.APIName  `json:"hops"`
}

// LineageEdge is a declarative provenance edge between two schema nodes.
type LineageEdge struct {
	From        string `json:"from"`
	To          string `json:"to"`
	Kind        string `json:"kind"`
	Description string `json:"description,omitempty"`
}

// DependencyReport is the transitive closure of upstream/downstream neighbors
// for one schema node.
type DependencyReport struct {
	Node       string         `json:"node"`
	Upstream   []string       `json:"upstream,omitempty"`
	Downstream []string       `json:"downstream,omitempty"`
	Metadata   map[string]any `json:"metadata,omitempty"`
}

func (g *SchemaGraph) outgoingLinks(from types.APIName) []types.LinkType {
	return g.outgoing[from]
}

// ShortestPath returns the fewest-hop path between two object types.
func (g *SchemaGraph) ShortestPath(from, to types.APIName) (Path, bool) {
	if from == to {
		return Path{From: from, To: to, Hops: []types.APIName{from}}, true
	}
	type state struct {
		node types.APIName
		path Path
	}
	queue := []state{{node: from, path: Path{From: from, To: to, Hops: []types.APIName{from}}}}
	seen := map[types.APIName]bool{from: true}
	for len(queue) > 0 {
		cur := queue[0]
		queue = queue[1:]
		for _, lt := range g.outgoingLinks(cur.node) {
			next := lt.ToObjectType
			if seen[next] {
				continue
			}
			path := Path{
				From:  from,
				To:    to,
				Links: append(slices.Clone(cur.path.Links), lt),
				Hops:  append(slices.Clone(cur.path.Hops), next),
			}
			if next == to {
				return path, true
			}
			seen[next] = true
			queue = append(queue, state{node: next, path: path})
		}
	}
	return Path{}, false
}

// Paths enumerates every acyclic path up to maxDepth hops.
func (g *SchemaGraph) Paths(from, to types.APIName, maxDepth int) []Path {
	if maxDepth <= 0 {
		return nil
	}
	var out []Path
	var walk func(node types.APIName, path Path, seen map[types.APIName]bool)
	walk = func(node types.APIName, path Path, seen map[types.APIName]bool) {
		if len(path.Links) >= maxDepth {
			return
		}
		for _, lt := range g.outgoingLinks(node) {
			next := lt.ToObjectType
			if seen[next] {
				continue
			}
			nextPath := Path{
				From:  from,
				To:    to,
				Links: append(slices.Clone(path.Links), lt),
				Hops:  append(slices.Clone(path.Hops), next),
			}
			if next == to {
				out = append(out, nextPath)
				continue
			}
			nextSeen := cloneSeen(seen)
			nextSeen[next] = true
			walk(next, nextPath, nextSeen)
		}
	}
	walk(from, Path{From: from, To: to, Hops: []types.APIName{from}}, map[types.APIName]bool{from: true})
	return out
}

// Cycles returns every simple cycle discovered in the schema graph.
func (g *SchemaGraph) Cycles() [][]types.APIName {
	var out [][]types.APIName
	var dfs func(start, node types.APIName, stack []types.APIName, seen map[types.APIName]bool)
	dfs = func(start, node types.APIName, stack []types.APIName, seen map[types.APIName]bool) {
		for _, lt := range g.outgoingLinks(node) {
			next := lt.ToObjectType
			if next == start {
				out = append(out, append(slices.Clone(stack), start))
				continue
			}
			if seen[next] {
				continue
			}
			nextSeen := cloneSeen(seen)
			nextSeen[next] = true
			dfs(start, next, append(slices.Clone(stack), next), nextSeen)
		}
	}
	for _, ot := range g.ontology.ObjectTypes {
		dfs(ot.APIName, ot.APIName, []types.APIName{ot.APIName}, map[types.APIName]bool{ot.APIName: true})
	}
	return dedupeCycles(out)
}

// ImpactAnalysis returns upstream and downstream schema dependencies for one
// object type.
func (g *SchemaGraph) ImpactAnalysis(object types.APIName) DependencyReport {
	return DependencyReport{
		Node:       string(object),
		Upstream:   g.reachableIncoming(object),
		Downstream: g.reachableOutgoing(object),
	}
}

func (g *SchemaGraph) reachableOutgoing(start types.APIName) []string {
	return g.reachable(start, func(node types.APIName) []types.LinkType { return g.outgoingLinks(node) }, func(lt types.LinkType) types.APIName { return lt.ToObjectType })
}

func (g *SchemaGraph) reachableIncoming(start types.APIName) []string {
	return g.reachable(start, func(node types.APIName) []types.LinkType { return g.incoming[node] }, func(lt types.LinkType) types.APIName { return lt.FromObjectType })
}

func (g *SchemaGraph) reachable(start types.APIName, links func(types.APIName) []types.LinkType, nextNode func(types.LinkType) types.APIName) []string {
	queue := []types.APIName{start}
	seen := map[types.APIName]bool{start: true}
	var out []string
	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]
		for _, lt := range links(node) {
			next := nextNode(lt)
			if seen[next] {
				continue
			}
			seen[next] = true
			out = append(out, string(next))
			queue = append(queue, next)
		}
	}
	slices.Sort(out)
	return out
}

// LineageEdges emits schema-level provenance edges for sourced properties,
// explicit computed-property dependencies, and asset dependencies.
func (g *SchemaGraph) LineageEdges() []LineageEdge {
	var edges []LineageEdge
	for _, ot := range g.ontology.ObjectTypes {
		for _, prop := range ot.Properties {
			propNode := propertyNode(ot.APIName, prop.APIName)
			if src := prop.ResolvedSourceColumn(); src != "" {
				edges = append(edges, LineageEdge{
					From:        fmt.Sprintf("%s.%s", ot.Source.Table, src),
					To:          propNode,
					Kind:        "source_column",
					Description: "declared source column",
				})
			}
			if prop.Computed != nil {
				for _, dep := range prop.Computed.DependsOn {
					edges = append(edges, LineageEdge{
						From:        propertyNode(ot.APIName, dep),
						To:          propNode,
						Kind:        "computed_dependency",
						Description: prop.Computed.Expression,
					})
				}
			}
		}
	}
	for _, asset := range g.ontology.Assets {
		for _, dep := range asset.Dependencies {
			edges = append(edges, LineageEdge{
				From:        dep.Target,
				To:          string(asset.APIName),
				Kind:        dep.Kind,
				Description: dep.Description,
			})
		}
	}
	return edges
}

func propertyNode(object, property types.APIName) string {
	return fmt.Sprintf("%s.%s", object, property)
}

func cloneSeen(in map[types.APIName]bool) map[types.APIName]bool {
	out := make(map[types.APIName]bool, len(in))
	for k, v := range in {
		out[k] = v
	}
	return out
}

func dedupeCycles(cycles [][]types.APIName) [][]types.APIName {
	seen := make(map[string]bool, len(cycles))
	var out [][]types.APIName
	for _, cycle := range cycles {
		key := canonicalCycleKey(cycle)
		if seen[key] {
			continue
		}
		seen[key] = true
		out = append(out, cycle)
	}
	return out
}

func canonicalCycleKey(cycle []types.APIName) string {
	if len(cycle) == 0 {
		return ""
	}
	best := ""
	for i := range cycle[:len(cycle)-1] {
		var key string
		for j := 0; j < len(cycle)-1; j++ {
			if j > 0 {
				key += "->"
			}
			key += string(cycle[(i+j)%(len(cycle)-1)])
		}
		if best == "" || key < best {
			best = key
		}
	}
	return best
}
