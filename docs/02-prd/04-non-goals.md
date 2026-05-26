# Non-Goals

The following are explicitly outside the scope of Tesela. Teams that need these capabilities should use purpose-built tools and integrate them with Tesela at the boundary.

## Authentication and Identity

Tesela validates JWT tokens presented in requests. It does not issue tokens, manage login flows, handle OAuth or SAML exchanges, store passwords, manage sessions, or provide a user directory. Authentication is delegated to Auth0, Keycloak, Clerk, Cognito, or any OIDC-compatible identity provider.

## Cloud Infrastructure Provisioning

Tesela does not create cloud resources. It does not provision VPCs, subnets, managed databases, object storage buckets, message queues, or service accounts. Provisioning is the responsibility of Terraform, Pulumi, or the team's cloud provider tooling. Tesela configuration references pre-existing infrastructure.

## Message Brokers and Event Streaming

Tesela does not implement a message broker. It does not run Kafka, NATS, Pub/Sub, or RabbitMQ. Where event delivery is needed for async action handlers or upload notifications, Tesela integrates with an externally operated broker through its adapter interface.

## BI and Data Visualization

Tesela exposes data through typed APIs. It does not provide dashboards, charts, pivot tables, or report generation. Teams connect visualization tools (Metabase, Grafana, Superset, Looker) to the Tesela API or directly to the underlying datasources.

## Notebook Environments

Tesela does not provide a Jupyter or equivalent notebook environment. Data exploration in notebooks is done through Tesela SDK clients or direct database connections.

## Container Orchestration

Tesela produces binaries and Docker images. It does not manage Kubernetes deployments, Helm chart configuration, or service mesh policies. These are the responsibility of the team's platform operations function.

## Full-Text Search Engine

Tesela does not implement a search engine. For full-text search capabilities, teams configure a search adapter (such as Elasticsearch or Typesense) and map object type properties to it. Tesela routes search queries to the configured adapter.

## Workflow Orchestration Beyond Actions

Tesela supports composite action handlers that sequence steps. It does not implement a general-purpose durable workflow engine. For long-running, stateful workflows with complex branching and compensation, teams use Temporal or a similar system and invoke it from an action webhook handler.

## Own Compute Cluster

Tesela does not manage compute workers for distributed data processing. The worker process for async job execution is a Tesela binary that teams deploy and scale. For distributed processing of very large datasets, teams configure Ray, Spark, or equivalent external systems and invoke them through asset transform configurations.
