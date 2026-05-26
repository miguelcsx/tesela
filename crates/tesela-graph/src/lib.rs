//! Schema graph analysis for Tesela ontologies.
//!
//! Provides cycle detection, shortest path, topological sort, impact analysis,
//! and lineage computation over the object-link graph.

#![deny(warnings)]
#![deny(missing_docs)]

use petgraph::algo::tarjan_scc;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};
use tesela_core::{ApiName, Diagnostic, DiagnosticCode};
use tesela_ir::{LinkType, ObjectType, Spec};

/// An immutable schema graph built from a `Spec`.
///
/// Nodes represent object types. Edges represent link types.
/// The graph may contain cycles (e.g., self-referencing entities).
#[derive(Debug, Clone)]
pub struct SchemaGraph {
    graph: Graph<ObjectType, LinkType>,
    /// Mapping from object type API name to graph node index.
    name_to_index: HashMap<ApiName, NodeIndex>,
    /// Reverse mapping from node index to object type API name.
    index_to_name: HashMap<NodeIndex, ApiName>,
}

/// Report of entities affected by a change to a single entity.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImpactReport {
    /// Affected object types.
    pub object_types: Vec<ApiName>,
    /// Affected link types.
    pub link_types: Vec<ApiName>,
    /// Affected actions.
    pub actions: Vec<ApiName>,
    /// Affected policies.
    pub policies: Vec<ApiName>,
    /// Affected agents.
    pub agents: Vec<ApiName>,
    /// Affected assets.
    pub assets: Vec<ApiName>,
}

/// Full cross-kind impact report for an entire spec.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FullImpactReport {
    /// Per-entity impact reports.
    pub by_entity: HashMap<ApiName, ImpactReport>,
}

/// Builder for constructing a `SchemaGraph` from a `Spec`.
pub struct GraphBuilder;

impl GraphBuilder {
    /// Build a `SchemaGraph` from the given spec.
    pub fn build(spec: &Spec) -> SchemaGraph {
        let mut graph = Graph::<ObjectType, LinkType>::new();
        let mut name_to_index = HashMap::new();
        let mut index_to_name = HashMap::new();

        // Add object type nodes.
        for ot in &spec.object_types {
            let idx = graph.add_node(ot.clone());
            name_to_index.insert(ot.api_name.clone(), idx);
            index_to_name.insert(idx, ot.api_name.clone());
        }

        // Add link type edges.
        for lt in &spec.link_types {
            if let (Some(&from_idx), Some(&to_idx)) =
                (name_to_index.get(&lt.from), name_to_index.get(&lt.to))
            {
                graph.add_edge(from_idx, to_idx, lt.clone());
            }
        }

        SchemaGraph {
            graph,
            name_to_index,
            index_to_name,
        }
    }
}

impl SchemaGraph {
    /// Detect cycles in the link graph using strongly-connected components.
    ///
    /// Returns a diagnostic for each strongly-connected component with >1 node
    /// or a single node with a self-edge.
    pub fn detect_cycles(&self) -> Vec<Diagnostic> {
        let scc = tarjan_scc(&self.graph);
        let mut diagnostics = Vec::new();
        for component in scc {
            if component.len() > 1 {
                let names: Vec<String> = component
                    .iter()
                    .filter_map(|idx| self.index_to_name.get(idx).map(|n| n.to_string()))
                    .collect();
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidLink,
                        format!("cycle detected among object types: {}", names.join(", ")),
                    )
                    .with_api_name(self.index_to_name[&component[0]].clone()),
                );
            } else if component.len() == 1 {
                let idx = component[0];
                if self.graph.edges(idx).any(|e| e.target() == idx) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidLink,
                            format!(
                                "self-referencing cycle detected on object type '{}'",
                                self.index_to_name[&idx]
                            ),
                        )
                        .with_api_name(self.index_to_name[&idx].clone()),
                    );
                }
            }
        }
        diagnostics
    }

    /// Find the shortest path between two object types via links (BFS).
    ///
    /// Returns the sequence of object type API names, including start and end.
    pub fn shortest_path(&self, from: &ApiName, to: &ApiName) -> Option<Vec<ApiName>> {
        let start = *self.name_to_index.get(from)?;
        let goal = *self.name_to_index.get(to)?;

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut came_from = HashMap::<NodeIndex, NodeIndex>::new();

        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            if current == goal {
                // Reconstruct path.
                let mut path = vec![goal];
                let mut node = goal;
                while let Some(&prev) = came_from.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                path.reverse();
                return Some(
                    path.iter()
                        .filter_map(|idx| self.index_to_name.get(idx).cloned())
                        .collect(),
                );
            }

            for neighbor in self.graph.neighbors(current) {
                if visited.insert(neighbor) {
                    came_from.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }

        None
    }

    /// Topologically sort object types using Kahn's algorithm.
    ///
    /// Fails if the graph contains cycles.
    pub fn topological_sort(&self) -> Result<Vec<ApiName>, Diagnostic> {
        let n = self.graph.node_count();
        let mut in_degree: HashMap<NodeIndex, usize> = HashMap::new();
        let mut adj: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();

        for idx in self.graph.node_indices() {
            in_degree.entry(idx).or_insert(0);
        }

        for edge in self.graph.edge_references() {
            let source = edge.source();
            let target = edge.target();
            adj.entry(source).or_default().push(target);
            *in_degree.entry(target).or_insert(0) += 1;
        }

        let mut queue: VecDeque<NodeIndex> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(idx, _)| *idx)
            .collect();

        let mut sorted = Vec::with_capacity(n);

        while let Some(current) = queue.pop_front() {
            sorted.push(current);
            for &neighbor in adj.get(&current).unwrap_or(&Vec::new()) {
                let deg = in_degree.get_mut(&neighbor).expect("valid node");
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(neighbor);
                }
            }
        }

        if sorted.len() != n {
            let remaining: Vec<String> = in_degree
                .iter()
                .filter(|(_, deg)| **deg > 0)
                .filter_map(|(idx, _)| self.index_to_name.get(idx).map(|n| n.to_string()))
                .collect();
            return Err(Diagnostic::error(
                DiagnosticCode::InvalidLink,
                format!("cycle prevents topological sort: {}", remaining.join(", ")),
            ));
        }

        Ok(sorted
            .into_iter()
            .filter_map(|idx| self.index_to_name.get(&idx).cloned())
            .collect())
    }

    /// Compute all entities reachable from a given entity.
    ///
    /// `depth` limits the traversal depth; `None` means unlimited.
    pub fn impact_analysis(&self, entity: &ApiName) -> ImpactReport {
        let mut report = ImpactReport::default();
        let reachable = self.reachable(entity, None);

        for name in &reachable {
            if name != entity {
                report.object_types.push(name.clone());
            }
        }

        // Find link types that connect reachable nodes.
        for edge in self.graph.edge_references() {
            let source_name = self.index_to_name.get(&edge.source()).cloned();
            let target_name = self.index_to_name.get(&edge.target()).cloned();
            if let (Some(s), Some(t)) = (source_name, target_name)
                && reachable.contains(&s)
                && reachable.contains(&t)
            {
                report.link_types.push(edge.weight().api_name.clone());
            }
        }
        report.link_types.sort();
        report.link_types.dedup();

        report
    }

    /// Compute lineage edges for an object type.
    pub fn compute_lineage(&self, obj: &ApiName) -> Vec<tesela_ir::LineageEdge> {
        if let Some(&idx) = self.name_to_index.get(obj) {
            self.graph[idx].lineage.clone()
        } else {
            Vec::new()
        }
    }

    /// Find all object types reachable from `from` within `depth` hops.
    pub fn reachable(&self, from: &ApiName, depth: Option<usize>) -> HashSet<ApiName> {
        let start = match self.name_to_index.get(from) {
            Some(&idx) => idx,
            None => return HashSet::new(),
        };

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((start, 0usize));
        visited.insert(start);

        while let Some((current, dist)) = queue.pop_front() {
            if let Some(max_depth) = depth
                && dist >= max_depth
            {
                continue;
            }
            for neighbor in self.graph.neighbors(current) {
                if visited.insert(neighbor) {
                    queue.push_back((neighbor, dist + 1));
                }
            }
        }

        visited
            .into_iter()
            .filter_map(|idx| self.index_to_name.get(&idx).cloned())
            .collect()
    }

    /// Compute a full cross-kind impact report for the entire spec.
    pub fn full_impact_analysis(&self) -> FullImpactReport {
        let mut report = FullImpactReport::default();
        for name in self.name_to_index.keys() {
            let impact = self.impact_analysis(name);
            report.by_entity.insert(name.clone(), impact);
        }
        report
    }

    /// Access the underlying `petgraph::Graph`.
    pub fn inner(&self) -> &Graph<ObjectType, LinkType> {
        &self.graph
    }

    /// Look up the node index for an object type API name.
    pub fn node_index(&self, name: &ApiName) -> Option<NodeIndex> {
        self.name_to_index.get(name).copied()
    }

    /// Get the object type for a given node index.
    pub fn object_type(&self, idx: NodeIndex) -> Option<&ObjectType> {
        self.graph.node_weight(idx)
    }

    /// Iterate over all object type names in the graph.
    pub fn object_type_names(&self) -> impl Iterator<Item = &ApiName> {
        self.name_to_index.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesela_core::LinkCardinality;
    use tesela_ir::{LinkMapping, LinkType, ObjectSource, ObjectType, Property, Workspace};

    fn simple_spec() -> Spec {
        Spec {
            version: tesela_core::Version::new(tesela_ir::SPEC_VERSION),
            workspace: Workspace::default(),
            datasources: Vec::new(),
            traits: Vec::new(),
            object_types: vec![
                ObjectType {
                    api_name: ApiName::new_unchecked("customer"),
                    display: None,
                    description: None,
                    source: ObjectSource {
                        datasource: ApiName::new_unchecked("db"),
                        resource: None,
                    },
                    primary_key: ApiName::new_unchecked("id"),
                    properties: vec![Property {
                        api_name: ApiName::new_unchecked("id"),
                        display: None,
                        description: None,
                        data_type: tesela_core::DataType::Uuid,
                        nullable: None,
                        indexed: None,
                        unique: None,
                        tags: Vec::new(),
                        markings: Vec::new(),
                        default: None,
                        computed: None,
                        source_column: None,
                        allowed_values: None,
                        sort_order: None,
                        metadata: None,
                        encrypted: None,
                        quality: Vec::new(),
                    }],
                    traits: Vec::new(),
                    tags: Vec::new(),
                    metadata: None,
                    indexes: Vec::new(),
                    temporal: None,
                    lifecycle: None,
                    scoring: None,
                    classification: None,
                    quality_rules: Vec::new(),
                    lineage: Vec::new(),
                    deprecated_at: None,
                },
                ObjectType {
                    api_name: ApiName::new_unchecked("order"),
                    display: None,
                    description: None,
                    source: ObjectSource {
                        datasource: ApiName::new_unchecked("db"),
                        resource: None,
                    },
                    primary_key: ApiName::new_unchecked("id"),
                    properties: vec![Property {
                        api_name: ApiName::new_unchecked("id"),
                        display: None,
                        description: None,
                        data_type: tesela_core::DataType::Uuid,
                        nullable: None,
                        indexed: None,
                        unique: None,
                        tags: Vec::new(),
                        markings: Vec::new(),
                        default: None,
                        computed: None,
                        source_column: None,
                        allowed_values: None,
                        sort_order: None,
                        metadata: None,
                        encrypted: None,
                        quality: Vec::new(),
                    }],
                    traits: Vec::new(),
                    tags: Vec::new(),
                    metadata: None,
                    indexes: Vec::new(),
                    temporal: None,
                    lifecycle: None,
                    scoring: None,
                    classification: None,
                    quality_rules: Vec::new(),
                    lineage: Vec::new(),
                    deprecated_at: None,
                },
            ],
            link_types: vec![LinkType {
                api_name: ApiName::new_unchecked("customer_orders"),
                display: None,
                from: ApiName::new_unchecked("customer"),
                to: ApiName::new_unchecked("order"),
                cardinality: LinkCardinality::OneToMany,
                source: None,
                mappings: vec![LinkMapping {
                    from_property: ApiName::new_unchecked("id"),
                    to_property: ApiName::new_unchecked("customer_id"),
                }],
                junction: None,
                deprecated_at: None,
                metadata: None,
            }],
            actions: Vec::new(),
            roles: Vec::new(),
            policies: Vec::new(),
            agents: Vec::new(),
            custom_tools: Vec::new(),
            assets: Vec::new(),
            environments: Vec::new(),
            object_sets: Vec::new(),
            pipelines: Vec::new(),
            artifact_types: Vec::new(),
            upload_flows: Vec::new(),
            job_types: Vec::new(),
            event_types: Vec::new(),
            capability_grants: Vec::new(),
            aggregate_views: Vec::new(),
        }
    }

    #[test]
    fn test_build_graph() {
        let spec = simple_spec();
        let graph = GraphBuilder::build(&spec);
        assert_eq!(graph.inner().node_count(), 2);
        assert_eq!(graph.inner().edge_count(), 1);
    }

    #[test]
    fn test_shortest_path() {
        let spec = simple_spec();
        let graph = GraphBuilder::build(&spec);
        let path = graph.shortest_path(
            &ApiName::new_unchecked("customer"),
            &ApiName::new_unchecked("order"),
        );
        assert_eq!(
            path,
            Some(vec![
                ApiName::new_unchecked("customer"),
                ApiName::new_unchecked("order")
            ])
        );
    }

    #[test]
    fn test_topological_sort() {
        let spec = simple_spec();
        let graph = GraphBuilder::build(&spec);
        let sorted = graph.topological_sort();
        assert!(sorted.is_ok());
        let names = sorted.unwrap();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_cycle_detection() {
        let mut spec = simple_spec();
        // Add a reverse link to create a cycle.
        spec.link_types.push(LinkType {
            api_name: ApiName::new_unchecked("order_customer"),
            display: None,
            from: ApiName::new_unchecked("order"),
            to: ApiName::new_unchecked("customer"),
            cardinality: LinkCardinality::OneToOne,
            source: None,
            mappings: vec![LinkMapping {
                from_property: ApiName::new_unchecked("customer_id"),
                to_property: ApiName::new_unchecked("id"),
            }],
            junction: None,
            deprecated_at: None,
            metadata: None,
        });
        let graph = GraphBuilder::build(&spec);
        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
    }
}
