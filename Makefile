## Tesela development Makefile.
##
## Tesela Rust and native Python development targets.

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
build: rust-build python-build ## Build Rust crates and the Python extension package.

.PHONY: verify
verify: rust-fmt-check rust-clippy rust-test rust-package python-test python-build docs-check ## Run the local pre-push gate.

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
rust-package: ## Build all public .crate archives without publishing.
	cargo package --workspace --exclude tesela-python --allow-dirty --no-verify

.PHONY: python-test
python-test: ## Install the extension and run Python tests.
	$(PYTHON) -m pip install -e sdk/python
	$(PYTHON) -m pytest sdk/python/tests/ -q

.PHONY: python-build
python-build: ## Build Python sdist/wheel.
	$(PYTHON) -m build sdk/python

.PHONY: docs-check
docs-check: ## Verify local Markdown links and version consistency.
	$(PYTHON) scripts/check_release.py

.PHONY: clean
clean: ## Remove local build artifacts.
	rm -rf dist target sdk/python/dist sdk/python/build sdk/python/*.egg-info
