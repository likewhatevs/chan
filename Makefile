.PHONY: run build check clean

run:
	cd src-tauri && cargo tauri dev

build:
	cd src-tauri && cargo tauri build

check:
	cd src-tauri && cargo check

clean:
	cd src-tauri && cargo clean
