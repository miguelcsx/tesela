# Upload and Ingestion Pipeline

## Design Principle

Large data files never pass through the Tesela API server. The server generates a signed URL pointing to object storage. The client uploads directly to that URL. Tesela orchestrates validation and loading but does not act as a data proxy.

## Phase 1: Upload URL Request

The client requests an upload by specifying the target asset API name and optional metadata (such as a correlation identifier for the domain object this upload belongs to). The API server creates an upload record with a generated upload identifier and a status of pending. It generates a time-limited signed URL for direct client upload to object storage, placing the object at a path that encodes the workspace, asset name, and upload identifier. It returns the signed URL and the upload identifier to the client.

## Phase 2: Direct Client Upload

The client uploads the file directly to object storage using the signed URL. For large files, resumable upload protocols (such as GCS resumable uploads or S3 multipart uploads) allow the client to pause and resume without losing progress. Tesela is not involved in data transfer during this phase.

## Phase 3: Upload Notification

When the upload completes, either the client calls the upload completion endpoint with a checksum, or the object storage system delivers an event notification to a Tesela-managed webhook. The API server marks the upload record as uploaded and enqueues an ingestion job.

## Phase 4: Schema Discovery

The worker picks up the ingestion job. It reads the first chunk of the uploaded file from object storage without loading the entire file into memory. It detects the file format (CSV, Parquet, JSON Lines, or Avro) from the file header or extension. It reads the column names and infers their data types from a sample of rows. It computes statistics for each column: null rate, unique rate, minimum and maximum values, and a set of sample values.

The discovery output is stored in the upload record and presented to the user through the upload status endpoint. The user reviews the detected columns and their statistics.

## Phase 5: Column Mapping

The user specifies or confirms the mapping from detected column names to the asset's declared property names. A mapping entry for each detected column specifies the target property API name and, optionally, a type coercion rule (such as parsing a string column as a timestamp with a declared format) and a value mapping (translating specific source values to canonical property values).

If the asset has a pre-declared column mapping in its definition, this step is skipped — the saved mapping is applied automatically. If column names match property names exactly, a default mapping is proposed and can be confirmed without modification.

## Phase 6: Pre-Load Validation

The worker reads a configurable sample of rows (by default, the first ten thousand rows) from the file and evaluates the asset's quality rules against this sample. Quality rules include not-null checks, uniqueness checks within the sample, range checks, allowed-value checks, and regular expression checks. Rules have two levels: error (which blocks the load) and warning (which proceeds with a flag).

If any error-level rules fail in the sample, the upload status is set to failed with a structured error report. No data is loaded. The user can correct the source file and upload again.

## Phase 7: Bulk Load

If pre-load validation passes, the worker triggers a bulk load operation from object storage to the target datasource. The bulk load does not route data through the worker — it instructs the datasource to read directly from object storage. For BigQuery, this is a load job. For Postgres, this is a COPY FROM operation. For Snowflake, this is a COPY INTO operation.

Each row loaded is tagged with the upload identifier using a system-managed column. This tag enables rollback if post-load validation fails.

## Phase 8: Post-Load Validation

After the bulk load completes, the worker executes validation queries directly against the target datasource. These queries evaluate the full quality rule set against all loaded rows (not just the sample), using the upload identifier tag to isolate the newly loaded rows. This is efficient because it runs in the compute environment of the datasource, not in the worker process.

If post-load validation fails for error-level rules, the worker executes a delete operation targeting all rows with the upload identifier. The upload status is set to failed with the full validation report.

## Phase 9: Commit

If post-load validation passes, the worker removes the upload identifier tag from the loaded rows (or updates the asset's active version pointer to include them), updates the asset version record in the metadata database, and sets the upload status to completed. Any on-complete actions declared in the asset definition are triggered.
