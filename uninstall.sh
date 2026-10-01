#!/usr/bin/env bash
set -e

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${CYAN}=== ⚡ Uninstalling arch-disk-tui ===${NC}"

TARGET_LOCATIONS=(
    "$HOME/.local/bin/arch-disk-tui"
    "/usr/local/bin/arch-disk-tui"
    "/usr/bin/arch-disk-tui"
    "$HOME/.cargo/bin/arch-disk-tui"
)

FOUND=0

for loc in "${TARGET_LOCATIONS[@]}"; do
    if [ -f "$loc" ]; then
        if [ -w "$loc" ]; then
            rm -f "$loc"
            echo -e "${GREEN}✓ Removed: $loc${NC}"
            FOUND=1
        else
            echo -e "${YELLOW}Need sudo permissions to remove: $loc${NC}"
            sudo rm -f "$loc"
            echo -e "${GREEN}✓ Removed (via sudo): $loc${NC}"
            FOUND=1
        fi
    fi
done

# Also check which arch-disk-tui in PATH in case installed elsewhere
WHICH_PATH=$(command -v arch-disk-tui 2>/dev/null || true)
if [ -n "$WHICH_PATH" ] && [ -f "$WHICH_PATH" ]; then
    if [ -w "$WHICH_PATH" ]; then
        rm -f "$WHICH_PATH"
        echo -e "${GREEN}✓ Removed: $WHICH_PATH${NC}"
        FOUND=1
    else
        echo -e "${YELLOW}Need sudo permissions to remove: $WHICH_PATH${NC}"
        sudo rm -f "$WHICH_PATH"
        echo -e "${GREEN}✓ Removed (via sudo): $WHICH_PATH${NC}"
        FOUND=1
    fi
fi

if [ $FOUND -eq 1 ]; then
    echo -e "${GREEN}✓ arch-disk-tui has been completely uninstalled.${NC}"
else
    echo -e "${YELLOW}No installed arch-disk-tui binary found.${NC}"
fi
