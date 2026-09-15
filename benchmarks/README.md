# Reproducible baseline

Use a release build and the installed Python wheel. Both scripts create one object type, 100 in-memory records and run 10,000 iterations of parse, serialize, search, get and upsert. `search.100` returns 100 records. Timings are descriptive baselines, not CI thresholds.

```bash
cargo run --release -p tesela --example baseline
python benchmarks/baseline.py
```

Run on the same host, record CPU, OS, Rust/Python versions and use the median of three runs when comparing a change. Rust and Python timings include their respective public API conversion costs.

## v0.1.0 local baseline

Measured 2026-09-15 on arm64 macOS 26.5.2, Rust 1.98.0 and Python 3.13.12. Both used release-optimized Rust; values are medians of three runs, in microseconds per operation.

| Operation | Rust | Python wheel |
|---|---:|---:|
| Spec parse | 1.869 | 14.343 |
| Spec serialize | 0.587 | 13.965 |
| Search 100 rows | 14.987 | 71.836 |
| Get | 0.440 | 3.455 |
| Upsert | 0.798 | 5.762 |

Python timings include JSON conversion at the binding boundary and dictionary creation for results. They establish a comparison point; no regression threshold is enforced yet.
