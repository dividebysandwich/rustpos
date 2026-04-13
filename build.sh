#!/bin/bash
set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Installing tools...${NC}"
command -v cargo-leptos >/dev/null 2>&1 || cargo install cargo-leptos
rustup target add wasm32-unknown-unknown

echo -e "${BLUE}Building with cargo-leptos...${NC}"
cargo leptos build --release

echo -e "${BLUE}Building print client...${NC}"
cargo build --release -p rustpos-printclient

echo -e "${BLUE}Preparing output directory...${NC}"
mkdir -p rustpos/data
cp target/release/rustpos rustpos/rustpos
cp target/release/rustpos-printclient rustpos/rustpos-printclient
cp printclient/printclient.toml rustpos/printclient.toml.example
cp -r site rustpos/site

echo -e "${GREEN}Build complete!${NC}"
echo -e "${GREEN}Run server with: cd rustpos && ./rustpos${NC}"
echo -e "${GREEN}Run print client with: ./rustpos-printclient printclient.toml${NC}"
