.PHONY: run build-wasm serve clean setup

# ── Development (native window) ──────────────────────────────────────
run:
	cargo run

# ── One-time setup for WASM toolchain ────────────────────────────────
setup:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-bindgen-cli

# ── Build WASM bundle ────────────────────────────────────────────────
build-wasm:
	./compile.sh release

# ── Build & serve locally ────────────────────────────────────────────
serve: build-wasm
	./serve.sh

clean:
	cargo clean
	rm -rf web/pkg
