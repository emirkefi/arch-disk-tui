#!/usr/bin/env bash
set -e

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${CYAN}=== ⚡ Installing arch-disk-tui ===${NC}"

# Check for cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed. Please install Rust first: https://rustup.rs${NC}"
    exit 1
fi

echo -e "${YELLOW}Building optimized release binary...${NC}"
cargo build --release

# Determine installation directory
INSTALL_DIR=""
if [ -d "$HOME/.local/bin" ] && [[ ":$PATH:" == *":$HOME/.local/bin:"* ]]; then
    INSTALL_DIR="$HOME/.local/bin"
elif [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -d "$HOME/.cargo/bin" ]; then
    INSTALL_DIR="$HOME/.cargo/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
    echo -e "${YELLOW}Note: Added $INSTALL_DIR. Ensure it is in your PATH.${NC}"
fi

cp target/release/arch-disk-tui "$INSTALL_DIR/arch-disk-tui"
chmod +x "$INSTALL_DIR/arch-disk-tui"

echo -e "${GREEN}✓ Successfully installed arch-disk-tui to $INSTALL_DIR/arch-disk-tui!${NC}"
echo -e "${CYAN}Run it now with: ${GREEN}arch-disk-tui${NC}"
