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
UPDATE_MODE=false
TARGET_TAG=""
NO_VERIFY=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dev) DEV_MODE=true; shift ;;
        --update) UPDATE_MODE=true; shift ;;
        --tag) TARGET_TAG="$2"; shift 2 ;;
        --no-verify) NO_VERIFY=true; shift ;;
        *) shift ;;
    esac
done

if [[ "$UPDATE_MODE" == "true" ]]; then
    echo -e "${BLUE}🎧 mpv-music Updater${NC}"
else
    echo -e "${BLUE}🎧 mpv-music Rust Installer${NC}"
fi

if command -v curl >/dev/null 2>&1; then
    FETCH_CMD="curl -sL"
    DOWNLOAD_CMD="curl -f -# -L -o"
elif command -v wget >/dev/null 2>&1; then
    FETCH_CMD="wget -qO-"
    DOWNLOAD_CMD="wget -q --show-progress -O"
else
    echo -e "${RED}[ERROR]${NC} 'curl' or 'wget' is required."
    exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
    SHASUM_CMD="sha256sum -c"
elif command -v shasum >/dev/null 2>&1; then
    SHASUM_CMD="shasum -a 256 -c"
else
    SHASUM_CMD=""
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
    if [[ "$UPDATE_MODE" == "false" ]]; then
        echo -e "${GREEN}[OK]${NC} Using existing installation directory: $INSTALL_DIR"
    fi
else
    echo -e "\nWhere would you like to install the binary?"
    read -rp "Installation directory [$DEFAULT_INSTALL_DIR]: " USER_INPUT < /dev/tty
    INSTALL_DIR="${USER_INPUT:-$DEFAULT_INSTALL_DIR}"
fi

INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"
mkdir -p "$INSTALL_DIR"

INSTALLED_BINARY="$INSTALL_DIR/mpv-music"

# --- 3. Fetch Release and Asset ---
if [[ -n "$TARGET_TAG" ]]; then
    if [[ "$UPDATE_MODE" == "false" ]]; then
        echo -e "\n${BLUE}[INFO]${NC} Using provided tag: $TARGET_TAG"
    else
        echo -e "\n${BLUE}[INFO]${NC} Fetching update: $TARGET_TAG"
    fi
    LATEST_TAG="$TARGET_TAG"
else
    echo -e "\n${BLUE}[INFO]${NC} Fetching release info..."
    API_ENDPOINT="https://furqanhun.github.io/mpv-music/latest.json"
    LATEST_JSON=$($FETCH_CMD "$API_ENDPOINT")

    if [[ "$DEV_MODE" == "true" ]]; then
        LATEST_TAG=$(echo "$LATEST_JSON" | grep -o '"dev".*' | grep -o '"tag_name": *"[^"]*"' | head -n 1 | cut -d'"' -f4)
    else
        LATEST_TAG=$(echo "$LATEST_JSON" | sed 's/"dev".*//' | grep -o '"tag_name": *"[^"]*"' | head -n 1 | cut -d'"' -f4)
    fi
    
    if [[ -z "$LATEST_TAG" ]]; then
        echo -e "${RED}[ERROR]${NC} Failed to parse latest version from $API_ENDPOINT."
        exit 1
    fi
fi

ASSET_NAME="mpv-music-${LATEST_TAG}-${ARCH}-${PLATFORM}.tar.gz"
ASSET_URL="https://github.com/$REPO_OWNER/$REPO_NAME/releases/download/$LATEST_TAG/$ASSET_NAME"
CHECKSUM_URL="https://github.com/$REPO_OWNER/$REPO_NAME/releases/download/$LATEST_TAG/checksums.txt"

# --- 4. Install Logic ---
if [[ -n "$LATEST_TAG" ]]; then
    echo -e "${GREEN}[OK]${NC} Target version: $ARCH-$PLATFORM ($LATEST_TAG)"
    TEMP_DIR=$(mktemp -d)
    
    echo -e "${BLUE}[INFO]${NC} Downloading binary..."
    if ! $DOWNLOAD_CMD "$TEMP_DIR/$ASSET_NAME" "$ASSET_URL"; then
        echo -e "${RED}[ERROR]${NC} Failed to download binary. Are you connected to the internet, and does this release exist?"
        rm -rf "$TEMP_DIR"
        exit 1
    fi

    if [[ "$NO_VERIFY" == "false" ]]; then
        if [[ -n "$SHASUM_CMD" ]]; then
            echo -e "${BLUE}[INFO]${NC} Downloading checksums..."
            if $DOWNLOAD_CMD "$TEMP_DIR/checksums.txt" "$CHECKSUM_URL"; then
                echo -e "${BLUE}[INFO]${NC} Verifying checksum..."
                cd "$TEMP_DIR" || exit 1
                
                EXPECTED_HASH=$(grep "$ASSET_NAME" checksums.txt | awk '{print $1}' || true)
                if [[ -z "$EXPECTED_HASH" ]]; then
                    echo -e "${RED}[ERROR]${NC} Checksum for $ASSET_NAME not found in checksums.txt!"
                    cd - > /dev/null || exit 1
                    rm -rf "$TEMP_DIR"
                    exit 1
                fi
                
                echo "$EXPECTED_HASH  $ASSET_NAME" > expected.sha256
                
                if ! $SHASUM_CMD expected.sha256 > /dev/null 2>&1; then
                    echo -e "${RED}[ERROR]${NC} Checksum validation failed! The download may be corrupted."
                    cd - > /dev/null || exit 1
                    rm -rf "$TEMP_DIR"
                    exit 1
                fi
                echo -e "${GREEN}[OK]${NC} Checksum verified."
                cd - > /dev/null || exit 1
            else
                echo -e "${YELLOW}[WARN]${NC} checksums.txt not found on release. Skipping validation."
            fi
        else
            echo -e "${YELLOW}[WARN]${NC} Checksum validation tool not found. Skipping validation."
        fi
    else
        echo -e "${YELLOW}[WARN]${NC} Checksum validation disabled via --no-verify."
    fi

    echo -e "${BLUE}[INFO]${NC} Extracting..."
    tar -xzf "$TEMP_DIR/$ASSET_NAME" -C "$TEMP_DIR"
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
if [[ "$UPDATE_MODE" == "true" ]]; then
    echo -e "${GREEN}[OK]${NC} mpv-music successfully updated in $INSTALLED_BINARY"
else
    echo -e "${GREEN}[OK]${NC} mpv-music installed to $INSTALLED_BINARY"
fi

# --- 5. Initial Configuration ---
if [[ "$UPDATE_MODE" == "false" ]]; then
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
fi
# --- 6. PATH Verification ---
case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        if [[ "$UPDATE_MODE" == "true" ]]; then
            echo -e "\n${GREEN}Update complete!${NC} You are now running the latest version."
        else
            echo -e "\n${GREEN}Installation complete!${NC} Run 'mpv-music' to start."
        fi
        ;;
    *)
        echo -e "\n${YELLOW}[WARNING]${NC} $INSTALL_DIR is not in your PATH."
        echo -e "Please add it to your shell configuration (e.g., .bashrc or .zshrc):"
        echo -e "    export PATH=\"\$PATH:$INSTALL_DIR\""
        echo -e "\nAfter adding it, run 'mpv-music' to start."
        ;;
esac
