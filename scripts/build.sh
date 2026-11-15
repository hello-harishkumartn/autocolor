#!/usr/bin/env bash
# Builds the engine and assembles dist/ (to publish) and dist-site/ (same, with engine.wasm as base64
# for the site repo, whose deploy workflow decodes *.b64 files).
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

(cd engine && cargo build --release --target wasm32-unknown-unknown)
WASM=engine/target/wasm32-unknown-unknown/release/autocolor_engine.wasm
[ -x tools/node_modules/.bin/wasm-opt ] || (cd tools && npm install --silent)

rm -rf dist dist-site
mkdir -p dist/licenses
cp app/*.html app/*.css app/*.js dist/
cp LICENSE dist/licenses/LICENSE
{
  echo "Rust crates in the dependency tree of engine.wasm (name version licence). Link-time optimisation"
  echo "removes code the engine does not call, so not all of them end up in the binary."
  echo
  (cd engine && cargo tree --target wasm32-unknown-unknown -e normal --prefix none -f '{p} | {l}' | grep ' | ' | sed 's/ ([^)]*)//g' | sort -u)
} > THIRD_PARTY/rust-crates.txt
cp THIRD_PARTY/rust-crates.txt dist/licenses/rust-crates.txt
cp THIRD_PARTY/lightcraft/LICENSE-MIT THIRD_PARTY/lightcraft/LICENSE-APACHE THIRD_PARTY/lightcraft/NOTICE dist/licenses/
tools/node_modules/.bin/wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals "$WASM" -o dist/engine.wasm

cp -r dist dist-site
base64 -w 76 dist/engine.wasm > dist-site/engine.wasm.b64
rm dist-site/engine.wasm
base64 -d dist-site/engine.wasm.b64 | cmp - dist/engine.wasm
echo "engine.wasm: $(stat -c %s dist/engine.wasm) bytes; base64 round trip ok"
