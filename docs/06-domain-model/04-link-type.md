# Link Type

## Definition

A link type is a named, directed relationship between two object types. It describes how instances of one type are connected to instances of another type, and how those connections are resolved at query time. Link types are the edges in the ontology graph.

## Attributes

**api_name**: A stable, unique identifier within the workspace. By convention, link type names use the format SourceType.relationshipName to make the direction and semantic clear (for example, Customer.orders or Order.items).

**display_name**: A human-readable label for the relationship.

**from_object_type**: The API name of the source object type — the type from which traversal begins.

**to_object_type**: The API name of the target object type — the type whose instances are returned by traversal.

**cardinality**: Describes the multiplicity of the relationship. The value is one of one_to_one, one_to_many, or many_to_many.

**property_mappings**: A list of join conditions. Each entry specifies a property on the from object type and a corresponding property on the to object type. The adapter translates these mappings into a join condition at query time. Multiple mappings in the same link type are combined with AND.

**junction**: Required only for many_to_many link types. Specifies the datasource, table name, from column, and to column of the junction table that records the relationships. Optional additional properties on the junction table can be declared and accessed when traversing the link.

## Traversal

Following a link type from a specific object instance produces the set of instances of the to_object_type that satisfy the join condition. Traversal results are subject to the actor's policy for the to_object_type — the policy engine applies row filters and property redaction to the target instances independently of the policy applied to the source instance.

## Cross-Adapter Links

A link type can connect object types that use different datasources. In this case, the query engine cannot generate a native SQL join because the data is in different systems. Instead, it executes the source query, extracts the join key values from the results, and uses them as a filter in a separate query against the target adapter. This approach handles any combination of adapters but does not benefit from database-level join optimizations.

## Auto-Generated Agent Tool

For every link type, the agent runtime generates a traversal tool. The tool accepts the primary key of a source object and an optional filter for the target type. Its description is derived from the link type's display_name and from_object_type and to_object_type references, providing the model with enough context to decide when to use the traversal.

## Constraints

- Both from_object_type and to_object_type must exist in the workspace.
- Property mappings must reference properties that exist on their respective object types.
- Computed properties cannot be used in property mappings.
- many_to_many link types must specify a junction configuration.
- A workspace can have multiple link types between the same pair of object types, provided each has a distinct api_name.
