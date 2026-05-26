# Design Principles

These eight principles govern every architectural and implementation decision in Tesela. When trade-offs arise, these principles determine the outcome.

## 1. Schema-Neutral

Tesela has no built-in domain knowledge. It knows nothing about customers, orders, patients, or shipments. Every entity, relationship, action, and policy is defined by the team using Tesela. The system is a runtime engine, not a domain model.

This means the same Tesela installation can serve a fintech team modeling accounts and trades, and a healthcare team modeling patients and encounters, with no code changes to the framework.

## 2. Adapter-Neutral

Tesela does not assume a specific data store. All data access goes through an adapter interface. Adapters exist for relational databases, analytical warehouses, embedded engines, and object storage. A workspace can use multiple adapters simultaneously — one object type reads from Postgres, another from BigQuery — with no coupling between them.

## 3. Policy-Neutral

Tesela does not prescribe roles or access patterns. Teams define their own roles, their inheritance hierarchy, and their policy rules. The policy engine evaluates rules at query time and action time, enforcing them by injecting filters into queries and blocking or allowing operations. There are no hardcoded permission checks anywhere in the codebase.

## 4. Language-Neutral

The runtime is implemented in Rust. SDKs for Python, Rust, and future languages
are maintained source packages that build the same canonical IR and call the
same native runtime. They are not generated HTTP clients.

## 5. Infrastructure-Neutral

Tesela runs the software. It does not provision cloud resources, manage networking, or own storage. Teams deploy Tesela on whatever infrastructure they operate — Kubernetes, Cloud Run, bare metal, or a local machine — and configure it to connect to infrastructure they manage separately.

## 6. Explicit Over Magic

No behavior is implicit or convention-based in a way that breaks at scale. Adapters are declared in configuration. Policies are declared in the ontology. Tool access is declared in agent definitions. If something is not declared, it does not happen.

## 7. Everything Audited

Every query, every action execution, every policy decision, and every ontology change is recorded in the audit log. The audit log is append-only. There is no mechanism in Tesela to suppress auditing. This is not configurable.

## 8. Ontology Is Live Data

The ontology is stored in the metadata database and cached in memory. Changes take effect without restarting the server. Adding a property, updating a policy rule, or registering a new action type is an API or CLI operation, not a deployment.
