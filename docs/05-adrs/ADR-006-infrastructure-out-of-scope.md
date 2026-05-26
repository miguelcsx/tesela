# ADR-006: Infrastructure Provisioning Is Outside Tesela's Scope

## Status

Accepted

## Context

An ontology-driven operational platform requires several infrastructure components: a metadata database, object storage for uploads and assets, an optional message queue, and the compute environment where Tesela runs. A question arose during design: should Tesela provision and manage this infrastructure, or should it assume the infrastructure exists and connect to it?

## Decision

Tesela does not provision infrastructure. It connects to infrastructure that teams provision using their own tooling (Terraform, Pulumi, cloud console, or similar). Tesela's configuration references pre-existing resources by endpoint and credentials. Tesela does not create databases, buckets, queues, or network resources.

## Reasoning

**Domain boundary**: Infrastructure provisioning is a distinct discipline from application runtime management. Terraform, Pulumi, Crossplane, and cloud provider CLIs solve infrastructure provisioning well. Adding this capability to Tesela would make it a more complex tool that solves two distinct problems instead of one.

**Operational control**: Teams need fine-grained control over their infrastructure: network placement, backup policies, encryption settings, access control at the cloud level, cost allocation tags, and disaster recovery configuration. Abstracting infrastructure creation through Tesela would limit this control.

**Avoid competing with established tools**: Terraform is the de facto standard for infrastructure-as-code. Competing with it would require building a provisioning model, a state management system, a plan/apply workflow, and provider coverage for all major clouds. This is not Tesela's purpose.

**Deployment environment flexibility**: Some teams run on Kubernetes, some on Cloud Run, some on bare metal. If Tesela managed infrastructure, it would need to support all of these environments at the infrastructure level. Staying out of provisioning means Tesela runs identically in all environments.

## What Tesela Does Provide

While Tesela does not provision infrastructure, it provides: example Docker Compose configurations for local development (complete with a Postgres instance and MinIO for object storage), example Kubernetes manifests for production deployment, and documentation describing what infrastructure components are needed and how to configure them. These are starting points for teams, not managed infrastructure.

## Consequences

Teams deploying Tesela are responsible for provisioning and operating the metadata database, object storage, and any other infrastructure that Tesela connects to. Tesela's documentation provides clear requirements for each component: minimum Postgres version, required object storage bucket policies, and queue configuration where applicable.
