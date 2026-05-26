# Property

## Definition

A property is a typed attribute of an object type. It maps a logical name (the API name used by clients and agents) to a physical location in the underlying datasource (a column name). Properties carry type and constraint metadata that the query engine and policy engine use during execution.

## Attributes

**api_name**: The stable identifier for this property within its object type. Used in query filters, SDK types, policy rules, and column mappings. Changing this identifier is a breaking change for clients.

**display_name**: A human-readable label.

**description**: A free-text explanation of what this property represents. Included in agent tool descriptions to help the model understand the property's meaning.

**data_type**: The canonical type of the property value. See the Data Types reference in the data model documentation for the full list.

**source_column**: The name of the column or field in the datasource that backs this property. If the source column name matches the api_name, this field may be omitted.

**nullable**: Whether null is a valid value for this property. Defaults to true.

**indexed**: Whether this property has an index in the datasource that the query engine should prefer for filtered queries.

**allowed_values**: An optional list of specific string values that are valid for this property. Used for validation and surfaced in the generated GraphQL enum type and OpenAPI enum schema.

**tags**: An optional list of informational tags such as pii, phi, sensitive, or financial. These tags do not affect query behavior — policy rules govern access. Tags serve as documentation and can drive tooling (such as data catalogs) that reads the ontology.

**sort_order**: An integer that determines the order in which properties appear in API responses and language SDK views.

## Computed Properties

A property may be marked as computed rather than sourced from a column. Computed properties specify an expression evaluated during result hydration. Expressions can reference other properties of the same object instance. The expression language is the same CEL (Common Expression Language) used by policy conditions.

Examples of computed properties: an age_years property derived from a date_of_birth property; a full_name property concatenating first_name and last_name; a days_since_created property derived from a created_at timestamp.

Computed properties are read-only. Mutations cannot target a computed property.

## Data Types

The supported data types are: string, integer, bigint, float, decimal, boolean, date, timestamp, timestamptz (timestamp with time zone), uuid, json, geometry, and parameterized types array (containing elements of another type) and enum (a closed set of string values declared inline).

## Constraints

- Two properties in the same object type cannot have the same api_name.
- The data_type must be one of the supported types.
- Computed properties cannot be used as primary keys or join keys in link type mappings.
- Computed properties cannot appear in sort order specifications in search queries (because there is no index to use).
