# ADR-004: Webhook as the Primary External Action Handler

## Status

Accepted

## Context

Action types need to execute business logic that goes beyond simple property mutations. This logic varies by team, by domain, and by the systems the team integrates with. Tesela needs a mechanism for teams to implement complex action logic in their own code and their own language without requiring that logic to run inside the Tesela process.

## Decision

The primary mechanism for external action logic is the webhook handler. When an action with a webhook handler is dispatched, Tesela makes an HTTP POST request to the configured URL, passing the action context as the request body, and records the response as the action output. The handler service is deployed and operated by the team, not by Tesela.

## Reasoning

**Language neutrality**: A webhook handler can be implemented in any language. Teams with existing Go, Python, Java, or TypeScript services can integrate their action logic without adopting a new runtime model. Tesela makes an HTTP call — the implementation on the other end is irrelevant.

**Operational separation**: The action logic and the Tesela runtime have independent deployment cycles, independent failure domains, and independent resource allocation. A slow or failing action handler does not impact Tesela's ability to serve read queries.

**Simplicity of contract**: The webhook contract is well-understood: HTTP POST, JSON body, HTTP response, configurable timeout and retry. Teams do not need to learn a new SDK, plugin protocol, or runtime model.

**Auditability**: Because all webhook calls pass through the action runtime, they receive the same idempotency, auditing, and policy enforcement as declarative handlers. The external service does not need to implement any of this — it only needs to implement the domain logic.

## Trade-offs Accepted

Webhook handlers require teams to operate an additional service. For simple mutations, this is overhead. This is addressed by the declarative handler types (crud_update, crud_create, crud_delete, composite) that handle common cases without any external service.

Webhook handlers introduce network latency and a dependency on the external service's availability. The retry configuration and timeout limits mitigate this, but a failing handler service will cause action failures.

## Alternatives Considered

**Embedded scripting (Lua, JavaScript, Rhai)**: Allows teams to write handler logic inline in the ontology definition. Rejected for Phase 1 because embedded scripting environments are complex to sandbox safely and debug effectively. This is planned as a future extension.

**WASM modules**: Allows teams to upload compiled WASM modules as handler logic. Rejected for Phase 1 due to the complexity of the WASM runtime integration and the immaturity of tooling for writing WASM in common languages. Planned for a later phase.

## Consequences

All action types with complex business logic require teams to deploy a handler service. The service must be reachable from the Tesela worker over the network. Secrets needed by the handler service are managed by the team and are not provided by Tesela — the handler receives the action context, not Tesela's internal credentials.
