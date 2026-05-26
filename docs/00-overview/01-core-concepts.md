# Core Concepts

## Workspace

A workspace is the top-level isolation boundary in Tesela. All ontology entities, data sources, policies, agents, and audit records belong to exactly one workspace. Workspaces are independent of each other. A single Tesela deployment can serve multiple workspaces, each with its own complete ontology and configuration.

## Datasource

A datasource is a named, configured connection to an external data store. It references an adapter type (such as postgres, bigquery, or duckdb) and holds encrypted connection credentials. Object types source their data from one or more datasources. Tesela does not own or manage the data stores; it connects to stores that teams provision independently.

## Object Type

An object type is the fundamental unit of the domain model. It represents a class of operational entities — customers, orders, trips, patients, shipments — and defines what properties each instance has, where the data comes from, and how it relates to other object types. Object types are defined in the ontology registry at runtime, not in application code.

## Property

A property is a typed field on an object type. Each property maps to one or more columns in the underlying datasource, with optional type coercion, column name aliasing, and computed expressions. Properties carry metadata such as data type, nullability, allowed values, and sensitivity tags.

## Link Type

A link type is a named, directional relationship between two object types. It has a cardinality (one-to-one, one-to-many, many-to-many) and a set of property mappings that define the join condition. Link types enable traversal: given an object, the system can follow its links to retrieve related objects. Many-to-many links may specify a junction table.

## Action Type

An action type is a typed mutation — an operation that changes state. It has a defined input schema, an optional output schema, an associated permission key, and a handler that executes the logic. Handlers can be declarative (update a property value, create an object, delete an object) or external (call an HTTP webhook). Actions are validated, policy-checked, deduplicated by idempotency key, executed, and audited.

## Role

A role is a named principal category that actors can hold. Roles are defined by the team in the ontology — there are no built-in roles. Roles can inherit from other roles, forming a hierarchy where inheriting roles accumulate all permissions of their ancestors.

## Policy Rule

A policy rule specifies what principals (roles) can do with which resources (object types) under what conditions. Conditions include attribute matching between actor claims and object properties, ownership relationships, time windows, data windows, and arbitrary expressions. Policy rules also define property-level restrictions (redacting sensitive fields from responses).

## Custom Tool

A custom tool is a user-defined capability that agents can invoke. Tools can be SQL queries against a datasource, webhook calls to external services, or composite sequences of other tools. Like action types, tools have typed input and output schemas.

## Agent

An agent is a defined AI assistant configuration. It specifies a model provider and model name, a system prompt, a list of tools (auto-generated from the ontology and custom), limits on resource consumption, and which roles can invoke it. Agents operate entirely through their tool list — they never access data stores directly.

## Asset

An asset is a data set with a defined schema, quality rules, and a sink destination. Assets are produced by ingestion jobs that receive uploaded files, validate them against the schema, transform them, and load them into a target datasource. Assets become the data backing for object types.

## Ontology Version

A snapshot of the entire ontology at a point in time. The registry maintains a history of versions. Teams can publish named versions, compare versions, and export the current ontology to a portable declarative document format.
