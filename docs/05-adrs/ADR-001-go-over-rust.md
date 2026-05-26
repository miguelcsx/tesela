# ADR-001: Rust as the Core Runtime Language

## Status

Accepted, superseding the earlier Go decision.

## Context

Tesela is now a library and native runtime, not a legacy Go platform. The core
runtime must be embeddable, publishable as Rust crates, callable from Python
through a C ABI, and safe to host inside teams' own services without generated
HTTP clients or hidden platform assumptions.

## Decision

Rust is the implementation language for the core runtime crates, native ABI,
GraphQL integration, MCP surface, and adapter contracts. Python remains a
hand-written SDK that builds the canonical IR and calls the native runtime.

## Reasoning

**Native embedding**: Rust produces a stable C-compatible shared library for
Python and future SDKs while keeping the runtime in process.

**Explicit contracts**: The type system makes backend capability traits,
policy decisions, audit sinks, and ontology IR structures explicit at compile
time.

**Packaging**: Cargo crates and PyPI wheels map cleanly to the library/toolchain
delivery model.

**Runtime safety**: Rust's ownership and concurrency model is a better fit for
running user-registered adapters and callbacks inside host applications.

## Consequences

Go-era packages, generated SDK clients, and HTTP-mediated SDK/runtime calls are
not part of the production architecture. External integrations are implemented
as Rust or Python adapters registered through the runtime contracts.
