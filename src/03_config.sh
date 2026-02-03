# --- Create Config Function ---
# Creates the configuration file for mpv-music.
create_config() {
    local safe_simple_args
        safe_simple_args=$(printf "%q " "${MPV_ARGS_SIMPLE[@]}")
    cat <<EOF > "$CONFIG_FILE"
# mpv-music configuration file

# --- Playback Control ---
# Shuffle playlist by default? (true/false)
# (CLI flags: --shuffle / --no-shuffle)
SHUFFLE=$SHUFFLE_MODE

# Looping Behavior
# Options:
#   "playlist" - Loop the entire queue endlessly (default)
#   "track"    - Loop the current track endlessly (Repeat One)
#   "no"       - Play once and stop
#   "inf"      - Explicit infinite loop (same as playlist)
#   "5"        - Loop 5 times (any number works)
# (CLI flags: --loop, --repeat, --no-loop)
LOOP_MODE="$LOOP_MODE"

# Set to true to include video files in library scans by default.
# (Command line flag: --video-ok)
VIDEO_OK=$VIDEO_OK

# Set to true to force single-threaded indexing.
# Useful for mechanical HDDs to prevent thrashing.
# (Command line flag: --serial)
SERIAL_MODE=$SERIAL_MODE

# Default playback volume (0-100)
# You can override this per-run with --volume=50
VOLUME=$VOLUME

# Latest versions of yt-dlp need the 'ejs' component from GitHub to handle YouTube.
# If your yt-dlp is "bundled" (has everything inside), set this to "false".
# If you get playback errors, set this to "true".
YTDLP_EJS_REMOTE_GITHUB=$YTDLP_EJS_REMOTE_GITHUB

# Audio extensions (space-separated)
# These are used when --video-ok is NOT specified.
AUDIO_EXTS="$AUDIO_EXTS_DEFAULT"

# Video extensions (space-separated)
# These are added to AUDIO_EXTS when --video-ok IS specified.
VIDEO_EXTS="$VIDEO_EXTS_DEFAULT"

# Playlist extensions (space-separated)
PLAYLIST_EXTS="$PLAYLIST_EXTS_DEFAULT"

# Max log file size in Kilobytes (KB) before rotating.
# Default is 5120 (5MB).
LOG_MAX_SIZE_KB=5120

BANNER_TEXT='$BANNER'
STATUS_MSG='$MPV_STATUS_MSG_DEFAULT'

# Default MPV arguments (Bash Array - Double Quoted)
# These will be used if no other MPV args are passed on the command line.
# Example:
# MPV_DEFAULT_ARGS=(
#     --no-video
#     --audio-display=no
#     --msg-level=cplayer=warn
#     --display-tags=
#     --no-term-osd-bar
#     "--term-playing-msg=\$(tput clear)\$BANNER_TEXT"
#     "--term-status-msg=\$STATUS_MSG"
# )
MPV_DEFAULT_ARGS=(
    $safe_simple_args
    "--term-playing-msg=\$(tput clear)\$BANNER_TEXT"
    "--term-status-msg=\$STATUS_MSG"
)

# Default music directories (Bash Array - Double Quoted)
# You can add multiple paths,
# Example:
# MUSIC_DIRS=(
#   "\$HOME/Music"
#   "/mnt/my_music_drive/audio"
# )
MUSIC_DIRS=(
    "${MUSIC_DIRS_DEFAULT[*]}"
)

EOF
    log_verbose "Created default config file at $CONFIG_FILE"
}

# Check if config file exists, if not, create a default one
if [[ ! -f "$CONFIG_FILE" ]]; then
    log_verbose "Config file not found, creating default config..."
    create_config
fi

# Source the configuration file
# This will set the variables like MUSIC_DIRS, MPV_DEFAULT_ARGS, etc.
# We disable 'set -e' temporarily so a syntax error in the config doesn't crash
# the script before we can warn the user.
set +e
# shellcheck source=/dev/null
. "$CONFIG_FILE"
CONFIG_STATUS=$?
set -e

LOG_MAX_SIZE_KB="${LOG_MAX_SIZE_KB:-5120}"

if ! [[ "$LOG_MAX_SIZE_KB" =~ ^[0-9]+$ ]]; then
    msg_warn "Invalid LOG_MAX_SIZE_KB in config. Using default 5120KB (5MB)." >&2
    LOG_MAX_SIZE_KB=5120
elif [[ "$LOG_MAX_SIZE_KB" -eq 0 ]]; then
    FILE_LOGGING_DISABLED=true
    # This message will only appear if -V is on, which is fine.
    log_verbose "LOG_MAX_SIZE_KB is 0. All logging to file is disabled."
fi

ARGS_TO_KEEP+=()

# --- Pre-Flight Flag Check ---
# We check these specific flags BEFORE sourcing the config file.
# This ensures that if the config file is broken (syntax error),
# you can still run --config to fix it or --update to patch the script.
while [[ $# -gt 0 ]]; do
    case "$1" in
        -V|--verbose) VERBOSE=true; shift;;
        --debug)
            DEBUG=true
            VERBOSE=true
            # Enable Bash Tracing
            set -x
            shift
            ;;
        --config|--config=*|--log|--log=*)
            key="${1%%=*}"
            val="${1#*=}"
            target_file=""
            read_only_mode="false"

            # Handle value extraction (Equals vs Space)
            if [[ "$val" == "$key" ]]; then
                val=""
                # Check next arg
                if [[ -n "${2:-}" && "$2" != -* ]]; then
                    val="$2"
                    shift # Eat the extra argument
                fi
            fi

            # Setup targets based on flag
            if [[ "$key" == "--config" ]]; then
                target_file="$CONFIG_FILE"
                read_only_mode="false"
            elif [[ "$key" == "--log" ]]; then
                target_file="$LOG_FILE"
                read_only_mode="true"
            fi

            # one function call handles defaults for both
            opener=$(resolve_editor "$val" "$read_only_mode")

            if [[ -f "$target_file" ]]; then
                log_verbose "Opening $target_file with $opener..."
                "$opener" "$target_file"
            else
                msg_warn "File not found: $target_file"
            fi
            exit 0
            ;;
        --remove-config|--rm-conf)
            if [[ -f "$CONFIG_FILE" ]]; then
                msg_warn "Deleting existing config: $CONFIG_FILE"
                rm "$CONFIG_FILE"
            fi
            msg_info "Config deleted."
            msg_note "Config will be regenerated on next run."
            exit 0
            ;;
        --remove-log|--rm-log)
            FILE_LOGGING_DISABLED=true
            if [[ -f "$LOG_FILE" ]]; then
                msg_warn "Deleting existing log: $LOG_FILE"
                rm "$LOG_FILE"
                msg_info "Log deleted."
            else
                msg_warn "File not found: $LOG_FILE"
            fi
            msg_info "Log deleted."
            exit 0
            ;;
        --update)
            invoke_updater
            exit 0
            ;;
        -v|--version)
            echo "mpv-music v$VERSION"
            exit 0
            ;;
        *)
            ARGS_TO_KEEP+=("$1")
            shift
            ;;
    esac
done

set -- "${ARGS_TO_KEEP[@]}"

if [[ $CONFIG_STATUS -ne 0 ]]; then
    msg_error "Your configuration file ($CONFIG_FILE) has syntax errors." >&2
    msg_note "Please run 'mpv-music --config' to fix it." >&2
    exit 1
fi

if [[ "$(declare -p MUSIC_DIRS 2>/dev/null)" =~ "declare -a" ]]; then
    MUSIC_DIRS_ARRAY=("${MUSIC_DIRS[@]}")
else
    msg_error "Invalid MUSIC_DIRS in config. Please run 'mpv-music --config' to fix it." >&2
    exit 1
fi

IFS=' ' read -ra AUDIO_EXTS_ARRAY <<< "$AUDIO_EXTS"
IFS=' ' read -ra VIDEO_EXTS_ARRAY <<< "$VIDEO_EXTS"
IFS=' ' read -ra PLAYLIST_EXTS_ARRAY <<< "$PLAYLIST_EXTS"

if [[ "$DEBUG" == "true" ]]; then
    safe_simple_args=$(printf "%q " "${MPV_ARGS_SIMPLE[@]}")
    MPV_DEFAULT_ARGS=(
        $safe_simple_args
        "--term-playing-msg=$BANNER_TEXT"
        "--term-status-msg=$STATUS_MSG"
        "--msg-level=ytdl_hook=trace"
    )
    msg_warn "DEBUG MODE ENABLED"
    log_debug "--- SYSTEM INFO ---"
    log_debug "OS: $(uname -sr)"
    log_debug "MPV: $(mpv --version | head -n 1)"
    log_debug "YT-DLP: $(yt-dlp --version 2>/dev/null || echo 'NOT FOUND')"
    log_debug "FFMPEG: $(ffmpeg -version | head -n 1)"
    log_debug "Script Version: $VERSION"
    log_debug "-------------------"
fi

if [ -n "${MPV_DEFAULT_ARGS+x}" ]; then
    MPV_DEFAULT_ARGS_ARRAY=("${MPV_DEFAULT_ARGS[@]}")
else
    MPV_DEFAULT_ARGS_ARRAY=()
fi

# --- Config Management Functions ---

config_add_dir() {
    local target="$1"
    local skip_refresh="${2:-false}"

    if ! command -v realpath &>/dev/null; then
         msg_error "Missing 'realpath'. Cannot resolve path."
         exit 1
    fi
    local abs_path
    abs_path=$(realpath "$target")

    if [[ ! -d "$abs_path" ]]; then
        msg_error "Directory does not exist: $abs_path"
        return 1
    fi

    reload_config_state

    # Check for duplicates
    for d in "${MUSIC_DIRS_ARRAY[@]}"; do
        if [[ "$d" == "$abs_path" ]]; then
            msg_warn "Directory already exists: $abs_path"
            return 0
        fi
    done

    MUSIC_DIRS_ARRAY+=("$abs_path")

    save_config_state
    msg_success "Added: $abs_path"

    if [[ "$skip_refresh" != "true" ]]; then
        if command -v update_music_index &>/dev/null; then
            log_verbose "Scanning new directory..."
            update_music_index
        else
            msg_warn "Could not refresh index automatically."
        fi
    fi
    return 0
}

config_remove_dir() {
    local target="$1"
    local skip_refresh="${2:-false}"
    local abs_path
    abs_path=$(realpath "$target")

    reload_config_state

    local new_array=()
    local found=false

    # filter the array
    for d in "${MUSIC_DIRS_ARRAY[@]}"; do
        if [[ "$d" == "$abs_path" ]]; then
            found=true
            continue
        fi
        new_array+=("$d")
    done

    if [[ "$found" == false ]]; then
        msg_warn "Directory not found in config: $abs_path"
        return 1
    fi

    MUSIC_DIRS_ARRAY=("${new_array[@]}")
    save_config_state
    msg_success "Removed: $abs_path"

    if [[ "$skip_refresh" != "true" ]]; then
        if command -v update_music_index &>/dev/null; then
            log_verbose "Updating index..."
            update_music_index
        fi
    fi
    return 0
}

# --- Dependency Checks ---
if ! command -v mpv &>/dev/null || ! command -v fzf &>/dev/null; then
  msg_error "Missing dependencies. mpv-music requires:"
  msg_error "- mpv: media player (https://mpv.io)"
  msg_error "- fzf: fuzzy finder (https://github.com/junegunn/fzf)"
  msg_note "Install them and try again."
  exit 1
fi

if ! command -v yt-dlp &>/dev/null; then
  msg_warn "yt-dlp not found. URL playback might be limited."
  msg_note "Install yt-dlp (https://github.com/yt-dlp/yt-dlp/) for full URL support."
  # no exit here, cuz local file/folder playback
fi

if ! command -v ffprobe &>/dev/null; then
  msg_error "ffprobe not found. Metadata features will be unavailable."
  msg_note "ffprobe is part of the FFmpeg suite. Install FFmpeg (https://ffmpeg.org/download.html) and try again."
  exit 1 # cuz metadata is a core feature
fi

if ! command -v jq &>/dev/null; then
  msg_error "jq not found. Metadata indexing and advanced features will be unavailable."
  msg_note "Install jq (e.g., sudo apt install jq or brew install jq) for full functionality."
  exit 1 # index file is json, so need jq for parsing metadata
fi

if ! command -v mediainfo &>/dev/null; then
  msg_warn "mediainfo not found. Metadata extraction for some files (e.g., certain Opus) might be limited."
  msg_note "Install mediainfo (e.g., sudo apt install mediainfo or brew install mediainfo)."
fi

if ! command -v find &>/dev/null; then
    msg_error "'find' command is missing."
    exit 1
fi

if ! find . -maxdepth 0 -print0 &>/dev/null; then
    msg_error "Your 'find' command does not support -print0."
    msg_note "mpv-music requires GNU find or a compatible version for safety."
    exit 1
fi
