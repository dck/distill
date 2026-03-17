.PHONY: build release install uninstall test lint fmt check clean

build:
	cargo build

release:
	cargo build --release

install: release
	cargo install --path .

uninstall:
	cargo uninstall distill

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

check: fmt lint test

clean:
	cargo clean
