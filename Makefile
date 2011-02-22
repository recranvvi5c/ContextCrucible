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
