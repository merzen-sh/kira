CARGO ?= cargo
BUN ?= bun
MPROCS ?= mprocs

.PHONY: all build build-cli build-all release dev install \
        install-hooks uninstall-hooks dashboard openapi \
        test test-hooks check fmt clippy clean

# Default
all: build

# Build
build:
	$(CARGO) build -p kira

release:
	$(CARGO) build --release -p kira

build-cli: dashboard
	$(CARGO) build --release -p kira-cli --no-default-features

build-all: dashboard
	$(CARGO) build --release -p kira
	$(CARGO) build --release -p kira-cli --no-default-features

# Development
dev: node_modules
	@trap 'kill 0' EXIT INT TERM; \
	$(MPROCS) --names vite,kira,kira-cli \
		"$(BUN) run --filter @kira/dashboard dev" \
		"$(CARGO) watch -w crates -x 'run -p kira'" \
		"$(CARGO) watch -w crates -x 'run -p kira-cli'"

# Dependencies
node_modules: package.json bun.lock
	$(BUN) install --frozen-lockfile

install: node_modules

# Git hooks
install-hooks:
	git config core.hooksPath .githooks
	chmod +x .githooks/pre-commit
	@echo "Installed Git hooks from .githooks"

uninstall-hooks:
	git config --unset core.hooksPath || true
	@echo "Uninstalled Git hooks"

# Dashboard
dashboard: node_modules
	$(BUN) run --filter @kira/dashboard build

# API
openapi: node_modules
	$(CARGO) xtask openapi generate
	$(BUN) run gen:api-types

# Tests
test:
	$(CARGO) test --workspace

test-hooks:
	bash scripts/test-pre-commit.sh

check: node_modules
	$(CARGO) fmt --check
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(CARGO) clippy -p kira --no-default-features -- -D warnings
	$(CARGO) test --workspace
	$(CARGO) xtask openapi check
	$(BUN) run check:api-types
	$(BUN) run typecheck
	$(BUN) run --filter @kira/dashboard test
	@set -eu; \
	cli_tree="$$($(CARGO) tree -p kira-cli)"; \
	kira_tree="$$($(CARGO) tree -p kira --all-features)"; \
	if printf '%s\n' "$$cli_tree" | grep -Eq 'kira-server|kira-bot|serenity'; then \
		echo "Unexpected server/bot dependency in kira-cli"; \
		exit 1; \
	fi; \
	if printf '%s\n' "$$kira_tree" | grep -Eq 'kira-api-docs|xtask'; then \
		echo "Unexpected dependency in kira"; \
		exit 1; \
	fi

# Formatting & linting
fmt:
	$(CARGO) fmt

clippy:
	$(CARGO) clippy

# Cleanup
clean:
	$(CARGO) clean
	rm -rf node_modules apps/dashboard/dist
