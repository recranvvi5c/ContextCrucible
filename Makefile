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

