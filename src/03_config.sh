# --- Create Config Function ---
# Creates the configuration file for mpv-music.
create_config() {
    cat <<EOF > "$CONFIG_FILE"
# mpv-music configuration file

# Default music directories (space-separated)
# You can add multiple paths, e.g., MUSIC_DIRS="\$HOME/Music /mnt/my_music_drive/audio"
MUSIC_DIRS="${MUSIC_DIRS_DEFAULT[*]}"

# Default MPV arguments (space-separated)
# These will be used if no other MPV args are passed on the command line.
# Example: MPV_DEFAULT_ARGS="--loop-playlist=inf --shuffle --no-video --volume=50"
MPV_DEFAULT_ARGS="${MPV_ARGS_DEFAULT[*]}"

# Audio extensions (space-separated)
# These are used when --video-ok is NOT specified.
AUDIO_EXTS="$AUDIO_EXTS_DEFAULT"

# Video extensions (space-separated)
# These are added to AUDIO_EXTS when --video-ok IS specified.
VIDEO_EXTS="$VIDEO_EXTS_DEFAULT"

# Playlist extensions (space-separated)
PLAYLIST_EXTS="$PLAYLIST_EXTS_DEFAULT"

# Max log file size in Kilobytes (KB) before rotating.
# Default is 1024 (1MB).
LOG_MAX_SIZE_KB=1024

EOF
    log_verbose "Created default config file at $CONFIG_FILE"
}

# Check if config file exists, if not, create a default one
if [[ ! -f "$CONFIG_FILE" ]]; then
    create_config
fi

# Source the configuration file
# This will set the variables like MUSIC_DIRS, MPV_DEFAULT_ARGS, etc.
# shellcheck source=/dev/null
. "$CONFIG_FILE"

# Convert space-separated strings from config into arrays
IFS=' ' read -ra MUSIC_DIRS_ARRAY <<< "$MUSIC_DIRS"
eval "MPV_DEFAULT_ARGS_ARRAY=(${MPV_DEFAULT_ARGS[@]})"
IFS=' ' read -ra AUDIO_EXTS_ARRAY <<< "$AUDIO_EXTS"
IFS=' ' read -ra VIDEO_EXTS_ARRAY <<< "$VIDEO_EXTS"
IFS=' ' read -ra PLAYLIST_EXTS_ARRAY <<< "$PLAYLIST_EXTS"
LOG_MAX_SIZE_KB="${LOG_MAX_SIZE_KB:-1024}"

if ! [[ "$LOG_MAX_SIZE_KB" =~ ^[0-9]+$ ]]; then
    echo "Warning: Invalid LOG_MAX_SIZE_KB in config. Using default 1024KB." >&2
    LOG_MAX_SIZE_KB=1024
elif [[ "$LOG_MAX_SIZE_KB" -eq 0 ]]; then
    FILE_LOGGING_DISABLED=true
    # This message will only appear if -V is on, which is fine.
    log_verbose "LOG_MAX_SIZE_KB is 0. All logging to file is disabled."
fi

# --- Dependency Checks ---
if ! command -v mpv &>/dev/null || ! command -v fzf &>/dev/null; then
  echo "Missing dependencies. mpv-music requires:"
  echo "- mpv: media player (https://mpv.io)"
  echo "- fzf: fuzzy finder (https://github.com/junegunn/fzf)"
  echo "Install them and try again."
  exit 1
fi

if ! command -v yt-dlp &>/dev/null; then
  echo "Warning: yt-dlp not found. URL playback might be limited."
  echo "Install yt-dlp (https://github.com/yt-dlp/yt-dlp/) for full URL support."
  # no exit here, cuz local file/folder playback
fi

if ! command -v ffprobe &>/dev/null; then
  echo "Error: ffprobe not found. Metadata features will be unavailable."
  echo "ffprobe is part of the FFmpeg suite. Install FFmpeg (https://ffmpeg.org/download.html) and try again."
  exit 1 # cuz metadata is a core feature
fi

if ! command -v jq &>/dev/null; then
  echo "Error: jq not found. Metadata indexing and advanced features will be unavailable."
  echo "Install jq (e.g., sudo apt install jq or brew install jq) for full functionality."
  exit 1 # index file is json, so need jq for parsing metadata
fi

if ! command -v mediainfo &>/dev/null; then
  echo "Warning: mediainfo not found. Metadata extraction for some files (e.g., certain Opus) might be limited."
  echo "Install mediainfo (e.g., sudo apt install mediainfo or brew install mediainfo)."
fi

if ! command -v find &>/dev/null || ! find --version 2>&1 | grep -q 'GNU findutils'; then
  echo "Error: GNU find is required. Your system might be using BSD find."
  echo "Please install GNU findutils!"
  exit 1
fi
