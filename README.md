# Tesela

Tesela is an ontology-driven application library and toolchain for teams that
want Foundry/AIP-style primitives without adopting a platform. It gives you the
core contracts — ontology, graph, policy, action, agent, audit, metadata, and
data-access abstractions — so you can build your own runtime on your own
infrastructure, against your own backends, in your own language ecosystem.

Tesela does not own your connectors, your warehouse semantics, your anomaly
logic, your entity-resolution strategy, or your catalog conventions. Those stay
external. Tesela provides explicit interfaces and declarative data models so
those capabilities can be plugged in cleanly instead of being hardcoded into
the framework.

## Core surfaces

- Programmatic ontology/spec registration via Rust builders, Python decorators,
  and canonical JSON specs.
- External backend contract with optional capabilities for query, mutation,
  bulk load, traversal, and explain-plan support.
- Policy engine, action runtime, audit pipeline, and agent tool derivation.
- Schema graph utilities for shortest path, multi-hop traversal planning, cycle
  detection, impact analysis, and explicit lineage edges.
- Extensible metadata, property transforms, computed-property dependencies, and
  discovery/statistics hooks.
- Hand-written SDK surfaces that emit the same canonical IR and embed the same
  native runtime without generated HTTP clients.

See [`docs/`](./docs/) for the architectural specification.

## Design principles

1. **Schema-neutral** — no built-in domain knowledge.
2. **Backend-neutral** — data access is defined by interfaces; connectors live outside the core.
3. **Policy-neutral** — teams define their own roles, hierarchies, and rules.
4. **Language-neutral** — SDKs in Python, Rust, and future languages build the same IR and call the same native runtime.
5. **Infrastructure-neutral** — Tesela is a library/toolchain; teams provision and operate their own runtime.
6. **Explicit over magic** — nothing happens that wasn't declared.
7. **Everything audited** — append-only, non-suppressible.
8. **Ontology is live data** — changes take effect without restart.

## What stays outside the core

- Concrete connectors and vendor-specific adapter packages.
- CDC, event streaming, anomaly detection, entity resolution, and catalog
  enrichment heuristics.
- Warehouse-specific optimizers, caches, and execution engines.
- Approval systems, notification systems, and governance workflows.

## Quick start

```bash
make build
make test
```

Native SDK runtime:

```bash
make build-cabi
```

Language SDKs compile their local builders to `tesela.spec.v1` JSON and pass
that IR to the native runtime library. They are not generated clients and do not
communicate with Tesela over HTTP.

## License

Apache 2.0. See [LICENSE](./LICENSE).
