#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
wasm-pack build crates/wasm --target web --out-dir "$(pwd)/web/src/wasm" --out-name osm
echo "wasm built to web/src/wasm"
