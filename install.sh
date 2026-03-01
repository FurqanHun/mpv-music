#!/usr/bin/env bash
# File: install.sh

set -euo pipefail

REPO_OWNER="FurqanHun"
REPO_NAME="mpv-music"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"
DEFAULT_CONFIG_DIR="$HOME/.config/mpv-music"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# --- 0. Parse Arguments ---
DEV_MODE=false
for arg in "$@"; do
    if [[ "$arg" == "--dev" ]]; then
        DEV_MODE=true
        break
    fi
done

echo -e "${BLUE}🎧 mpv-music Rust Installer${NC}"

if ! command -v jq &>/dev/null; then
    echo -e "${RED}[ERROR]${NC} 'jq' is not installed. It is required for the installer to parse release data."
    echo -e "Please install it via your package manager (e.g., 'sudo dnf install jq' or 'brew install jq')."
    exit 1
fi

# --- 1. System Detection ---
OS_TYPE=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH_RAW=$(uname -m)

case "$ARCH_RAW" in
    x86_64)        ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    armv7*)        ARCH="armv7" ;;
    *)             ARCH="unknown" ;;
esac

case "$OS_TYPE" in
    linux*)
        if [[ "$ARCH" == "armv7" ]]; then
            PLATFORM="unknown-linux-musleabihf"
        else
            PLATFORM="unknown-linux-musl"
        fi
        ;;
    darwin*) PLATFORM="apple-darwin" ;;
    *)       PLATFORM="unknown" ;;
esac

# --- 2. Path Selection ---
EXISTING_PATH=$(command -v mpv-music || echo "")

if [[ -n "$EXISTING_PATH" ]]; then
    INSTALL_DIR=$(dirname "$EXISTING_PATH")
    echo -e "${GREEN}[OK]${NC} Using existing installation directory: $INSTALL_DIR"
else
    echo -e "\nWhere would you like to install the binary?"
    read -rp "Installation directory [$DEFAULT_INSTALL_DIR]: " USER_INPUT < /dev/tty
    INSTALL_DIR="${USER_INPUT:-$DEFAULT_INSTALL_DIR}"
fi

INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"
mkdir -p "$INSTALL_DIR"

INSTALLED_BINARY="$INSTALL_DIR/mpv-music"

# --- 3. Fetch Release and Asset ---
echo -e "\n${BLUE}[INFO]${NC} Fetching release info..."
if [[ "$DEV_MODE" == "true" ]]; then
    API_ENDPOINT="https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases"
    LATEST_JSON=$(curl -sL "$API_ENDPOINT" | jq '.[0]')
else
    API_ENDPOINT="https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/latest"
    LATEST_JSON=$(curl -sL "$API_ENDPOINT")
fi

LATEST_TAG=$(echo "$LATEST_JSON" | jq -r ".tag_name // empty")
ASSET_URL=$(echo "$LATEST_JSON" | jq -r ".assets[] | select(.name | contains(\"$ARCH\") and contains(\"$PLATFORM\")) | .browser_download_url" 2>/dev/null || echo "")

# --- 4. Install Logic ---
if [[ -n "$ASSET_URL" && "$ASSET_URL" != "null" ]]; then
    echo -e "${GREEN}[OK]${NC} Found pre-compiled binary for $ARCH-$PLATFORM ($LATEST_TAG)"
    TEMP_DIR=$(mktemp -d)
    curl -sL "$ASSET_URL" -o "$TEMP_DIR/mpv-music.tar.gz"
    tar -xzf "$TEMP_DIR/mpv-music.tar.gz" -C "$TEMP_DIR"
    BINARY_SOURCE=$(find "$TEMP_DIR" -type f -name "mpv-music" | head -n 1)
    mv "$BINARY_SOURCE" "$INSTALLED_BINARY"
    rm -rf "$TEMP_DIR"
else
    # Dev fallback: manual compilation
    echo -e "${YELLOW}[WARN]${NC} No pre-compiled binary found for your system ($ARCH_RAW-$OS_TYPE)."
    read -rp "Compile from source now? [y/N]: " BUILD_CHOICE < /dev/tty
    if [[ "$BUILD_CHOICE" =~ ^[Yy]$ ]]; then
        command -v cargo &>/dev/null || { echo -e "${RED}[ERROR]${NC} Cargo not found."; exit 1; }
        echo -e "${BLUE}[INFO]${NC} Compiling mpv-music via cargo..."
        cargo install --git "https://github.com/$REPO_OWNER/$REPO_NAME" --root "$(dirname "$INSTALL_DIR")"
        mv "$(dirname "$INSTALL_DIR")/bin/mpv-music" "$INSTALLED_BINARY"
    else
        echo -e "${RED}[ERROR]${NC} Installation aborted."
        exit 1
    fi
fi

chmod +x "$INSTALLED_BINARY"
echo -e "${GREEN}[OK]${NC} mpv-music installed to $INSTALLED_BINARY"

# --- 5. Initial Configuration ---
echo -e "\n${BLUE}[INFO]${NC} Initial Setup"
echo "Would you like to add music directories now?"
read -rp "[y/N]: " SETUP_CHOICE < /dev/tty

if [[ "$SETUP_CHOICE" =~ ^[Yy]$ ]]; then
    COLLECTED_PATHS=()
    while true; do
        echo -e "\nEnter full path (or ENTER to finish):"
        read -rp "> " MUSIC_PATH < /dev/tty
        [[ -z "$MUSIC_PATH" ]] && break
        CLEAN_PATH=$(echo "$MUSIC_PATH" | sed -E "s/^['\"]|['\"]$//g")
        if [[ -d "$CLEAN_PATH" ]]; then
            COLLECTED_PATHS+=("$CLEAN_PATH")
            echo -e "${GREEN}[QUEUED]${NC} $CLEAN_PATH"
        else
            echo -e "${RED}[ERROR]${NC} Directory not found: $CLEAN_PATH"
        fi
    done
    if [[ ${#COLLECTED_PATHS[@]} -gt 0 ]]; then
        "$INSTALLED_BINARY" --add-dir "${COLLECTED_PATHS[@]}"
    fi
fi

# --- 6. PATH Verification ---
case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        echo -e "\n${GREEN}Installation complete!${NC} Run 'mpv-music' to start."
        ;;
    *)
        echo -e "\n${YELLOW}[WARNING]${NC} $INSTALL_DIR is not in your PATH."
        echo -e "Please add it to your shell configuration (e.g., .bashrc or .zshrc):"
        echo -e "    export PATH=\"\$PATH:$INSTALL_DIR\""
        echo -e "\nAfter adding it, run 'mpv-music' to start."
        ;;
esac
