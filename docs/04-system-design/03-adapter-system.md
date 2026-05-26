# Adapter System

## Purpose

The adapter system decouples Tesela's query and action logic from any specific data store. All data access goes through two interfaces: DataAdapter and Connection. No business logic package imports a specific adapter implementation. This allows new adapters to be added without modifying the core runtime.

## The DataAdapter Interface

A DataAdapter is a factory for connections to a specific type of data store. It exposes its adapter type identifier and a connect method that accepts a configuration map and returns a Connection. The configuration map contains the resolved credentials and connection parameters for the target store.

## The Connection Interface

A Connection represents an active, configured connection to a data store. It exposes the following operations:

**GetObject** retrieves a single record by primary key. It accepts the source configuration (which table or view to read from), the object type definition (which properties to select and how they map to source columns), the primary key value, and an optional policy row filter. It returns the record or a not-found result.

**SearchObjects** retrieves multiple records matching a filter. It accepts source configuration, object type definition, a structured query (filters, sort order, pagination), and a policy row filter. It returns a list of records and a total count.

**AggregateObjects** computes grouped aggregations. It accepts source configuration, object type definition, an aggregation specification (group-by fields, metric functions), and a policy row filter. It returns a list of group rows with computed values.

**ExecuteMutation** applies a write operation to the data store. It accepts source configuration and a structured mutation (the target table, the fields to write, and their values). It returns the number of affected rows and the primary key of any newly created record.

**Ping** verifies that the connection is live. It is used by health checks and by the initial connection test when a datasource is registered.

## Adapter Registration

Adapters are registered in the adapter registry at server startup. The registry maps adapter type identifiers to DataAdapter implementations. When a datasource is used for the first time, the registry looks up the adapter, calls its connect method with the resolved credentials, and caches the resulting Connection for reuse.

## Connection Pooling

Each adapter implementation is responsible for managing its connection pool. Pools are configured through the datasource configuration. Standard parameters include maximum open connections, maximum idle connections, and connection lifetime. The adapter registry does not manage pooling — it delegates this to each adapter.

## Supported Adapters

**Postgres** (Phase 1): Full read and write support via SQL generation. Supports all filter operators, all aggregate functions, all link traversal patterns, and all mutation types.

**DuckDB** (Phase 2): Read-only analytical queries. Supports the full filter and aggregation surface. Does not support mutations. Designed for assets backed by Parquet files in object storage.

**BigQuery** (Phase 2): Read-heavy with bulk write support for ingestion. Supports SQL generation for queries and aggregations. Mutations go through the BigQuery Storage Write API for batch operations and the streaming API for individual records.

**MySQL** (Phase 3): Full read and write support. SQL generation differs from Postgres in quoting conventions and function names.

**ClickHouse** (Phase 3): Read and analytical query support. Optimized for aggregation-heavy workloads.

**Snowflake** (Phase 4): Read-heavy with bulk write support. Integrates with Snowflake's internal copy mechanisms for ingestion.

**S3 / Object Storage** (Phase 2): Read-only access to Parquet, CSV, and JSON files. Used as a backing store for assets and as the staging area for ingestion.

**Elasticsearch / OpenSearch** (Phase 3): Read-only full-text search queries. Mapped to search operations on object types configured with a search source.
