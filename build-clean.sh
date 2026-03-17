#!/bin/bash
set -e

# ANSI colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Laconote Desktop Clean Build Script ===${NC}"

# Navigate to the script directory
cd "$(dirname "$0")"

echo -e "\n${YELLOW}1. Cleaning previous build artifacts...${NC}"
rm -rf dist
rm -rf src-tauri/target/release/bundle/macos/Laconote.app
echo -e "${GREEN}✓ Cleaned${NC}"

echo -e "\n${YELLOW}2. Building Frontend (Vite & React)...${NC}"
npm run build
echo -e "${GREEN}✓ Frontend built successfully${NC}"

echo -e "\n${YELLOW}3. Building Desktop App (Tauri & Rust)...${NC}"
export PATH="$HOME/.cargo/bin:$PATH"
npm run tauri build
echo -e "${GREEN}✓ Tauri build completed${NC}"

APP_BUNDLE="src-tauri/target/release/bundle/macos/Laconote.app"
VERSION=$(grep -o '"version": "[^"]*"' src-tauri/tauri.conf.json | head -1 | cut -d'"' -f4)
RAW_ARCH=$(uname -m)
case "$RAW_ARCH" in
    arm64) ARCH="aarch64" ;;
    x86_64) ARCH="x86_64" ;;
    *) ARCH="$RAW_ARCH" ;;
esac

if [ -d "$APP_BUNDLE" ]; then
    echo -e "\n${YELLOW}4. Re-registering with macOS Launch Services...${NC}"
    /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP_BUNDLE"
    echo -e "${GREEN}✓ Registered${NC}"
    
    echo -e "\n${GREEN}=== Build Successful! ===${NC}"
    echo -e "You can find your DMG at: ${BLUE}src-tauri/target/release/bundle/dmg/${NC}"
    
    DMG_FILE="src-tauri/target/release/bundle/dmg/Laconote_${VERSION}_${ARCH}.dmg"
    if [ -f "$DMG_FILE" ]; then
        open -R "$DMG_FILE"
    else
        echo -e "${YELLOW}DMG not found at expected path: ${DMG_FILE}${NC}"
        echo -e "${YELLOW}Listing available DMGs:${NC}"
        ls -1 src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null || echo "No DMGs found"
    fi
else
    echo -e "\n${RED}✗ Error: Laconote.app was not found after build.${NC}"
    exit 1
fi
