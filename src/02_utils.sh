# --- Log Management ---
rotate_log() {
  # Safe expansion: Default to 0 if unset
  if [[ "$FILE_LOGGING_DISABLED" == true ]] || [[ "${LOG_MAX_SIZE_KB:-0}" == "0" ]]; then return; fi

  # Check size only if file exists
  if [[ -f "$LOG_FILE" ]]; then
    local max_size_kb="${LOG_MAX_SIZE_KB:-0}"
    local current_size_kb
    current_size_kb=$(du -k "$LOG_FILE" | cut -f1)

    if [[ "$current_size_kb" -gt "$max_size_kb" ]]; then
      # wipe it clean
      : > "$LOG_FILE"

      local timestamp
      timestamp=$(date +'%Y-%m-%d %H:%M:%S')
      echo -e "[$timestamp] [DEBUG] Log file limit ($max_size_kb KB) reached. File wiped." >> "$LOG_FILE"

      if [[ "$DEBUG" == true ]]; then
          echo -e "${YELLOW}[DEBUG] Log file limit reached. File wiped.${NC}" >&2
      fi
    fi
  fi
}

# --- Colors & Printing Helpers ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# --- Verbose and Debug Mode ---
VERBOSE=false
DEBUG=false
FILE_LOGGING_DISABLED=false

# Standardized message helpers
msg_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
    if [[ "$FILE_LOGGING_DISABLED" == false ]]; then
        rotate_log
        echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] [ERROR] $1" >> "$LOG_FILE"
    fi
}

msg_warn() {
    echo -e "${YELLOW}[WARN]${NC}  $1" >&2
    if [[ "$FILE_LOGGING_DISABLED" == false ]]; then
        rotate_log
        echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] [WARN]  $1" >> "$LOG_FILE"
    fi
}

msg_success() {
    echo -e "${GREEN}[OK]${NC}    $1" >&2
    if [[ "$FILE_LOGGING_DISABLED" == false ]]; then
        rotate_log
        echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] [OK]    $1" >> "$LOG_FILE"
    fi
}

msg_info() {
    echo -e "${BLUE}[INFO]${NC}  $1" >&2
    if [[ "$FILE_LOGGING_DISABLED" == false ]]; then
        rotate_log
        echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] [INFO]  $1" >> "$LOG_FILE"
    fi
}

msg_note() {
    echo -e "${CYAN}[NOTE]${NC}  $1" >&2
    if [[ "$FILE_LOGGING_DISABLED" == false ]]; then
        rotate_log
        echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] [NOTE]  $1" >> "$LOG_FILE"
    fi
}

# Helper function for verbose logging
log_verbose() {
    local message="[VERBOSE] $@"

    # Screen Output
    if [[ "$VERBOSE" == true ]]; then
        echo -e "$message" >&2
    fi

    # File Output
    if [[ "$FILE_LOGGING_DISABLED" == false ]]; then
        rotate_log
        echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] $message" >> "$LOG_FILE"
    fi
}

# Helper function for debug logging
log_debug() {
    local message="[DEBUG] $@"

    # Screen Output
    if [[ "$DEBUG" == true ]]; then
        echo -e "$message" >&2
    fi

    # File Output
    if [[ "$FILE_LOGGING_DISABLED" == false ]]; then
        rotate_log
        echo -e "[$(date +'%Y-%m-%d %H:%M:%S')] $message" >> "$LOG_FILE"
    fi
}

# --- Temporary File Management ---
# Array to store all temporary files
declare -a TEMP_FILES=()

# Function to create a temporary file and track it for cleanup
create_temp_file() {
  local -n out_var=$1
  out_var=$(mktemp)
  TEMP_FILES+=("$out_var")
}

# Function to clean up all temporary files
cleanup_temp_files() {
    trap '' HUP INT TERM QUIT  # Ignore signals during cleanup

    log_debug "--- Cleanup triggered ---"

    # Only log cleanup if we actually have files to clean, to avoid spam
    if [[ ${#TEMP_FILES[@]} -gt 0 ]]; then
        log_debug "Cleaning up ${#TEMP_FILES[@]} temporary files..."
        for tmp_file in "${TEMP_FILES[@]}"; do
        if [[ -f "$tmp_file" ]]; then
            rm -f "$tmp_file"
            # Verify removal
            [[ ! -f "$tmp_file" ]] && log_debug "Removed: $tmp_file" || log_debug "Failed to remove: $tmp_file"
        fi
        done
        log_debug "--- Cleanup complete ---"
    else
        log_debug "No temporary files found."
    fi
    trap - HUP INT TERM QUIT
}
# Set up comprehensive trap for all common termination signals
# Ensure cleanup happens on ANY exit
trap cleanup_temp_files EXIT

# Force a hard exit on signals (which triggers the EXIT trap above)
trap "echo -e '\nBye Bye!'; exit 1" HUP INT TERM QUIT

# --- Update Trigger Function ---
invoke_updater() {

    if [[ -f "$CONFIG_FILE" ]]; then
        # Temporarily disable exit-on-error in case config is slightly broken
        set +e
        # shellcheck source=/dev/null
        source "$CONFIG_FILE"
        set -e
    fi
    # Lazy Dependency Check
    if ! command -v curl &>/dev/null; then
        msg_error "Cannot update: 'curl' is missing."
        msg_note "Please install curl to use the update feature."
        exit 1
    fi

    local updater_url="https://raw.githubusercontent.com/FurqanHun/mpv-music/master/mpv-music-updater"
    local local_updater="$CONFIG_DIR/mpv-music-updater"
    local current_script_path

    # robustly get the current script path
    current_script_path=$(readlink -f "$0")

    log_verbose "Fetching updater..."

    if curl -sL "$updater_url" -o "$local_updater"; then
        chmod +x "$local_updater"

        # Pass control to the updater
        # We use 'exec' so this script process ends and the updater takes over PID
        local channel="${1:-stable}"
        local flag="--${channel##--}"
        log_verbose "Launching updater..."
        exec "$local_updater" "$current_script_path" "$flag"
    else
        msg_error "Failed to retrieve updater script."
        exit 1
    fi
}

# validate_index <index_file>
# Returns 0 if healthy, 1 if corrupt.
# FULL SCAN: Reads the whole file to catch errors in the middle.
validate_index() {
    local idx="$1"
    if [[ ! -s "$idx" ]]; then return 1; fi

    # "jq empty" reads the whole stream.
    # It takes <0.1s for normal libraries.
    # If ANY line is bad (middle or end), this returns 1.
    if ! jq -e . "$idx" >/dev/null 2>&1; then
        return 1
    fi

    return 0
}

# --- Helper Function: Check & Heal Index ---
ensure_index_integrity() {
    # Only run if index exists
    if [[ -f "$MUSIC_INDEX_FILE" ]]; then
        # Use validate_index (defined in utils) to check health
        if ! validate_index "$MUSIC_INDEX_FILE"; then
            msg_warn "Index corruption detected. Performing surgical repair..."

            temp_healed=$(mktemp)
            # select: Keep ONLY if it has ALL 8 required fields.
            jq -c -R 'fromjson? | select(.path and .title and .artist and .album and .genre and .mtime and .size and .media_type)' "$MUSIC_INDEX_FILE" > "$temp_healed"

            if [[ -s "$temp_healed" ]]; then
                mv "$temp_healed" "$MUSIC_INDEX_FILE"
                log_verbose "Bad lines removed, calling update_music_index"
                update_music_index
                msg_success "Index structure restored."
            else
                msg_warn "Index was too damaged to save. Rebuilding..."
                rm "$temp_healed"
                build_music_index
            fi
        fi
    fi
}

save_config_state() {
    local temp_conf
    create_temp_file temp_conf

    # ONLY delete the specific MUSIC_DIRS block range.
    # do NOT delete global orphans
    sed -e '/^MUSIC_DIRS=(/,/^)/d' \
        -e '/^MUSIC_DIRS=/d' \
        "$CONFIG_FILE" > "$temp_conf"

    {
            echo "MUSIC_DIRS=("
            for dir in "${MUSIC_DIRS_ARRAY[@]}"; do
                # we assuming the path dont contain double quotes (") inside them.
                echo "    \"$dir\""
            done
            echo ")"
        } >> "$temp_conf"

    mv "$temp_conf" "$CONFIG_FILE"

    log_verbose "Saved config state."

    # Reload to ensure memory matches disk
    reload_config_state
}

reload_config_state() {
    MUSIC_DIRS_ARRAY=()

    # prevents 'ghost' data if the file is empty/broken
    unset MUSIC_DIRS

    if [[ -f "$CONFIG_FILE" ]]; then
        source "$CONFIG_FILE"

        # Map the config variable to the script variable
        if [[ -n "${MUSIC_DIRS[*]}" ]]; then
             MUSIC_DIRS_ARRAY=("${MUSIC_DIRS[@]}")
        fi

        log_verbose "Reloaded: Found ${#MUSIC_DIRS_ARRAY[@]} dirs."
    else
        log_verbose "No config file found. Using defaults."
    fi
}

# Helper: Resolve which editor/viewer to use
resolve_editor() {
    local explicit="$1"
    local prefer_readonly="${2:-false}"

    if [[ -n "$explicit" ]]; then
        echo "$explicit"
        return
    fi

    if [[ "$prefer_readonly" == "true" ]]; then
        # Prioritize 'less' (Pager) -> 'vi' -> 'nano'
        # We ignore $EDITOR here because we don't want to edit logs by default.
        if command -v less &>/dev/null; then echo "less"
        elif command -v vi &>/dev/null; then echo "vi"
        else echo "nano"; fi
    else
        # Prioritize $EDITOR -> 'nano' -> 'vi'
        # We want the user's preferred editor here.
        if [[ -n "$EDITOR" ]]; then echo "$EDITOR"
        elif command -v nano &>/dev/null; then echo "nano"
        elif command -v vi &>/dev/null; then echo "vi"
        else echo "vi"; fi
    fi
}
