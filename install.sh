#!/usr/bin/env bash
# File: install.sh
# Usage: curl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/mpv-music-sh-archive/install.sh | bash

set -euo pipefail

REPO_OWNER="FurqanHun"
REPO_NAME="mpv-music"
TARGET_VERSION="v0.23.5" # Locked Legacy Version

DEFAULT_INSTALL_DIR="$HOME/.local/bin"
DEFAULT_CONFIG_DIR="$HOME/.config/mpv-music"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}🎧 mpv-music (Legacy Bash) Installer${NC}"

# --- 0. Legacy Notice ---
echo -e "${YELLOW}======================================================${NC}"
echo -e "${YELLOW} NOTICE: This Bash version is ARCHIVED and DEPRECATED.${NC}"
echo -e " You are installing the final Bash release: ${GREEN}$TARGET_VERSION${NC}"
echo -e ""
echo -e " A complete rewrite in Rust (faster, cleaner) is available."
echo -e " We highly recommend installing that instead:"
echo -e " ${BLUE}https://github.com/FurqanHun/mpv-music${NC}"
echo -e "${YELLOW}======================================================${NC}"
echo ""
read -rp "Press ENTER to continue with Legacy Install (or Ctrl+C to abort)..."

# --- 1. Interactive Path Selection ---
echo -e "\nWhere would you like to install the script?"
echo -e "Default: ${GREEN}$DEFAULT_INSTALL_DIR${NC} (Recommended)"
read -rp "Press ENTER to use default, or type a custom path: " USER_INPUT < /dev/tty

INSTALL_DIR="${USER_INPUT:-$DEFAULT_INSTALL_DIR}"

# Expand ~ if user typed it manually
INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"
INSTALL_DIR="${INSTALL_DIR%/}"

echo -e "${BLUE}[INFO]${NC} Installing to: $INSTALL_DIR"

# Create dir if missing
if [[ ! -d "$INSTALL_DIR" ]]; then
    echo -e "${YELLOW}[WARN]${NC} Directory does not exist. Creating it..."
    mkdir -p "$INSTALL_DIR" || { echo -e "${RED}[ERROR]${NC} Failed to create directory."; exit 1; }
fi

# Check write permissions
if [[ ! -w "$INSTALL_DIR" ]]; then
    echo -e "${RED}[ERROR]${NC} You do not have write permissions for $INSTALL_DIR."
    echo "Please run this script with sudo, or pick a folder you own (like ~/.local/bin)."
    exit 1
fi

# --- 2. Existing Config Check ---
if [[ -d "$DEFAULT_CONFIG_DIR" ]]; then
    echo -e "\n${YELLOW}[WARN]${NC} Existing configuration/database found at: $DEFAULT_CONFIG_DIR"
    read -rp "Do you want to WIPE the existing config and index? [y/N]: " WIPE_CHOICE < /dev/tty

    if [[ "$WIPE_CHOICE" =~ ^[Yy]$ ]]; then
        rm -rf "$DEFAULT_CONFIG_DIR"
        echo -e "${GREEN}[OK]${NC} Configuration wiped."
    else
        echo -e "${BLUE}[INFO]${NC} Keeping existing configuration."
    fi
fi

# --- 3. Dependency Check ---
echo -e "\n${BLUE}[INFO]${NC} Checking dependencies..."
MISSING_DEPS=()
# 'jq' removed as we don't need to parse GitHub API for hardcoded versions
for dep in mpv curl fzf find; do
    if ! command -v "$dep" &>/dev/null; then
        MISSING_DEPS+=("$dep")
    fi
done

if [[ ${#MISSING_DEPS[@]} -gt 0 ]]; then
    echo -e "${RED}[ERROR] Missing dependencies: ${MISSING_DEPS[*]}${NC}"
    echo "Please install them via your package manager and run this installer again."
    exit 1
fi

# --- 4. Download Main Script ---
echo -e "\n${BLUE}[INFO]${NC} Downloading mpv-music $TARGET_VERSION..."

# Using raw.githubusercontent with the hardcoded tag
BASE_URL="https://raw.githubusercontent.com/$REPO_OWNER/$REPO_NAME/$TARGET_VERSION"
INSTALLED_SCRIPT="$INSTALL_DIR/mpv-music"

if curl -sL "$BASE_URL/mpv-music" -o "$INSTALLED_SCRIPT"; then
    chmod +x "$INSTALLED_SCRIPT"
    echo -e "${GREEN}[OK]${NC} Script installed."
else
    echo -e "${RED}[ERROR]${NC} Download failed. Check your internet connection."
    exit 1
fi

# --- 5. Download Optional Indexer (Legacy Monke Engine) ---
ARCH=$(uname -m)
ASSET_NAME=""

case "$ARCH" in
    x86_64)
        ASSET_NAME="mpv-music-indexer-linux-x86_64"
        ;;
    aarch64|arm64)
        ASSET_NAME="mpv-music-indexer-linux-aarch64"
        ;;
    armv7l|armv7)
        ASSET_NAME="mpv-music-indexer-linux-armv7"
        ;;
    *)
        ASSET_NAME="" # Unknown
        ;;
esac

if [[ -n "$ASSET_NAME" ]]; then
    echo -e "\n${BLUE}[OPTIONAL]${NC} Install Accelerated Indexer (Legacy Binary)?"
    echo -e "Detected Architecture: ${GREEN}$ARCH${NC}"
    read -rp "Install pre-compiled indexer? [Y/n]: " RUST_CHOICE < /dev/tty
    RUST_CHOICE=${RUST_CHOICE:-Y}

    if [[ "$RUST_CHOICE" =~ ^[Yy]$ ]]; then
        CONFIG_DIR="$HOME/.config/mpv-music"

        echo -e "\n${BLUE}[QUESTION]${NC} Where should the Indexer Binary be placed?"
        echo -e "  [1] ${GREEN}$INSTALL_DIR${NC} (Recommended)"
        echo -e "  [2] ${YELLOW}$CONFIG_DIR${NC} (Project Config Folder)"
        read -rp "Select [1/2] (Default: 1): " BIN_LOC_CHOICE < /dev/tty
        BIN_LOC_CHOICE=${BIN_LOC_CHOICE:-1}

        if [[ "$BIN_LOC_CHOICE" == "2" ]]; then
            TARGET_BIN_DIR="$CONFIG_DIR"
        else
            TARGET_BIN_DIR="$INSTALL_DIR"
        fi

        mkdir -p "$TARGET_BIN_DIR"

        LOCAL_BINARY_NAME="mpv-music-indexer"
        # Using the hardcoded TARGET_VERSION for release assets
        BINARY_URL="https://github.com/$REPO_OWNER/$REPO_NAME/releases/download/$TARGET_VERSION/$ASSET_NAME"
        DESTINATION_PATH="$TARGET_BIN_DIR/$LOCAL_BINARY_NAME"

        echo "Downloading Indexer..."

        if curl -sL --fail "$BINARY_URL" -o "$DESTINATION_PATH"; then
            chmod +x "$DESTINATION_PATH"
            echo -e "${GREEN}[OK]${NC} Indexer installed to $DESTINATION_PATH"
        else
            echo -e "${YELLOW}[WARN]${NC} Download failed (Asset not found for $TARGET_VERSION)."
            echo "Falling back to pure Bash indexing."
        fi
    else
        echo -e "${YELLOW}[INFO]${NC} Skipping binary installation."
    fi
else
    echo -e "\n${YELLOW}[INFO]${NC} Architecture '$ARCH' has no pre-compiled indexer."
    echo "Using pure Bash mode."
fi

# --- 6. Initial Configuration ---
echo -e "\n${BLUE}[INFO]${NC} Initial Setup"
echo "Would you like to add music directories now?"
read -rp "[y/N]: " SETUP_CHOICE < /dev/tty

if [[ "$SETUP_CHOICE" =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Tip:${NC} You can drag and drop folders into this terminal."

    COLLECTED_PATHS=()

    while true; do
        echo -e "\nEnter full path to music directory (or press ENTER to finish):"
        read -rp "> " MUSIC_PATH < /dev/tty

        if [[ -z "$MUSIC_PATH" ]]; then
            break
        fi

        # Clean quotes safely
        CLEAN_PATH=$(echo "$MUSIC_PATH" | sed -E "s/^['\"]|['\"]$//g")

        if [[ -d "$CLEAN_PATH" ]]; then
            COLLECTED_PATHS+=("$CLEAN_PATH")
            echo -e "${GREEN}[QUEUED]${NC} $CLEAN_PATH"
        else
            echo -e "${RED}[ERROR]${NC} Directory not found: $CLEAN_PATH"
        fi
    done

    if [[ ${#COLLECTED_PATHS[@]} -gt 0 ]]; then
        echo -e "\n${BLUE}[INFO]${NC} Configuring mpv-music..."
        if "$INSTALLED_SCRIPT" --add-dir "${COLLECTED_PATHS[@]}"; then
             echo -e "${GREEN}[OK]${NC} Configuration updated."
        else
             echo -e "${RED}[ERROR]${NC} Failed to update configuration."
        fi
    fi
fi

# --- 7. PATH Check ---
echo -e "\n${BLUE}[INFO]${NC} Verifying PATH..."
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}[WARN] $INSTALL_DIR is NOT in your PATH.${NC}"
    echo "To run 'mpv-music' from anywhere, add this to your shell config (~/.bashrc or ~/.zshrc):"
    echo -e "\n    export PATH=\"$INSTALL_DIR:\$PATH\"\n"
    echo "Then run: source ~/.bashrc"
else
    echo -e "${GREEN}[OK]${NC} Install directory is in your PATH."
fi

echo -e "\n${GREEN}Legacy Installation complete!${NC}"
echo "Run 'mpv-music' to start."
