# Schema Discovery

## The Problem

Data files uploaded by users have no guaranteed column naming convention. A column representing a trip identifier might be named trip_id, tripID, TripID, ID, id_trip, or any other variation. The same physical concept appears under different names in files from different sources, teams, or time periods. Requiring users to rename columns before upload is not practical at scale.

## Tesela's Approach

Schema discovery separates the raw file schema (whatever the source uses) from the ontology schema (the canonical property names the team has defined). The mapping between them is an explicit, inspectable, versioned configuration — not an implicit convention.

## Discovery Process

When the worker receives an ingestion job, it reads a limited number of rows from the top of the file and performs the following analysis for each column:

**Name detection**: The exact column name as it appears in the file. No normalization or case conversion is applied at this stage.

**Type inference**: The worker reads the column's values across the sample and infers the most specific compatible type. The inference hierarchy is: boolean, integer, float, date, timestamp, and string (as fallback). A column with mixed types that do not coerce to a single type is inferred as string.

**Null rate**: The fraction of sampled rows where the column value is null, empty, or the string "null" or "NULL". High null rates are surfaced as information for the user.

**Unique rate**: The fraction of sampled rows where the column value is distinct from all other sampled values. A unique rate near 100% suggests the column may be a primary key or identifier.

**Sample values**: A small set of representative values (typically five) drawn from the sample, excluding nulls. Sample values help users verify that the detected column represents what they expect.

**Candidate match**: The worker compares the detected column name against the asset's declared property API names using a combination of exact match, case-insensitive match, and edit distance. The best-scoring candidate is presented as the suggested mapping.

## Column Mapping Configuration

The column mapping is stored as part of the asset definition. Each mapping entry specifies:

- The source column name (as it appears in the file)
- The target property API name
- An optional type coercion rule (for example, parsing a string formatted as an ISO 8601 timestamp into a timestamp type)
- An optional value mapping (a dictionary from source values to canonical values, for example mapping "automobile" to "car" in a transport mode column)
- Whether the column is required (missing required columns block the load)

Columns that appear in the file but have no mapping entry are ignored or flagged depending on the asset's unmapped column policy (warn or error).

## Saved Mappings

Once a column mapping is confirmed for an asset, it is saved in the asset definition. Subsequent uploads of files for the same asset apply the saved mapping automatically without prompting the user. If a new upload contains columns that are not covered by the saved mapping, those columns are flagged for review.

## AI-Assisted Mapping Suggestion

The schema discovery output (column names and sample values) and the asset's property definitions (names, descriptions, and data types) are well-suited for AI-assisted suggestion. An agent configured for ontology assistance can receive the discovery output as input, compare it against the asset's declared properties, and suggest a complete mapping. The user reviews and confirms the suggestion. This is not a Tesela core feature but a use case that emerges naturally from the agent and tool system.
