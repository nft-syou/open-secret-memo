#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

STAGE="$(mktemp -d)"
KIT="$STAGE/open-secret-memo-recovery-kit"
mkdir -p "$KIT"

# 1. Build the wasm for plain web (no bundler) into the kit.
wasm-pack build crates/wasm --target web --out-dir "$KIT" --out-name osm
# wasm-pack writes package.json/.gitignore we do not need in the kit.
rm -f "$KIT/package.json" "$KIT/.gitignore" "$KIT/README.md"

# 2. Copy the recovery page and docs.
cp recovery/index.html recovery/app.js recovery/README_RESTORE.md "$KIT/"
cp spec/SPEC.md spec/test-vector.json "$KIT/"

# 3. Zip it.
mkdir -p dist
( cd "$STAGE" && zip -r -q open-secret-memo-recovery-kit.zip open-secret-memo-recovery-kit )
mv "$STAGE/open-secret-memo-recovery-kit.zip" dist/
rm -rf "$STAGE"
echo "built dist/open-secret-memo-recovery-kit.zip"
