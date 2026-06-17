# tesela-sdk

Python SDK for [Tesela](https://github.com/miguelcsx/tesela) — a schema-neutral, policy-first runtime for ontology-driven applications.

## Installation

Install from a pre-built wheel (recommended):

```bash
pip install tesela-sdk
```

Pre-built wheels are available for:
- Linux x86_64 / aarch64
- macOS x86_64 / aarch64
- Windows x86_64

## Quick Start

```python
from tesela import App, NativeRuntime, String, Integer

# Define an ontology
app = App("my-app")
app.datasource("main", "memory")
app.object_type("user") \
    .source("main", "users") \
    .property("id", String) \
    .property("name", String) \
    .property("age", Integer) \
    .primary_key("id") \
    .done()

# Run with the native Rust runtime
with NativeRuntime.from_app(app) as rt:
    rt.mutate("user", {
        "op": "create",
        "data": {"id": "u1", "name": "Alice", "age": 30},
    })
    page = rt.search("user", {"limit": 10})
    print(page["records"])
```

## Development

Requires Python >= 3.10.

```bash
pip install -e ".[dev]"
pytest
```

For local development with the native PyO3 module:

```bash
python -m maturin develop
pytest
```

## License

Apache-2.0
