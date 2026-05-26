# Object Type

## Definition

An object type is a named class of operational entities. It defines what properties instances of this class have, where their data comes from, and what the primary key is. Object types are the nodes in the ontology graph. They are the subjects of queries, the targets of links, and the subjects or recipients of actions.

## Attributes

**api_name**: A stable, unique identifier within the workspace. Used in API paths, link type references, action type references, and SDK type names. Should be in PascalCase by convention. Once set and in use, changing this identifier breaks link type references and client code.

**display_name**: A human-readable label shown in UIs and documentation.

**description**: A free-text explanation of what this object type represents in the business domain. Included in the auto-generated agent system prompt.

**primary_key**: The API name of the property that uniquely identifies instances. This property must exist in the object type's property list.

**source**: Configuration specifying where instances of this type are stored. At minimum, this includes the datasource API name and the source table or view name. For computed or federated types, additional configuration specifies how to join or combine sources.

**environments**: An optional list of environment names (such as staging and production) in which this object type is available. If not specified, the type is available in all environments.

**deprecated_at**: When set, indicates that this object type is planned for removal. Queries still succeed, but the API returns a deprecation warning header.

## Properties

An object type contains a list of property definitions. See the Property entity for the full property model.

Object types also support computed properties: properties whose values are derived from an expression evaluated against other properties of the same instance. Computed properties are not stored in the datasource — they are evaluated during result hydration.

## Lifecycle

Object types are created through the ontology API or by applying an ontology spec. They can be updated to add or modify properties, change descriptions, or update source configuration. Removing properties from an object type is a breaking change for clients and should be preceded by a deprecation period. Deleting an object type removes it from the ontology and invalidates all link types that reference it.

## Constraints

- Two object types in the same workspace cannot have the same api_name.
- The primary_key property must be declared in the object type's property list.
- The referenced datasource must exist in the workspace.
- An object type cannot reference a datasource that has been deleted.
