#!/usr/bin/env bash
set -e

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

REPO="emirkefi/arch-disk-tui"
BIN_NAME="arch-disk-tui"

echo -e "${CYAN}=== ⚡ Installing ${BIN_NAME} ===${NC}"

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
    echo -e "${YELLOW}Note: Created $INSTALL_DIR. Ensure it is in your PATH.${NC}"
fi

# 1. If run inside a local checkout of the repository, build directly
if [ -f "Cargo.toml" ] && grep -q 'name = "arch-disk-tui"' Cargo.toml 2>/dev/null; then
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: Cargo is not installed. Please install Rust first: https://rustup.rs${NC}"
        exit 1
    fi
    echo -e "${YELLOW}Building optimized release binary from local source...${NC}"
    cargo build --release
    cp "target/release/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"
    echo -e "${GREEN}✓ Installed ${BIN_NAME} to ${INSTALL_DIR}/${BIN_NAME}${NC}"
    exit 0
fi

# 2. Try downloading prebuilt release binary from GitHub Releases
ARCH="$(uname -m)"
LATEST_URL="https://github.com/${REPO}/releases/latest/download/${BIN_NAME}-linux-${ARCH}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo -e "${YELLOW}Checking for prebuilt binary (${ARCH})...${NC}"
if curl -fsSL "$LATEST_URL" -o "${TMP_DIR}/${BIN_NAME}" 2>/dev/null; then
    mv "${TMP_DIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"
    echo -e "${GREEN}✓ Installed prebuilt ${BIN_NAME} to ${INSTALL_DIR}/${BIN_NAME}!${NC}"
    echo -e "${CYAN}Run it now with: ${GREEN}${BIN_NAME}${NC}"
    exit 0
fi

# 3. Fallback: clone master branch into temp directory and build with Cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: No prebuilt binary found for ${ARCH} and Cargo is not installed.${NC}"
    echo -e "${YELLOW}Install Rust via https://rustup.rs and re-run.${NC}"
    exit 1
fi

echo -e "${YELLOW}Cloning ${REPO} (master) and building release binary...${NC}"
git clone --depth 1 -b master "https://github.com/${REPO}.git" "${TMP_DIR}/repo"
cargo build --release --manifest-path "${TMP_DIR}/repo/Cargo.toml"
cp "${TMP_DIR}/repo/target/release/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"

echo -e "${GREEN}✓ Successfully installed ${BIN_NAME} to ${INSTALL_DIR}/${BIN_NAME}!${NC}"
echo -e "${CYAN}Run it now with: ${GREEN}${BIN_NAME}${NC}"
