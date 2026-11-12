CARGO := cargo

.PHONY: all build dev release build-cli build-all install install-hooks uninstall-hooks dashboard openapi test check clean fmt clippy

all: build

build:
	$(CARGO) build -p kira

dev: node_modules
	mprocs --names vite,kira,kira-cli \
		"npm run dev -w @kira/dashboard" \
		"cargo watch -w crates -x 'run -p kira'" \
		"cargo watch -w crates -x 'run -p kira-cli'"

release:
	$(CARGO) build --release -p kira

node_modules: package.json package-lock.json
	npm ci

install: node_modules

install-hooks:
	git config core.hooksPath .githooks
	chmod +x .githooks/pre-commit
	@echo "Installed Git hooks from .githooks"

uninstall-hooks:
	git config --unset core.hooksPath || true
	@echo "Uninstalled Git hooks"

dashboard: node_modules
	npm run build -w @kira/dashboard

build-cli: dashboard
	$(CARGO) build --release -p kira-cli

build-all: dashboard
	$(CARGO) build --release -p kira -p kira-cli

openapi: node_modules
	$(CARGO) xtask openapi generate
	npm run gen:api-types

test:
	$(CARGO) test --workspace

check: node_modules
	$(CARGO) fmt --check
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	$(CARGO) test --workspace
	$(CARGO) xtask openapi check
	npm run check:api-types
	npm run typecheck
	npm run test -w @kira/dashboard
	@$(CARGO) tree -p kira-cli >/dev/null
	@$(CARGO) tree -p kira --all-features >/dev/null
	@! $(CARGO) tree -p kira-cli | grep -q kira-server
	@! $(CARGO) tree -p kira-cli | grep -q kira-bot
	@! $(CARGO) tree -p kira-cli | grep -q serenity
	@! $(CARGO) tree -p kira --all-features | grep -q kira-api-docs
	@! $(CARGO) tree -p kira --all-features | grep -q xtask

fmt:
	$(CARGO) fmt

clippy:
	$(CARGO) clippy

clean:
	$(CARGO) clean
	rm -rf node_modules apps/dashboard/dist
