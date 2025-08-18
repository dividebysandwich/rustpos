#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Installing tools...${NC}"
command -v trunk >/dev/null 2>&1 || cargo install trunk
rustup target add wasm32-unknown-unknown

echo -e "${BLUE}Building frontend...${NC}"
cd frontend
trunk build --release #--public-url /
cd ..

echo -e "${BLUE}Copying frontend files to backend...${NC}"
mkdir -p rustpos/data
rm -rf rustpos/static
cp -r frontend/dist rustpos/static
echo "Frontend files copied to rustpos/static"

echo -e "${BLUE}Building backend...${NC}"
cd backend
cargo build --release
cd ..
cp target/release/rustpos-backend rustpos/rustpos

echo -e "${GREEN}Build complete!${NC}"
echo -e "${GREEN}Binary location: rustpos/rustpos${NC}"
echo -e "${GREEN}Run with: cd rustpos && ./rustpos${NC}"
