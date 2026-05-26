# Tesela Documentation

Tesela is an open-source ontology-driven application runtime. Teams define their domain — object types, relationships, actions, policies, and agents — and Tesela generates REST and GraphQL APIs, enforces access policies, executes actions, and audits every operation, against any data store.

## What This Documentation Covers

This documentation set describes the complete architecture, design decisions, domain model, component specifications, security model, deployment strategies, and engineering standards for the Tesela system.

## Folder Index

| Folder | Contents |
|--------|----------|
| `00-overview/` | Introduction, core concepts, and design principles |
| `01-vision/` | Problem statement, market context, and long-term vision |
| `02-prd/` | Product requirements, user stories, and success metrics |
| `03-srs/` | System requirements specification |
| `04-system-design/` | Architecture, runtime topology, and pipeline designs |
| `05-adrs/` | Architecture Decision Records |
| `06-domain-model/` | All domain entities and their relationships |
| `07-data-model/` | Metadata storage schema and data type definitions |
| `08-components/` | Specification of each runtime component |
| `09-security/` | Authentication, authorization, policy model, encryption |
| `10-deployment/` | Deployment modes, configuration, and operations |
| `11-observability/` | Traces, metrics, logs, and alerting |
| `12-prr/` | Production Readiness Review checklist and targets |
| `13-engineering-standards/` | Code style, testing, API design, and conventions |
| `14-roadmap/` | Phase-by-phase delivery plan |
| `15-extensions/` | How to extend Tesela with adapters and tools |
| `15-ui-ux/` | CLI design principles and command reference |
| `15-user-flows/` | End-to-end flows for core user journeys |
| `16-data-state/` | State machines for uploads, actions, and assets |
| `17-technical-specifications/` | Format specifications for YAML, API, filters, SDKs |
| `18-enterprise-architecture/` | Multi-tenancy, HA, scaling, and compliance |
| `19-submodules-and-dependencies/` | Dependency inventory and management |
| `20-folder-architecture/` | Repository layout and package boundary rules |
| `21-crates/` | Per-package descriptions: files, purpose, responsibilities |

## Quick Navigation

- New to Tesela? Start with `00-overview/00-introduction.md`
- Evaluating fit? Read `01-vision/01-problem-statement.md` and `02-prd/01-product-overview.md`
- Integrating Tesela? See `17-technical-specifications/` and `15-user-flows/`
- Contributing? Read `13-engineering-standards/` and `20-folder-architecture/`
- Deploying? Go to `10-deployment/`
