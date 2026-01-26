# --- Log Management ---
rotate_log() {
    if [[ "$FILE_LOGGING_DISABLED" == true ]]; then return; fi
  # Only rotate the log if BOTH flags are enabled
  if [[ "$VERBOSE" == true && "$DEBUG" == true ]]; then
    local max_size_kb="$LOG_MAX_SIZE_KB"
    if [[ -f "$LOG_FILE" ]]; then
      local current_size_kb
      current_size_kb=$(du -k "$LOG_FILE" | cut -f1)
      if [[ "$current_size_kb" -gt "$max_size_kb" ]]; then
        mv "$LOG_FILE" "${LOG_FILE}.old"
        # Use log_debug so this message also gets logged
        log_debug "Log file rotated. Old log is at ${LOG_FILE}.old"
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

# Standardized message helpers
msg_error()   { echo -e "${RED}[ERROR]${NC} $1" >&2; }
msg_warn()    { echo -e "${YELLOW}[WARN]${NC}  $1" >&2; }
msg_success() { echo -e "${GREEN}[OK]${NC}    $1" >&2; }
msg_info()    { echo -e "${BLUE}[INFO]${NC}  $1" >&2; }
msg_note()    { echo -e "${CYAN}[NOTE]${NC}  $1" >&2; }

# --- Verbose and Debug Mode ---
VERBOSE=false
DEBUG=false
FILE_LOGGING_DISABLED=false

# Helper function for verbose logging
log_verbose() {
    # This outer 'if' ensures the message is printed to the screen if --verbose is on
    if [[ "$VERBOSE" == true ]]; then
        local message="[VERBOSE] $@"
        # This inner 'if' checks if we should ALSO write to the log file
        if [[ "$DEBUG" == true && "$FILE_LOGGING_DISABLED" == false ]]; then
            echo -e "$message" | tee -a "$LOG_FILE" >&2
        else
            echo -e "$message" >&2
        fi
    fi
}

# Helper function for debug logging
log_debug() {
    # This outer 'if' ensures the message is printed to the screen if --debug is on
    if [[ "$DEBUG" == true ]]; then
        local message="[DEBUG] $@"
        # This inner 'if' checks if we should ALSO write to the log file
        if [[ "$VERBOSE" == true && "$FILE_LOGGING_DISABLED" == false ]]; then
            echo -e "$message" | tee -a "$LOG_FILE" >&2
        else
            echo -e "$message" >&2
        fi
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
    log_debug "\n--- Cleanup triggered ---"
  if [[ ${#TEMP_FILES[@]} -gt 0 ]]; then # Ignore signals during cleanup
    log_debug "Cleaning up ${#TEMP_FILES[@]} temporary files..."
    for tmp_file in "${TEMP_FILES[@]}"; do
      if [[ -f "$tmp_file" ]]; then
        log_debug "Removing: $tmp_file" >&2
        rm -f "$tmp_file"
        # Verify removal
        [[ ! -f "$tmp_file" ]] && log_debug "✓ Successfully removed" || log_debug "❌ Failed to remove"
      else
        log_debug "File already gone: $tmp_file"
      fi
    done
    log_debug "--- Cleanup complete ---"
  else
      log_debug "No temporary files to clean up"
  fi
  trap - HUP INT TERM QUIT  # Restore signal handling
}
# Set up comprehensive trap for all common termination signals
trap cleanup_temp_files EXIT HUP INT TERM QUIT

# --- Update Trigger Function ---
invoke_updater() {
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
        log_verbose "Launching updater..."
        exec "$local_updater" "$current_script_path"
    else
        msg_error "Failed to retrieve updater script."
        exit 1
    fi
}
