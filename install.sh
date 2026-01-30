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
INSTALLED_SCRIPT="$INSTALL_DIR/mpv-music"

echo "Downloading mpv-music..."
if curl -sL "$BASE_URL/mpv-music" -o "$INSTALLED_SCRIPT"; then
    chmod +x "$INSTALLED_SCRIPT"
    echo -e "${GREEN}[OK]${NC} Script installed."
else
    echo -e "${RED}[ERROR]${NC} Download failed."
    exit 1
fi

# --- 5. Download Rust Indexer (Monke Engine) ---
ARCH=$(uname -m)

if [[ "$ARCH" == "x86_64" ]]; then
    echo -e "\n${BLUE}[OPTIONAL]${NC} Install High-Performance Indexer?"
    echo -e "The Rust-based indexer is ${GREEN}significantly faster${NC}."
    read -rp "Install pre-compiled binary? [Y/n]: " RUST_CHOICE < /dev/tty
    RUST_CHOICE=${RUST_CHOICE:-Y}

    if [[ "$RUST_CHOICE" =~ ^[Yy]$ ]]; then
        # Default config dir logic matches your script
        CONFIG_DIR="$HOME/.config/mpv-music"

        echo -e "\n${BLUE}[QUESTION]${NC} Where should the Indexer Binary be placed?"
        echo -e "  [1] ${GREEN}$INSTALL_DIR${NC} (Recommended - Same as script)"
        echo -e "  [2] ${YELLOW}$CONFIG_DIR${NC} (Project Config Folder)"
        read -rp "Select [1/2] (Default: 1): " BIN_LOC_CHOICE < /dev/tty
        BIN_LOC_CHOICE=${BIN_LOC_CHOICE:-1}

        if [[ "$BIN_LOC_CHOICE" == "2" ]]; then
            TARGET_BIN_DIR="$CONFIG_DIR"
        else
            TARGET_BIN_DIR="$INSTALL_DIR"
        fi

        mkdir -p "$TARGET_BIN_DIR"

        # We download the specific x86 asset, but save it as the generic name
        RELEASE_ASSET_NAME="mpv-music-indexer-linux-x86_64"
        LOCAL_BINARY_NAME="mpv-music-indexer"
        BINARY_URL="https://github.com/$REPO_OWNER/$REPO_NAME/releases/download/$LATEST_TAG/$RELEASE_ASSET_NAME"
        DESTINATION_PATH="$TARGET_BIN_DIR/$LOCAL_BINARY_NAME"

        echo "Downloading MPV Music Indexer ($RELEASE_ASSET_NAME)..."

        if curl -sL --fail "$BINARY_URL" -o "$DESTINATION_PATH"; then
            chmod +x "$DESTINATION_PATH"
            echo -e "${GREEN}[OK]${NC} Indexer installed to $DESTINATION_PATH"
        else
            echo -e "${YELLOW}[WARN]${NC} Download failed (Asset '$RELEASE_ASSET_NAME' not found)."
            echo "Falling back to Bash logic."
        fi
    else
        echo -e "${YELLOW}[INFO]${NC} Skipping binary installation."
    fi
else
    # Non-x86_64 handling
    echo -e "\n${YELLOW}[INFO]${NC} Architecture detected: $ARCH"
    echo "The pre-compiled indexer is currently x86_64 only."
    echo -e "If you want speed, build from source:"
    echo -e "${BLUE}  git clone https://github.com/$REPO_OWNER/$REPO_NAME${NC}"
    echo -e "${BLUE}  cd $REPO_NAME/crates/mpv-music-indexer && cargo install --path .${NC}"
fi

# --- 6. Initial Configuration (BATCH MODE) ---
echo -e "\n${BLUE}[INFO]${NC} Initial Setup"
echo "Would you like to add music directories now?"
read -rp "[y/N]: " SETUP_CHOICE < /dev/tty

if [[ "$SETUP_CHOICE" =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Tip:${NC} You can drag and drop folders into this terminal."

    # Array to store collected paths
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

    # If we collected any paths, process them all at once
    if [[ ${#COLLECTED_PATHS[@]} -gt 0 ]]; then
        echo -e "\n${BLUE}[INFO]${NC} Configuring mpv-music..."
        # Pass all collected paths as arguments to --add-dir
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

echo -e "\n${GREEN}Installation complete!${NC}"
echo "Run 'mpv-music' to start."
