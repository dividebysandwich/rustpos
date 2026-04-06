#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Installing tools...${NC}"
command -v cargo-leptos >/dev/null 2>&1 || cargo install cargo-leptos
rustup target add wasm32-unknown-unknown

echo -e "${BLUE}Building with cargo-leptos...${NC}"
cargo leptos build --release

echo -e "${BLUE}Preparing output directory...${NC}"
mkdir -p rustpos/data
cp -r backend/data/* rustpos/data/ 2>/dev/null || true
cp -r target/site rustpos/site 2>/dev/null || true
cp target/server/release/rustpos rustpos/rustpos 2>/dev/null || \
  cp target/release/rustpos rustpos/rustpos 2>/dev/null || \
  echo "Note: Binary location may vary. Check target/ directory."

echo -e "${GREEN}Build complete!${NC}"
echo -e "${GREEN}Run with: cd rustpos && ./rustpos${NC}"
