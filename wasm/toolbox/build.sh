#!/bin/bash
set -euo pipefail

# Build the WASM toolbox
echo "Building WASM toolbox..."
cargo build --target wasm32-wasip1 --release --manifest-path="$(dirname "$0")/Cargo.toml"

WASM_PATH="$(dirname "$0")/target/wasm32-wasip1/release/toolbox.wasm"
echo "Built: $WASM_PATH"
echo "Size: $(du -h "$WASM_PATH" | cut -f1)"

# Optimize with wasm-opt if available
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing with wasm-opt..."
    wasm-opt -Oz "$WASM_PATH" -o "$WASM_PATH.opt"
    mv "$WASM_PATH.opt" "$WASM_PATH"
    echo "Optimized size: $(du -h "$WASM_PATH" | cut -f1)"
else
    echo "wasm-opt not found, skipping optimization"
fi
