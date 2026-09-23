#!/usr/bin/env bash
# Builds the web version into dist/, ready to be served as static assets.
#
# Needs: the wasm32-unknown-unknown target, wasm-bindgen-cli matching the wasm-bindgen
# version in Cargo.lock, and optionally wasm-opt (binaryen) for a smaller binary.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT=dist
WASM=target/wasm32-unknown-unknown/wasm-release/fractal3D.wasm
# Cloudflare Workers static assets are capped at 25 MiB per file
MAX_BYTES=$((25 * 1024 * 1024))

cargo build --profile wasm-release --target wasm32-unknown-unknown

rm -rf "$OUT"
wasm-bindgen --out-dir "$OUT" --out-name fractal3D --target web --no-typescript "$WASM"

if command -v wasm-opt >/dev/null; then
    wasm-opt -Oz --all-features "$OUT/fractal3D_bg.wasm" -o "$OUT/fractal3D_bg.wasm"
else
    echo "warning: wasm-opt not found, skipping size optimization" >&2
fi

cp web/index.html "$OUT/"
cp -r assets "$OUT/"

size=$(wc -c < "$OUT/fractal3D_bg.wasm")
echo "fractal3D_bg.wasm: $((size / 1024 / 1024)) MiB ($size bytes)"
if [ "$size" -gt "$MAX_BYTES" ]; then
    echo "error: wasm is over Cloudflare's 25 MiB asset limit" >&2
    exit 1
fi
