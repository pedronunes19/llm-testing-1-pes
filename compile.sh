#!/usr/bin/env bash
set -euo pipefail

# Ensure Cargo / Rust toolchain is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Move to the script's directory (repo root)
cd "$(dirname "$0")"

MODE="${1:-release}"

echo "⚽ Compiling Vibe PES 2019 for WebAssembly ($MODE mode)..."

if [ "$MODE" = "release" ] || [ "$MODE" = "--release" ]; then
    cargo build --release --target wasm32-unknown-unknown
    WASM_PATH="target/wasm32-unknown-unknown/release/vibe-pes2019.wasm"
else
    cargo build --target wasm32-unknown-unknown
    WASM_PATH="target/wasm32-unknown-unknown/debug/vibe-pes2019.wasm"
fi

echo "📦 Generating JavaScript & WASM bindings..."
mkdir -p web/pkg
wasm-bindgen "$WASM_PATH" \
    --out-dir web/pkg \
    --target web \
    --no-typescript

echo "✅ Compilation complete! Web artifacts ready in web/pkg/"
