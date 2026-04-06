#!/bin/bash
# dev.sh - Development mode with hot reload
set -e

command -v cargo-leptos >/dev/null 2>&1 || cargo install cargo-leptos
rustup target add wasm32-unknown-unknown

cargo leptos watch
