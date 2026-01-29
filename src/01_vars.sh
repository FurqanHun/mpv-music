# --- Configuration ---
CONFIG_DIR="$HOME/.config/mpv-music"
CONFIG_FILE="$CONFIG_DIR/mpv-music.conf"
MUSIC_INDEX_FILE="$CONFIG_DIR/music_index.jsonl"
LOG_FILE="$CONFIG_DIR/mpv-music.log"

# Default values (will be overridden by config file if it exists)
MUSIC_DIRS_DEFAULT=(
    "$HOME/Music"
)

# visual setup
BANNER="\n╔══  MPV-MUSIC  ══╗\n"

# MPV_ARGS_DEFAULT=(--loop-playlist=inf --shuffle --no-video --audio-display=no --msg-level=cplayer=warn --display-tags= "--term-playing-msg='\${BANNER}'" "--term-status-msg='▶ \${?metadata/artist:\${metadata/artist} - }\${?metadata/title:\${metadata/title}}\${!metadata/title:\${filename}} • \${time-pos} / \${duration} • (\${percent-pos}%)'")
MPV_STATUS_MSG_DEFAULT='▶ ${?metadata/artist:${metadata/artist} - }${?metadata/title:${metadata/title}}${!metadata/title:${media-title}} • ${time-pos} / ${duration} • (${percent-pos}%)'
MPV_ARGS_SIMPLE=(
    --loop-playlist=inf
    --shuffle
    --no-video
    --audio-display=no
    --msg-level=cplayer=warn
    --display-tags=
    --no-term-osd-bar
)
AUDIO_EXTS_DEFAULT="mp3 flac wav m4a aac ogg opus wma alac aiff amr"
VIDEO_EXTS_DEFAULT="mp4 mkv webm avi mov flv wmv mpeg mpg 3gp ts vob m4v"
PLAYLIST_EXTS_DEFAULT="m3u m3u8 pls"

# Ensure config directory exists
mkdir -p "$CONFIG_DIR"
