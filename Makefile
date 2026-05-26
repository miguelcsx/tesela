## Tesela development Makefile.
##
## Tesela is a Rust runtime with a hand-written Python SDK. Targets here avoid
## legacy Go-era build paths and keep development checks explicit.

PYTHON ?= python

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help.
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: fmt
fmt: rust-fmt ## Format all Rust source files.

.PHONY: check
check: rust-check ## Type-check all Rust crates.

.PHONY: lint
lint: rust-clippy ## Run Clippy with warnings denied.

.PHONY: test
test: rust-test python-test ## Run Rust and Python tests.

.PHONY: build
build: rust-build build-cabi ## Build Rust crates and the native ABI library.

.PHONY: verify
verify: rust-fmt-check rust-clippy rust-test python-test ## Run the local pre-push gate.

.PHONY: rust-build
rust-build: ## Build all Rust crates.
	cargo build --workspace

.PHONY: rust-check
rust-check: ## Type-check all Rust crates.
	cargo check --workspace

.PHONY: rust-fmt
rust-fmt: ## Format all Rust source files.
	cargo fmt --all

.PHONY: rust-fmt-check
rust-fmt-check: ## Check Rust formatting.
	cargo fmt --all -- --check

.PHONY: rust-clippy
rust-clippy: ## Run Clippy with warnings denied.
	cargo clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: rust-test
rust-test: ## Run all Rust tests.
	cargo test --workspace

.PHONY: rust-doc
rust-doc: ## Build Rust documentation.
	cargo doc --workspace --no-deps

.PHONY: rust-package
rust-package: ## Check crate package metadata without publishing.
	cargo package --workspace --allow-dirty --no-verify

.PHONY: build-cabi
build-cabi: ## Build the native runtime shared library used by Python.
	cargo build -p tesela-cabi --release
	mkdir -p dist sdk/python/tesela
	cp target/release/libtesela_cabi.so dist/libtesela_cabi.so 2>/dev/null || \
	  cp target/release/libtesela_cabi.dylib dist/libtesela_cabi.dylib 2>/dev/null || \
	  cp target/release/tesela_cabi.dll dist/tesela_cabi.dll 2>/dev/null || true
	cp target/release/libtesela_cabi.so sdk/python/tesela/ 2>/dev/null || \
	  cp target/release/libtesela_cabi.dylib sdk/python/tesela/ 2>/dev/null || \
	  cp target/release/tesela_cabi.dll sdk/python/tesela/ 2>/dev/null || true
	@echo "Native library written to dist/ and sdk/python/tesela/"

.PHONY: build-cabi-debug
build-cabi-debug: ## Build the native runtime shared library in debug mode.
	cargo build -p tesela-cabi
	mkdir -p dist sdk/python/tesela
	cp target/debug/libtesela_cabi.so dist/libtesela_cabi.so 2>/dev/null || \
	  cp target/debug/libtesela_cabi.dylib dist/libtesela_cabi.dylib 2>/dev/null || \
	  cp target/debug/tesela_cabi.dll dist/tesela_cabi.dll 2>/dev/null || true
	cp target/debug/libtesela_cabi.so sdk/python/tesela/ 2>/dev/null || \
	  cp target/debug/libtesela_cabi.dylib sdk/python/tesela/ 2>/dev/null || \
	  cp target/debug/tesela_cabi.dll sdk/python/tesela/ 2>/dev/null || true

.PHONY: python-test
python-test: ## Run Python SDK tests against the selected native library.
	cd sdk/python && PYTHONPATH=. $(PYTHON) -m pytest tests/ -v

.PHONY: python-build
python-build: ## Build Python sdist/wheel.
	cd sdk/python && $(PYTHON) -m build

.PHONY: clean
clean: ## Remove local build artifacts.
	rm -rf dist target sdk/python/dist sdk/python/build sdk/python/*.egg-info
