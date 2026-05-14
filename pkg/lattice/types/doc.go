// Package core defines the canonical Lattice domain types.
//
// core has zero dependencies on other Lattice packages: it is the bedrock
// every other layer builds on. As a consequence, it never imports
// internal/errs and validation methods return plain `error` values that
// callers (typically the ontology validator or the API layer) wrap with a
// proper errs.Code at the boundary.
//
// Concepts mirror docs/06-domain-model:
//
//   - Configuration entities: Workspace, Datasource, ObjectType, Property,
//     LinkType, ActionType, Role, PolicyRule, CustomTool, Agent, Asset.
//   - Runtime entities: Upload, ActionRun, AgentRun, AssetVersion, Job.
//   - Governance entities: AuditRecord, OntologyVersion.
//
// Cross-cutting value types: Actor, Filter (AST), QuerySpec, AggregateSpec,
// Mutation, Record, Page, AggregateResult, MutationResult, DataType,
// Cardinality, Operation, APIName.
package types
