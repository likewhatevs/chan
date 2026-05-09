.PHONY: run build check fmt fmt-check lint test clean

run:
	cd src-tauri && cargo tauri dev

build:
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
