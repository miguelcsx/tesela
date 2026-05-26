# Query Pipeline

## Overview

The query pipeline transforms an incoming object query request into a policy-filtered result set. Every step in the pipeline is mandatory and executed in a fixed order. There is no mechanism to skip any step.

## Step 1: Actor Resolution

The API server extracts the bearer token from the request. The token is validated against the configured JWT issuer. The actor is assembled from the token's claims: a user identifier, a set of roles, and a claims map containing arbitrary attributes (region, department, clearance level, or any other attribute the identity provider includes in the token).

## Step 2: Ontology Lookup

The query runtime loads the target object type from the ontology cache using the type's API name. If the object type is not found, the request fails with a not-found error before any database access occurs. The lookup also retrieves the object type's properties, the datasource configuration, and the adapter reference.

## Step 3: Policy Evaluation

The policy engine receives the actor, the target object type, and the operation (read or search). It evaluates all applicable policy rules in priority order. For each matching rule, it accumulates the set of allowed conditions. The evaluation produces two outputs: a row filter (a structured predicate that will be injected into the query) and a property deny list (the set of properties that must be redacted from the response).

If no rule allows the operation, the policy engine returns a deny decision. The request fails with a forbidden error and an audit record is written. No database access occurs.

## Step 4: Query Construction

The query package constructs an adapter query from the client's request parameters (filters, sort order, pagination) and the policy row filter. The client's filter and the policy filter are combined with a logical AND — the policy filter is never optional and cannot be overridden by client parameters.

The query also resolves property names from the client's requested fields to their source column names using the property definitions. If the client does not specify a field selection, all properties not in the deny list are selected.

## Step 5: Adapter Execution

The query is dispatched to the adapter corresponding to the object type's datasource. The adapter translates the structured query into the native query language of the data store (SQL for relational stores, API calls for warehouse services) and executes it. The adapter returns a list of raw records and a total count.

## Step 6: Result Hydration

The raw records from the adapter are mapped back from source column names to property API names. Computed properties are evaluated at this stage. Properties in the deny list are set to null in the result. The result set is bounded by the workspace's maximum rows configuration — any records beyond this limit are dropped, and a truncation flag is set in the response.

## Step 7: Audit and Response

An audit record is written containing: the actor identifier, the actor's roles, the operation, the object type, any filters applied, the policy decision (allow), the rules that matched, and the number of records returned. The response is serialized and returned to the client.

## Link Traversal Variant

When the request is for link traversal rather than a direct search, the pipeline runs twice. The first pass resolves the source object (by primary key, with policy check). The second pass constructs the join query using the link type's property mappings as the join condition, applies the target object type's policy for the actor, and executes the join. The join can cross adapters if the source and target object types use different datasources — in this case, the primary key of the source object is passed as a filter parameter to the target adapter rather than as a SQL join.
