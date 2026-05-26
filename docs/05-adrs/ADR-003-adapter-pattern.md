# ADR-003: Adapter Pattern for Data Store Abstraction

## Status

Accepted

## Context

Tesela must support multiple data stores: relational databases, analytical warehouses, embedded query engines, and object storage. The query and action logic must work identically regardless of which data store backs a given object type. Different object types within the same workspace may use different data stores.

## Decision

All data access goes through two interfaces: DataAdapter and Connection. No business logic package imports a specific data store driver. Adapter implementations are separate packages that register themselves with the adapter registry at startup.

## Reasoning

**Universality requirement**: No single data store is universally used by all teams. Mandating Postgres would exclude teams on BigQuery. Mandating BigQuery would exclude self-hosted teams. The adapter interface allows Tesela to be neutral.

**Changeability**: Teams change their data store infrastructure over time. A workspace that starts on Postgres may migrate some object types to BigQuery as their data grows. The adapter interface makes this a configuration change, not a code change.

**Testability**: Business logic packages (query, policy, actions) can be tested with an in-memory adapter implementation that does not require an external database. This speeds up the test suite and eliminates database dependency in unit tests.

**Extension by the community**: Third parties can implement adapters for data stores that the Tesela maintainers do not support. The interface is stable and published. A community adapter for Oracle, for example, would register with the same interface without requiring changes to the core runtime.

## Trade-offs Accepted

The adapter interface is a lowest-common-denominator abstraction. Some data stores have capabilities (such as specific window functions, native geospatial operations, or vector similarity search) that cannot be expressed in the generic interface. For these capabilities, adapters may expose extension points that are only available when the specific adapter is configured, but which are not guaranteed by the interface.

Query generation is duplicated across adapter implementations. A SQL query for Postgres and a SQL query for BigQuery have different syntax for some operations (date functions, array handling, quoting). Each adapter generates the correct syntax for its target store.

## Consequences

The adapter registry must be initialized before any request can be served.
Production paths do not silently install adapters. Teams register Rust adapters
directly or Python adapters through the native callback ABI, and development
memory adapters are opt-in.
