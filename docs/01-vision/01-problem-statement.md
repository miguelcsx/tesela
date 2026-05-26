# Problem Statement

## The Core Problem

Operational platforms — the internal systems through which teams query, analyze, and act on their domain data — are built repeatedly from scratch. Every company building a CRM, a logistics platform, a clinical operations tool, or a financial risk system faces the same set of problems: representing the domain model in code, connecting to heterogeneous data stores, enforcing fine-grained access control, building APIs that expose the domain to frontends and agents, and auditing everything that happens.

These problems are solved poorly and repeatedly. The domain model is implicit, scattered across database schemas, service code, and API handlers. Access control is an afterthought. Audit logs are incomplete. When the domain grows, every layer must change independently.

## Why Existing Tools Fall Short

General-purpose web frameworks provide HTTP primitives and database access but no domain modeling, no policy enforcement, and no ontology. Teams must build these from scratch every time.

Data catalogs and metadata platforms capture what data exists but do not provide an operational runtime — they cannot serve queries, enforce policies at query time, or execute actions.

Low-code platforms impose their own data model and limit extensibility, making them unsuitable for complex operational domains.

Proprietary operational platforms such as Palantir Foundry solve this problem well but are expensive, closed-source, and require organizational commitment to a vendor's ecosystem.

There is no production-grade, open-source, general-purpose ontology-driven application runtime.

## The Consequence

The absence of a standard solution means every team builds the same infrastructure repeatedly, at different levels of quality. The resulting systems are inconsistent, poorly audited, hard to extend, and tightly coupled to specific technology choices. When AI agents need governed access to operational data, there is no standard interface to plug them into.

## The Opportunity

A well-designed open-source runtime for ontology-driven applications would allow teams to define their domain once and receive governed APIs, policy enforcement, audit logs, and agent tools automatically. The domain definition becomes a portable artifact that drives the entire operational layer.
