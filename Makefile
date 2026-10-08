# contextcrucible — the token foundry
#
# A thin orchestration layer over cargo and npm. Targets are phony; the real
# work lives in the Rust crate and the TypeScript explorer.

CARGO ?= cargo
NPM   ?= npm
FIXTURE := fixtures/sample-repo

.PHONY: all build test fmt clippy release explorer explorer-test demo compare scan clean help

all: build test explorer-test

## build: compile the Rust CLI (debug)
build:
	$(CARGO) build

## release: compile the optimised release binary
release:
	$(CARGO) build --release

## test: run all Rust unit + integration tests
test:
	$(CARGO) test

## fmt: format the Rust sources
fmt:
	$(CARGO) fmt

## clippy: lint the Rust sources (if clippy is installed)
clippy:
	$(CARGO) clippy --all-targets -- -D warnings

## explorer: build the TypeScript explorer
explorer:
	cd explorer && $(NPM) install --no-audit --no-fund && $(NPM) run build

## explorer-test: build and test the TypeScript explorer
explorer-test:
	cd explorer && $(NPM) install --no-audit --no-fund && $(NPM) test

## demo: compile a pack from the fixture repo and print the explorer view
demo: build explorer
	$(CARGO) run --quiet -- compile --path $(FIXTURE) --budget 1200 \
		--query "budget solver knapsack" --label demo \
		--out examples/demo-pack.txt --manifest examples/demo-manifest.json
	node explorer/dist/cli.js examples/demo-manifest.json

## scan: report kept/rejected files for the fixture repo
scan: build
	$(CARGO) run --quiet -- scan --path $(FIXTURE)

## compare: build two packs and weigh them against each other
compare: build
	$(CARGO) run --quiet -- compile --path $(FIXTURE) --budget 600 \
		--query "budget solver knapsack" --label tight \
		--manifest examples/tight-manifest.json --out examples/tight-pack.txt
	$(CARGO) run --quiet -- compile --path $(FIXTURE) --budget 1200 \
		--query "budget solver knapsack" --label demo \
		--manifest examples/demo-manifest.json --out examples/demo-pack.txt
	$(CARGO) run --quiet -- compare --a examples/tight-manifest.json --b examples/demo-manifest.json

## clean: remove build artefacts
clean:
	$(CARGO) clean
	cd explorer && $(NPM) run clean || true

## help: list targets
help:
	@echo "contextcrucible targets:"
	@grep -E '^## ' $(MAKEFILE_LIST) | sed 's/## /  /'
