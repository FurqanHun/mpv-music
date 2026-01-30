# Cross-platform stat helpers
get_mtime() { stat -c %Y "$1" 2>/dev/null || stat -f %m "$1" 2>/dev/null; }
get_size() { stat -c %s "$1" 2>/dev/null || stat -f %z "$1" 2>/dev/null; }

# --- Helper: Pure Bash JSON Escaping ---
# Replaces jq for simple object creation.
json_escape() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\n'/\\n}"
    s="${s//$'\r'/}"
    s="${s//$'\t'/\\t}"
    printf '%s' "$s"
}

# --- Build Extension Filter Function ---
# Builds the 'find' extension filter based on script settings
build_ext_filter() {
  local current_exts=()
  if [[ -n "$CUSTOM_EXTS" ]]; then
    IFS=',' read -ra current_exts <<< "$CUSTOM_EXTS"
  elif [[ "$VIDEO_OK" == true ]]; then
    current_exts=("${AUDIO_EXTS_ARRAY[@]}" "${VIDEO_EXTS_ARRAY[@]}" "${PLAYLIST_EXTS_ARRAY[@]}")
  else
    current_exts=("${AUDIO_EXTS_ARRAY[@]}" "${PLAYLIST_EXTS_ARRAY[@]}")
  fi

  # Clear the global EXT_FILTER before rebuilding
  EXT_FILTER=()
  for i in "${!current_exts[@]}"; do
    EXT_FILTER+=( -iname "*.${current_exts[$i]}" )
    if [[ $((i+1)) -lt ${#current_exts[@]} ]]; then
      EXT_FILTER+=( -o )
    fi
  done
}

# --- Metadata Extraction Function ---
# get_audio_metadata <file_path>
# Outputs: title,artist,album,genre (comma-separated, empty if not found)
# Example: "Song Title,Artist Name,Album Name,Genre Type"
get_audio_metadata() {
  local file="$1"

  local file_ext="${file##*.}"
    file_ext="${file_ext,,}"

    # if the file is a playlist first
    if [[ " ${PLAYLIST_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
      local title
      title="$(basename "$file" | sed 's/\.[^.]*$//')"
      # Return special metadata for playlists
      echo -e "${title}\tPlaylist\tPlaylists\tPlaylist"
      return 0
    fi

  local title=""
  local artist=""
  local album=""
  local genre=""
  local metadata_json

  # --- Attempt 1: Use ffprobe ---
  log_debug "using ffprobe!"
  metadata_json=$(timeout 2s ffprobe -v quiet -hide_banner \
      -analyzeduration 10000000 -probesize 10000000 \
      -show_format -show_streams -of json "$file" 2>/dev/null)

  if [[ -n "$metadata_json" ]]; then
    # The output is tab-separated (@tsv), which is safer for parsing.
    local metadata_line
    metadata_line=$(echo "$metadata_json" | jq -r '
      [
        .format.tags.title // .streams[0]?.tags?.title // .format.tags.TIT2 // .streams[0]?.tags?.TIT2 // .format.tags.NAME // .streams[0]?.tags?.NAME // .format.tags.TITLE // .streams[0]?.tags?.TITLE // .format.tags."Track name" // .streams[0]?.tags?."Track name" // "",
        .format.tags.artist // .streams[0]?.tags?.artist // .format.tags.TPE1 // .streams[0]?.tags?.TPE1 // .format.tags.TPE2 // .streams[0]?.tags?.TPE2 // .format.tags.album_artist // .streams[0]?.tags?.album_artist // .format.tags.ARTIST // .streams[0]?.tags?.ARTIST // .format.tags.Performer // .streams[0]?.tags?.Performer // "",
        .format.tags.album // .streams[0]?.tags?.album // .format.tags.TALB // .streams[0]?.tags?.TALB // .format.tags.ALBUM // .streams[0]?.tags?.ALBUM // "",
        .format.tags.genre // .streams[0]?.tags?.genre // .format.tags.TCON // .streams[0]?.tags?.TCON // .format.tags.GENRE // .streams[0]?.tags?.GENRE // ""
      ] | @tsv
    ')

    # Read the tab-separated output from jq directly into variables.
    IFS=$'\t' read -r title artist album genre <<< "$metadata_line"
  else
      log_debug "ffprobe returned empty JSON for: $file"
  fi

  # --- Attempt 2: Fallback to mediainfo if ffprobe didn't find title/artist ---
  # Only try mediainfo if both title AND artist are still empty from ffprobe
  if [[ -z "$title" && -z "$artist" ]]; then
    if command -v mediainfo &>/dev/null; then
      # Use mediainfo to get specific fields
      local mediainfo_raw
      log_debug "Using mediainfo fallback for '$file' (ffprobe failed to find title/artist)."
      mediainfo_raw=$(mediainfo --Inform="General;%Track_Name%|%Performer%" "$file" 2>/dev/null)

      log_debug "mediainfo raw output for '$file':"
      log_debug "$mediainfo_raw"

      if [[ -n "$mediainfo_raw" ]]; then
        # Split by pipe '|' which is the default separator for --Inform
        IFS='|' read -r title artist <<< "$mediainfo_raw"
        log_debug "MediaInfo found - Title: '$title', Artist: '$artist'"
      fi
    fi
  fi

  # Final cleanup for newlines/carriage returns and potential quotes
  title=$(echo "$title" | tr -d '\n\r' | sed 's/^"//;s/"$//')
  artist=$(echo "$artist" | tr -d '\n\r' | sed 's/^"//;s/"$//')
  album=$(echo "$album" | tr -d '\n\r' | sed 's/^"//;s/"$//')
  genre=$(echo "$genre" | tr -d '\n\r' | sed 's/^"//;s/"$//')

  # --- Apply Default Values ---
  # Fallback to filename for title if still empty
  if [[ -z "$title" ]]; then
    title="$(basename "$file" | sed 's/\.[^.]*$//')" # Get filename without extension
    log_debug "No metadata title found. Using filename: $title"
  fi

  # Default other empty fields to UNKNOWN
  artist="${artist:-UNKNOWN}"
  album="${album:-UNKNOWN}"
  genre="${genre:-UNKNOWN}"

  log_debug "Parsed: Title='${title:0:50}', Artist='${artist:0:50}', Album='${album:0:50}'"

 # Output as tab-separated string to avoid issues with commas in titles
  echo -e "${title}\t${artist}\t${album}\t${genre}"
}
