NIGHTLY=nightly-2025-04-29

.PHONY: fmt check test build clippy build all-checks fmt-and-checks

setup-env:
	rustup toolchain install $(NIGHTLY) --component rustfmt,clippy
	cargo install cargo-nextest --locked
	cargo install cargo-machete --version 0.8.0 --locked
	cargo install taplo-cli --locked

format-tomls:
	taplo format

lint-tomls:
	taplo check

unused-deps:
	cargo machete --with-metadata

fix-unused-deps:
	cargo machete --fix

fmt: format-tomls
	cargo +$(NIGHTLY) fmt --all

check:
	cargo +$(NIGHTLY) fmt --all --check

clippy:
	cargo clippy --all-targets --all-features -- --no-deps -D warnings

test:
	cargo nextest run --all-targets --all-features --no-tests pass

build:
	cargo build

build-release:
	cargo build --release

all-checks: check clippy test build unused-deps lint-tomls

fix-all: fmt fix-unused-deps all-checks

