# Introduction

## What Tesela Is

Tesela is an open-source ontology-driven application runtime. It provides a structured way for any team — fintech, healthcare, logistics, urban planning, CRM, supply chain, or any other domain — to define their operational data model and immediately operate on it through governed APIs, policy-enforced queries, typed actions, and AI agents.

Tesela is not a SaaS product. It is a framework and runtime that teams deploy on their own infrastructure, on top of their existing data stores.

## The Problem It Solves

Most operational platforms are built by encoding the business domain implicitly into application code, scattered across services, ORMs, and API handlers. When the domain grows or changes, everything must be updated manually. There is no central place that answers: what objects exist, how they relate, what can be done to them, and who is allowed to do it.

Tesela makes the domain explicit. The ontology — the definition of objects, relationships, actions, and policies — is the system's source of truth. Everything else is derived from it.

## Positioning

Tesela occupies the space between a general-purpose web framework and a proprietary operational platform such as Palantir Foundry or IAP. It is:

- More structured than a web framework: it has explicit concepts for objects, links, actions, policies, and agents
- More flexible than a proprietary platform: it runs on any infrastructure and connects to any data store
- More reusable than an internal platform: it is designed as a general-purpose open-source runtime, not tied to one company's domain

## Who Uses Tesela

Tesela is used by engineering teams that need to build operational platforms — internal applications where users query, analyze, and act on operational data. It is especially relevant when:

- The domain has well-defined entities and relationships
- Multiple data sources contribute to a unified operational view
- Access control must be fine-grained and auditable
- AI agents need governed access to operational data
- The platform must serve multiple teams or tenants

## What Tesela Is Not

Tesela is not a replacement for authentication systems, cloud provisioning tools, message brokers, or BI platforms. It integrates with all of these but does not implement them. See `02-prd/04-non-goals.md` for a complete list of what falls outside Tesela's scope.
