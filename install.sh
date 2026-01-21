#!/usr/bin/env bash
# File: install.sh
# Usage: curl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.sh | bash

set -euo pipefail

REPO_OWNER="FurqanHun"
REPO_NAME="mpv-music"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"
API_URL="https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}🎧 mpv-music Installer${NC}"

# --- 1. Interactive Path Selection ---
echo -e "\nWhere would you like to install the script?"
echo -e "Default: ${GREEN}$DEFAULT_INSTALL_DIR${NC} (Recommended)"
read -rp "Press ENTER to use default, or type a custom path: " USER_INPUT

INSTALL_DIR="${USER_INPUT:-$DEFAULT_INSTALL_DIR}"

# Expand ~ if user typed it manually
INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"
# FIX: Remove trailing slash so PATH check matches correctly
INSTALL_DIR="${INSTALL_DIR%/}"

echo -e "${BLUE}[INFO]${NC} Installing to: $INSTALL_DIR"

# Create dir if missing
if [[ ! -d "$INSTALL_DIR" ]]; then
    echo -e "${YELLOW}[WARN]${NC} Directory does not exist. Creating it..."
    mkdir -p "$INSTALL_DIR" || {
        echo -e "${RED}[ERROR]${NC} Failed to create directory. Do you need sudo?"
        exit 1
    }
fi

# Check write permissions
if [[ ! -w "$INSTALL_DIR" ]]; then
    echo -e "${RED}[ERROR]${NC} You do not have write permissions for $INSTALL_DIR."
    echo "Please run this script with sudo, or pick a folder you own (like ~/.local/bin)."
    exit 1
fi

# --- 2. Dependency Check ---
echo -e "\n${BLUE}[INFO]${NC} Checking dependencies..."
MISSING_DEPS=()
# 'jq' is mandatory for the script to function
for dep in mpv curl jq fzf find; do
    if ! command -v "$dep" &>/dev/null; then
        MISSING_DEPS+=("$dep")
    fi
done

if [[ ${#MISSING_DEPS[@]} -gt 0 ]]; then
    echo -e "${RED}[ERROR] Missing dependencies: ${MISSING_DEPS[*]}${NC}"
    echo "Please install them via your package manager and run this installer again."
    exit 1
fi

# --- 3. Fetch Latest Version ---
echo -e "\n${BLUE}[INFO]${NC} Fetching latest version info..."
# Get the first tag from the list (works for Pre-releases too)
LATEST_JSON=$(curl -sL "$API_URL")
LATEST_TAG=$(echo "$LATEST_JSON" | jq -r '.[0].tag_name // empty')

if [[ -z "$LATEST_TAG" || "$LATEST_TAG" == "null" ]]; then
    echo -e "${RED}[ERROR]${NC} Could not find any releases on GitHub."
    exit 1
fi

echo -e "${GREEN}[OK]${NC} Found version: $LATEST_TAG"

# --- 4. Download File ---
BASE_URL="https://raw.githubusercontent.com/$REPO_OWNER/$REPO_NAME/$LATEST_TAG"

echo "Downloading mpv-music..."
if curl -sL "$BASE_URL/mpv-music" -o "$INSTALL_DIR/mpv-music"; then
    chmod +x "$INSTALL_DIR/mpv-music"
    echo -e "${GREEN}[OK]${NC} Script installed."
else
    echo -e "${RED}[ERROR]${NC} Download failed."
    exit 1
fi

# --- 5. PATH Check ---
echo -e "\n${BLUE}[INFO]${NC} Verifying PATH..."
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}[WARN] $INSTALL_DIR is NOT in your PATH.${NC}"
    echo "To run 'mpv-music' from anywhere, add this to your shell config (~/.bashrc or ~/.zshrc):"
    echo -e "\n    export PATH=\"$INSTALL_DIR:\$PATH\"\n"
    echo "Then run: source ~/.bashrc"
else
    echo -e "${GREEN}[OK]${NC} Install directory is in your PATH."
fi

echo -e "\n${GREEN}✅ Installation complete!${NC}"
echo "Try running: mpv-music"
