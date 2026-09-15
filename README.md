# Tesela

Tesela is a Rust library with native Python bindings for ontology-driven applications. Define object types, links, actions and policies as a `tesela.spec.v1` document, then run that ontology against stores you provide. The library owns the ontology contract and runtime; your application owns concrete connectors and transports.

## Rust

Add `tesela = "0.1"` to your `Cargo.toml`. The facade includes the declarative macros by default:

```rust
use tesela::ObjectType;

#[derive(ObjectType)]
#[tesela(datasource = "memory", primary_key = "id")]
struct Customer {
    id: String,
    email: Option<String>,
}

fn main() {
    let object_type = Customer::tesela_object_type();
    println!("{}", object_type.api_name);
}
```

Build a `tesela::Spec`, register a store with `tesela::StaticStoreRouter`, and construct `tesela::Runtime` with an explicit policy engine. `tesela::MemoryStore` and `tesela::AllowAllPolicy` are intended for local development. Production applications provide their own `OntologyStore` and `PolicyEngine`; audit and event ports are optional.

## Python

Install `tesela` with pip. The Python package embeds the same Rust runtime; it is not an HTTP client.

```python
from tesela import MemoryStore, Runtime, Spec

spec = Spec("demo")
spec.datasource("memory")

@spec.object_type(datasource="memory", primary_key="id")
class Customer:
    id: str
    email: str | None

runtime = Runtime(spec, stores={"memory": MemoryStore()}, policy="allow_all")
actor = {"user_id": "local"}
runtime.mutate("customer", {"create": {"values": {"id": "1", "email": "a@example.com"}}}, actor=actor)
print(runtime.get("customer", "1", actor=actor)["values"])
```

`Spec` also has decorators for traits, links, actions and policies. `Spec.add(section, definition)` accepts all fields of the canonical IR for advanced declarations. `Spec.to_json()` validates declarations against the Rust IR and preserves supplied optional fields.

Python stores implement `search(object_type, query)`, `get(object_type, primary_key)`, `create(object_type, values)`, `update(object_type, primary_key, values)` and `delete(object_type, primary_key)`. They may implement `aggregate`, `traverse` and `execute_action`. Inputs and results are ordinary dictionaries matching the Rust contract. A policy object implements `evaluate(request)` and returns a decision dictionary with `allow`; optional audit and event objects implement `record(event)` and `publish(event)`.

Runtime operations are `search`, `get`, `mutate`, `aggregate`, `traverse`, `resolve_object_set`, `compose_object_sets` and `execute_action`. Every operation takes an explicit actor. Actions execute through the store of their declared `subject` object type; an action without `subject` remains metadata and cannot execute. `tool_definitions()` returns the native ontology tool definitions for agent integrations.

The `@spec.action` and Rust `#[action]` decorators declare action metadata; execution is supplied by the subject's store through `execute_action`.
Declared policy rules are metadata; the policy engine you pass to `Runtime` evaluates access requests.
Action input and output schemas are metadata in v1; the executing store validates payloads if needed.

## Development and release

Install Rust, Python 3.10–3.13, maturin, build and pytest. Run `make verify` for formatting, Clippy, Rust tests and Python tests; `make python-build` produces Python wheel and sdist. The release workflow checks version tags, runs tests, publishes the internal Rust crates in dependency order and then publishes the `tesela` facade. PyPI publishing requires a Trusted Publisher for `.github/workflows/release.yml`; Cargo publishing requires `CARGO_REGISTRY_TOKEN`.

The reproducible performance baseline is in the repository's `benchmarks/README.md`. The API has no built-in HTTP, GraphQL or MCP server. Policy enforcement is required; audit and event delivery occur only when those ports are configured.

## License

Apache-2.0.
