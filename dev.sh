#!/bin/bash
# dev.sh - Development mode with hot reload
set -e

command -v cargo-leptos >/dev/null 2>&1 || cargo install cargo-leptos
rustup target add wasm32-unknown-unknown

mkdir -p data
# Copy data files if not present
[ -f data/logo_receipt.png ] || cp backend/data/logo_receipt.png data/ 2>/dev/null || true

cargo leptos watch
