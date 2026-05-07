# chan top-level Makefile.
#
# Convenience wrappers for the common workflows. The actual gates
# (CI, pre-push hook) call cargo / npm directly; the Makefile is
# for the development shell.
#
# Targets:
#   make           default; builds frontend + binary (alias for `all`)
#   make all       same as `make`
#   make web       npm install + npm run build (frontend bundle)
#   make build     cargo build --release -p chan
#   make test      cargo test --workspace
#   make lint      cargo fmt + cargo clippy (mirrors pre-push)
#   make hooks     install the pre-push git hook (one-time)
#   make install   copy the release binary to PREFIX/bin
#   make uninstall remove it from PREFIX/bin
#   make clean     wipe target/, web/dist/, web/node_modules/
#   make dev       run `chan serve /tmp/chan-dev --no-token` against
#                  a fresh dev drive
#
# PREFIX defaults to /usr/local. Override per invocation:
#   make install PREFIX=$$HOME/.local

PREFIX ?= /usr/local
CARGO ?= cargo
NPM ?= npm

BIN := target/release/chan

.PHONY: all
all: web build

.PHONY: web
web:
	cd web && $(NPM) install && $(NPM) run build

.PHONY: build
build:
	$(CARGO) build --release -p chan

.PHONY: test
test:
	$(CARGO) test --workspace

.PHONY: lint
lint:
	$(CARGO) fmt --check
	$(CARGO) clippy --all-targets -- -D warnings

.PHONY: hooks
hooks:
	./scripts/install-hooks

.PHONY: install
install: build
	install -d $(PREFIX)/bin
	install -m 755 $(BIN) $(PREFIX)/bin/chan
	@echo "installed to $(PREFIX)/bin/chan"

.PHONY: uninstall
uninstall:
	rm -f $(PREFIX)/bin/chan
	@echo "removed $(PREFIX)/bin/chan"

.PHONY: clean
clean:
	$(CARGO) clean
	rm -rf web/dist web/node_modules web/pkg

.PHONY: dev
dev: build
	$(BIN) serve /tmp/chan-dev --no-token

.PHONY: help
help:
	@awk 'BEGIN{FS=":.*?##"} /^[a-zA-Z_-]+:.*?##/ {printf "  %-12s %s\n", $$1, $$2}' $(MAKEFILE_LIST) || \
		grep -E '^[a-zA-Z_-]+:' $(MAKEFILE_LIST) | sed 's/:.*//' | sort -u
