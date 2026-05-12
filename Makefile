.PHONY: run build check fmt fmt-check lint test clean chan-bin

# Sibling chan repo. Override via CHAN_REPO=/path on CI.
CHAN_REPO ?= ../chan
TARGET_TRIPLE := $(shell rustc -vV | sed -n 's/host: //p')
CHAN_BIN := src-tauri/binaries/chan-$(TARGET_TRIPLE)

# Stage chan as the bundled sidecar. Rebuilds every invocation so
# in-progress chan edits flow into `make run` without a separate
# step; cargo's own incremental build keeps the cost small when
# nothing changed.
chan-bin:
	cd $(CHAN_REPO)/web && npm run build
	cd $(CHAN_REPO) && cargo build --release --bin chan
	mkdir -p src-tauri/binaries
	cp $(CHAN_REPO)/target/release/chan $(CHAN_BIN)

run: chan-bin
	cd src-tauri && cargo tauri dev

build: chan-bin
	cd src-tauri && cargo tauri build

check:
	cd src-tauri && cargo check

fmt:
	cd src-tauri && cargo fmt --all

fmt-check:
	cd src-tauri && cargo fmt --all -- --check

lint:
	cd src-tauri && cargo clippy --all-targets -- -D warnings

test:
	cd src-tauri && cargo test --all-targets

clean:
	cd src-tauri && cargo clean
	rm -rf src-tauri/binaries
