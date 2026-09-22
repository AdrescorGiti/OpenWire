#!/bin/bash
# OpenWire packaging script.
#
# Flat layout: build the frontend assets, compile once with cargo, then
# makepkg just assembles files. No nested tarballs, no rebuild inside makepkg.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> Building frontend assets (tailwind + ES modules)..."
cd "$ROOT"
if command -v npm >/dev/null 2>&1; then
  [ -d node_modules ] || npm install --no-audit --no-fund
  # копирует src/js + src/fonts в ui/ и собирает ui/app.css из src/css
  npm run build
else
  echo "   npm not found — using prebuilt ui/ assets" >&2
fi

echo "==> Compiling release binary..."
cd "$ROOT/src-tauri"
cargo build --release --locked

echo "==> Assembling pkg.tar.zst..."
cd "$ROOT/packaging"
rm -rf pkg src *.pkg.tar.zst
makepkg -f

echo "==> Done:"
ls -lh ./*.pkg.tar.zst
