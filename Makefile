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
#   make models    pre-fetch the default embedding model into
#                  crates/chan-server/resources/models.tar.zst so a
#                  `--features embed-model` build can bundle it.
#                  Idempotent; reads HTTPS_PROXY / HTTP_PROXY.
#                  ONLY needed for `make build-release`; plain
#                  `make build` skips the bundle (systacean-6 split).
#   make build     cargo build --release -p chan (~26 MB; no embedded
#                  model — runtime resolver looks for one at
#                  <user-config>/chan/models/ or surfaces a
#                  "model not downloaded" error on Hybrid search)
#   make build-release  models + web + `cargo build --release
#                  --features embed-model` (~89 MB; bundles the
#                  default BGE-small model so search works offline
#                  out of the box; matches pre-systacean-6 behaviour)
#   make test      cargo test --workspace
#   make lint      cargo fmt + cargo clippy (mirrors pre-push)
#   make hooks     install the pre-push git hook (one-time)
#   make install   build-release + copy the binary to PREFIX/bin
#                  (default PREFIX = $XDG_BIN_HOME or ~/.local; no
#                  sudo). Override for a system-wide install:
#                    make install PREFIX=/usr/local
#   make uninstall remove it from PREFIX/bin
#   make rpm       build an .rpm for RPM_TARGET (default
#                  x86_64-unknown-linux-musl, matches release.yml).
#                  Uses cargo-zigbuild so the host arch doesn't have
#                  to match the target arch. One-time setup:
#                    rustup target add $(RPM_TARGET)
#                    cargo install cargo-zigbuild cargo-generate-rpm
#                    # plus zig in PATH (https://ziglang.org)
#   make clean     wipe target/, web/dist/, web/node_modules/
#   make dev       run `chan serve /tmp/chan-dev --no-token` against
#                  a fresh dev drive
#
# PREFIX defaults to $XDG_BIN_HOME or ~/.local; the install target
# drops the binary at $(PREFIX)/bin/chan, so the default lands at
# ~/.local/bin/chan and avoids needing sudo. Override per
# invocation for a system-wide install:
#   make install PREFIX=/usr/local

PREFIX ?= $(if $(XDG_BIN_HOME),$(XDG_BIN_HOME:/bin=),$(HOME)/.local)
CARGO ?= cargo
NPM ?= npm
RPM_TARGET ?= x86_64-unknown-linux-musl

BIN := target/release/chan

.PHONY: all
all: web build

.PHONY: web
web:
	cd web && $(NPM) install && $(NPM) run build

.PHONY: build
build:
	$(CARGO) build --release -p chan

# Pre-fetch the default embedding model into chan-server's
# resources/models/. fetch-models is idempotent: re-runs with the
# model already cached are fast no-ops. Honors HTTPS_PROXY /
# HTTP_PROXY for restricted networks.
.PHONY: models
models:
	$(CARGO) run --release -p fetch-models

# One-shot release build that bundles the embedding model. systacean-6
# split this from the default `make build` path: the model add ~63 MB
# to the binary and most users either don't need Hybrid search or
# would rather fetch the model on demand (chan-drive runtime resolver
# + systacean-7 download flow). Use `make build-release` for
# distribution where Hybrid should work offline immediately; use
# `make build` for the default lean binary.
#
# `--features embed-model` implies `embeddings`; the workspace
# feature graph wires `chan-server/embed-model` →
# `dep:tar + dep:zstd + embeddings`, which is what `embed_seed.rs`
# needs for the bundle decode.
.PHONY: build-release
build-release: models web
	$(CARGO) build --release --features embed-model -p chan

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
install: build-release
	install -d $(PREFIX)/bin
	install -m 755 $(BIN) $(PREFIX)/bin/chan
	@echo "installed to $(PREFIX)/bin/chan"
	@case ":$$PATH:" in *":$(PREFIX)/bin:"*) ;; \
		*) echo "note: $(PREFIX)/bin is not in PATH; add it to your shell rc";; \
	esac

.PHONY: uninstall
uninstall:
	rm -f $(PREFIX)/bin/chan
	@echo "removed $(PREFIX)/bin/chan"

.PHONY: rpm
rpm: models web
	$(CARGO) zigbuild --release --features embed-model --target $(RPM_TARGET) -p chan
	# cargo-generate-rpm reads asset paths verbatim from the
	# [package.metadata.generate-rpm] block (../../target/release/chan)
	# and does not rewrite them when --target is passed, so stage
	# the cross-built binary at the un-prefixed location it expects.
	mkdir -p target/release
	cp target/$(RPM_TARGET)/release/chan target/release/chan
	# --auto-req no: skip the ldd-based shared-lib scan. Required when
	# cross-compiling (host ldd can't read foreign-arch ELF) and
	# correct anyway since musl binaries are statically linked.
	cd crates/chan && $(CARGO) generate-rpm --target $(RPM_TARGET) --auto-req no
	@find . -path '*/generate-rpm/*.rpm' -type f 2>/dev/null | head -1 | \
		xargs -I{} echo "built {}"

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
