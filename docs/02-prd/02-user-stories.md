# User Stories

## Ontology Definition

As a platform engineer, I want to describe my domain entities and relationships in a declarative spec so that I have a single, portable, version-controlled definition of my operational data model.

As a platform engineer, I want to apply my ontology definition to a running Tesela instance without restarting the server so that ontology changes take effect immediately.

As a platform engineer, I want to see a diff between two ontology versions so that I understand what changed and what downstream impact to expect.

## Data Access

As an API consumer, I want to query objects by type and filter criteria so that I can retrieve operational data without writing SQL.

As an API consumer, I want to follow a link from one object to its related objects so that I can traverse the domain graph without constructing joins.

As an API consumer, I want to aggregate objects by property values so that I can compute summaries without managing analytical queries.

As a developer, I want a typed SDK client generated from the live ontology so that I interact with the API in a language-native way with compile-time safety.

## Access Control

As a security engineer, I want to define roles and their inheritance hierarchy so that permission structures reflect my organization's actual structure.

As a security engineer, I want to define conditions on policy rules so that access restrictions are attribute-based and not limited to flat role assignments.

As a security engineer, I want certain properties automatically redacted from API responses for specific roles so that sensitive data never leaves the system for unauthorized actors.

## Actions

As a product engineer, I want to define a typed action that updates an object's properties without writing a custom endpoint so that simple state mutations require no code.

As a product engineer, I want to define an action that calls an external service when executed so that complex business logic can live in a service I own and control.

As a product engineer, I want every action execution to be deduplicated by an idempotency key so that retries are safe.

## Data Upload

As a data engineer, I want to upload a large file directly to object storage without routing it through the Tesela server so that uploads are fast and do not exhaust server memory.

As a data engineer, I want Tesela to detect the uploaded file's schema and suggest a mapping to my ontology's asset properties so that I do not need to know column names in advance.

As a data engineer, I want to see a structured validation report after upload so that I understand which rows failed quality checks and why.

## Agents

As an AI engineer, I want agents to automatically receive query and traversal tools for all object types in my ontology so that I do not manually define agent tool schemas.

As a security officer, I want agent tool calls to go through the same policy engine as human API requests so that agents cannot access data or execute actions beyond their granted permissions.

As an operator, I want every agent tool call recorded in the audit log so that I have a complete trace of what the agent did and why.

## Operations

As a platform operator, I want a single configuration file to wire Tesela to all my datasources, secret providers, and observability backends so that deployment is declarative and reproducible.

As a platform operator, I want health and readiness endpoints so that my orchestration layer can manage Tesela lifecycle correctly.

As a platform operator, I want OpenTelemetry traces covering every request so that I can diagnose latency and errors in production.
