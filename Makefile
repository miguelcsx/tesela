## Lattice — development Makefile.
##
## Targets are intentionally minimal. Production deployment, container builds, and CI
## live outside this file (see docs/10-deployment/ when published).

GO              ?= go
GOFLAGS         ?=
PKGS            := ./...
COVER_PROFILE   := coverage.out

# Build artifacts go to ./bin/
BIN_DIR         := bin
API_BIN         := $(BIN_DIR)/lattice-api
WORKER_BIN      := $(BIN_DIR)/lattice-worker
CLI_BIN         := $(BIN_DIR)/lattice

# Linker flags inject version metadata. Override VERSION on tagged builds.
VERSION         ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev)
COMMIT          ?= $(shell git rev-parse --short HEAD 2>/dev/null || echo unknown)
BUILD_DATE      ?= $(shell date -u +%Y-%m-%dT%H:%M:%SZ)
LDFLAGS         := -s -w \
	-X github.com/miguelcsx/lattice/internal/buildinfo.Version=$(VERSION) \
	-X github.com/miguelcsx/lattice/internal/buildinfo.Commit=$(COMMIT) \
	-X github.com/miguelcsx/lattice/internal/buildinfo.Date=$(BUILD_DATE)

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help.
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: tidy
tidy: ## Reconcile go.mod / go.sum.
	$(GO) mod tidy

.PHONY: fmt
fmt: ## Format code (gofumpt + gci if available, falls back to gofmt).
	@command -v gofumpt >/dev/null && gofumpt -w . || $(GO) fmt $(PKGS)
	@command -v gci >/dev/null && gci write --skip-generated -s standard -s default -s 'prefix(github.com/miguelcsx/lattice)' . || true

.PHONY: lint
lint: ## Run static analysis (golangci-lint required).
	golangci-lint run ./...

.PHONY: vet
vet: ## go vet.
	$(GO) vet $(PKGS)

.PHONY: test
test: ## Run unit tests with race detector.
	$(GO) test -race -timeout 90s $(PKGS)

.PHONY: test-short
test-short: ## Run only short tests (skip integration).
	$(GO) test -race -short -timeout 60s $(PKGS)

.PHONY: test-integration
test-integration: ## Run integration tests (requires Docker for testcontainers).
	$(GO) test -race -tags=integration -timeout 300s $(PKGS)

.PHONY: cover
cover: ## Produce a coverage report (HTML output: coverage.html).
	$(GO) test -race -coverprofile=$(COVER_PROFILE) -covermode=atomic $(PKGS)
	$(GO) tool cover -html=$(COVER_PROFILE) -o coverage.html
	@echo "Coverage report written to coverage.html"

.PHONY: bench
bench: ## Run benchmarks.
	$(GO) test -run='^$$' -bench=. -benchmem $(PKGS)

.PHONY: build
build: build-api build-worker build-cli ## Build all three binaries.

.PHONY: build-cabi
build-cabi: $(BIN_DIR) ## Build liblattice.so (C ABI).
	$(GO) build $(GOFLAGS) -tags cabi -buildmode=c-shared -ldflags '$(LDFLAGS)' -o dist/liblattice.so ./pkg/lattice/cabi

.PHONY: build-cabi-all
build-cabi-all: ## Cross-compile liblattice.so for linux-amd64, linux-arm64, darwin-amd64, darwin-arm64, windows-amd64.
	@echo "Cross-compilation requires zig or golang.org/x/mobile. Placeholder target."
	@mkdir -p dist
	for pair in linux-amd64 linux-arm64 darwin-amd64 darwin-arm64 windows-amd64; do 		echo "Building for $$pair..."; 	done

.PHONY: upload-release
upload-release: ## Publish binaries to GitHub releases (requires gh CLI).
	@test -n "$(VERSION)" || (echo "VERSION is required"; exit 1)
	gh release create "v$(VERSION)" dist/liblattice-* --title "v$(VERSION)" --generate-notes

.PHONY: build-api
build-api: $(BIN_DIR) ## Build lattice-api.
	$(GO) build $(GOFLAGS) -ldflags '$(LDFLAGS)' -o $(API_BIN) ./cmd/lattice-api

.PHONY: build-worker
build-worker: $(BIN_DIR) ## Build lattice-worker.
	$(GO) build $(GOFLAGS) -ldflags '$(LDFLAGS)' -o $(WORKER_BIN) ./cmd/lattice-worker

.PHONY: build-cli
build-cli: $(BIN_DIR) ## Build lattice CLI.
	$(GO) build $(GOFLAGS) -ldflags '$(LDFLAGS)' -o $(CLI_BIN) ./cmd/lattice

$(BIN_DIR):
	mkdir -p $(BIN_DIR)

.PHONY: clean
clean: ## Remove build artifacts and coverage output.
	rm -rf $(BIN_DIR) $(COVER_PROFILE) coverage.html

.PHONY: verify
verify: vet lint test ## Lint + vet + test (run before pushing).
