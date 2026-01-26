# Cross-platform stat helpers
get_mtime() { stat -c %Y "$1" 2>/dev/null || stat -f %m "$1" 2>/dev/null; }
get_size() { stat -c %s "$1" 2>/dev/null || stat -f %z "$1" 2>/dev/null; }

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
  metadata_json=$(ffprobe -v quiet -hide_banner -show_format -show_streams -of json "$file" 2>/dev/null)

  log_debug "ffprobe JSON output for '$file':"
  log_debug "$metadata_json"

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
  fi

  # Default other empty fields to UNKNOWN
  artist="${artist:-UNKNOWN}"
  album="${album:-UNKNOWN}"
  genre="${genre:-UNKNOWN}"

 # Output as tab-separated string to avoid issues with commas in titles
  echo -e "${title}\t${artist}\t${album}\t${genre}"
}

# --- Temporary Index Build Function ---
# Builds a temporary index for a given directory.
build_temp_index() {
    local custom_dir="$1"
    local -n temp_index_ref=$2
    local ext_filter=("${EXT_FILTER[@]}")

    log_verbose "Temporarily indexing files from '$custom_dir' for selection..."

    create_temp_file temp_index_ref

    # Collect files into an array (same pattern as build_music_index)
    local all_files=()
    while IFS= read -r -d '' file; do
        all_files+=("$file")
    done < <(find "$custom_dir" -type f \( "${ext_filter[@]}" \) -print0)

    local file_count=${#all_files[@]}

    if [[ $file_count -eq 0 ]]; then
        msg_warn "No music files found in '$custom_dir'."
        # Create an empty index so downstream commands don't fail
        : > "$temp_index_ref"
        return 1
    fi

    log_verbose "Found $file_count files. Processing metadata..."
    local count=0

    # --- OPTIMIZATION ---
    # Create another temporary file to hold the line-delimited JSON objects.
    create_temp_file temp_json_lines

    for file_path in "${all_files[@]}"; do
        count=$((count + 1))

        # SMART PROGRESS BAR
        if [[ "$VERBOSE" == true ]]; then
             # Truncate filename to prevent messy wrapping
             local fname=$(basename "$file_path")
             if [[ ${#fname} -gt 30 ]]; then fname="${fname:0:27}..."; fi
             printf "\rIndexing: %d/%d (%s)          " "$count" "$file_count" "$fname" >&2
        else
             # Default: Just numbers
             printf "\rIndexing: %d/%d          " "$count" "$file_count" >&2
        fi

        local raw_metadata_output
        raw_metadata_output=$(get_audio_metadata "$file_path")
        IFS=$'\t' read -r -a metadata_array <<< "$raw_metadata_output"

        local title="${metadata_array[0]}"
        local artist="${metadata_array[1]}"
        local album="${metadata_array[2]}"
        local genre="${metadata_array[3]}"

        local file_ext="${file_path##*.}"
        file_ext="${file_ext,,}"
        local media_type="UNKNOWN"
        if [[ " ${AUDIO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
            media_type="audio"
        elif [[ " ${VIDEO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
            media_type="video"
        elif [[ " ${PLAYLIST_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
            media_type="playlist"
        fi

        jq -cn \
          --arg path "$file_path" \
          --arg title "$title" \
          --arg artist "$artist" \
          --arg album "$album" \
          --arg genre "$genre" \
          --arg media_type "$media_type" \
          '{path: $path, title: $title, artist: $artist, album: $album, genre: $genre, media_type: $media_type}' >> "$temp_json_lines"

    done

    echo "" >&2

    mv "$temp_json_lines" "$temp_index_ref"

    log_verbose "Temporary index created at '$temp_index_ref'."
}

# --- Music Library Indexing Function ---
# Builds or rebuilds the entire music index.
build_music_index() {
  local music_dirs=("${MUSIC_DIRS_ARRAY[@]}")
  local ext_filter=("${EXT_FILTER[@]}")
  local all_music_files=()
  local indexed_dirs_json_array="[]"

  log_verbose "Indexing music library for the first time... This may take a while for large collections."

  # Populate all_music_files and indexed_dirs_json_array
  for dir_path in "${music_dirs[@]}"; do
    if [[ -d "$dir_path" ]]; then
      # Fix for newline: Ensure dir_path is trimmed before passing to jq
      local trimmed_dir_path
      trimmed_dir_path=$(echo "$dir_path" | tr -d '\n\r')
      local dir_mtime
      dir_mtime=$(get_mtime "$dir_path" || echo "")
      local dir_json
      dir_json=$(jq -n --arg path "$trimmed_dir_path" --arg mtime "$dir_mtime" '{path: $path, mtime: $mtime}')
      indexed_dirs_json_array=$(echo "$indexed_dirs_json_array" | jq --argjson new_dir "$dir_json" '. + [$new_dir]')

      # Find files within this specific directory and its subdirectories
      while IFS= read -r -d '' file; do
        all_music_files+=("$file")
      done < <(find "$dir_path" -type f \( "${ext_filter[@]}" \) -print0)
    else
      msg_warn "Configured music directory '$dir_path' does not exist. Skipping."
    fi
  done

  # save directory state to its own file
  echo "$indexed_dirs_json_array" > "$DIRS_STATE_FILE"

  if [[ ${#all_music_files[@]} -eq 0 ]]; then
    msg_warn "No music files found in configured directories. Index will be empty."
    # make an empty file
    : > "$MUSIC_INDEX_FILE"
    return 0
  fi

  # --- OPTIMIZED INDEXING LOOP ---
  # Create a temporary file to store one JSON object per line
  create_temp_file temp_json_lines

  local count=0
  local total=${#all_music_files[@]}

  for file_path in "${all_music_files[@]}"; do
    count=$((count + 1))

    # SMART PROGRESS BAR
    if [[ "$VERBOSE" == true ]]; then
         local fname=$(basename "$file_path")
         if [[ ${#fname} -gt 30 ]]; then fname="${fname:0:27}..."; fi
         printf "\rIndexing: %d/%d (%s)          " "$count" "$total" "$fname" >&2
    else
         printf "\rIndexing: %d/%d          " "$count" "$total" >&2
    fi

    local raw_metadata_output
    raw_metadata_output="$(get_audio_metadata "$file_path")"

    log_debug "(build_index): Processing file: '$file_path'"
    log_debug "(build_index): Raw metadata function output: '$raw_metadata_output'"


    # Split metadata string (title,artist,album,genre)
    IFS=$'\t' read -r -a metadata_array <<< "$raw_metadata_output"

    local title="${metadata_array[0]}"
    local artist="${metadata_array[1]}"
    local album="${metadata_array[2]}"
    local genre="${metadata_array[3]}"
    local mtime
    mtime=$(get_mtime "$file_path" || echo "")
    local size
    size=$(get_size "$file_path" || echo "")
    local trimmed_file_path
    trimmed_file_path=$(echo "$file_path" | tr -d '\n\r')
    local file_ext="${file_path##*.}"
    file_ext="${file_ext,,}" # Convert to lowercase
    local media_type="UNKNOWN"

    # Check if extension is in audio or video lists
    if [[ " ${AUDIO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
      media_type="audio"
    elif [[ " ${VIDEO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
      media_type="video"
    elif [[ " ${PLAYLIST_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
      media_type="playlist"
    fi

    # Append a single, compact JSON line to the temp file for each track
    jq -cn \
      --arg path "$trimmed_file_path" \
      --arg title "$title" \
      --arg artist "$artist" \
      --arg album "$album" \
      --arg genre "$genre" \
      --arg mtime "$mtime" \
      --arg size "$size" \
      --arg media_type "$media_type" \
      '{path: $path, title: $title, artist: $artist, album: $album, genre: $genre, mtime: $mtime, size: $size, media_type: $media_type}' >> "$temp_json_lines"

  done

  printf "\rIndexing complete: %d/%d files processed.\n" "$total" "$total" >&2

  # --- EFFICIENT JSON ASSEMBLY ---
  # Just move the JSONL file to the final location.
  mv "$temp_json_lines" "$MUSIC_INDEX_FILE"

  msg_success "Index saved to $MUSIC_INDEX_FILE"

  # --- CLEANUP LEGACY JSON ---
    # If we successfully built the new JSONL index, remove the old JSON file.
    local legacy_index="${MUSIC_INDEX_FILE%.jsonl}.json"
    if [[ -f "$legacy_index" ]]; then
        rm "$legacy_index"
        log_verbose "Removed legacy index file: $(basename "$legacy_index")"
    fi
}

# --- Music Library Update Function ---
# Updates the music index by checking for new, removed, or modified files.
update_music_index() {
  local music_dirs=("${MUSIC_DIRS_ARRAY[@]}")
  local ext_filter=("${EXT_FILTER[@]}")
  local current_files_on_disk=()
  declare -A old_index_map
  # local old_index_json_string (Removed, no longer reading whole file into string)

  log_verbose "Updating music library index..."

  if [[ -f "$MUSIC_INDEX_FILE" ]]; then
    # Read line-by-line to build the map. Fast and low memory.
    while IFS= read -r line; do
        # Extract path safely using jq
        local p=$(echo "$line" | jq -r .path)
        old_index_map["$p"]="$line"
    done < "$MUSIC_INDEX_FILE"
  else
    msg_info "Index file not found. Building index for the first time."
    build_music_index
    return 0
  fi

  for dir_path in "${music_dirs[@]}"; do
    if [[ -d "$dir_path" ]]; then
      while IFS= read -r -d '' file; do
        current_files_on_disk+=("$file")
      done < <(find "$dir_path" -type f \( "${ext_filter[@]}" \) -print0)
    fi
  done

  if [[ ${#current_files_on_disk[@]} -eq 0 ]]; then
    msg_warn "No music files found on disk during update scan. Index will be empty."
    # Clear index, save empty state
    : > "$MUSIC_INDEX_FILE"
    echo "[]" > "$DIRS_STATE_FILE"
    return 0
  fi

  # --- OPTIMIZATION ---
  # Create a temporary file to store the JSON for each track, one object per line.
  create_temp_file new_index_lines

  local count=0
  local total=${#current_files_on_disk[@]}

  for file_path in "${current_files_on_disk[@]}"; do
    count=$((count + 1))

    # SMART PROGRESS BAR
    if [[ "$VERBOSE" == true ]]; then
         local fname=$(basename "$file_path")
         if [[ ${#fname} -gt 30 ]]; then fname="${fname:0:27}..."; fi
         printf "\rScanning and updating: %d/%d (%s)          " "$count" "$total" "$fname" >&2
    else
         printf "\rScanning and updating: %d/%d          " "$count" "$total" >&2
    fi

    local current_mtime=$(get_mtime "$file_path" || echo "")
    local current_size=$(get_size "$file_path" || echo "")
    local track_json_to_add
    local trimmed_file_path=$(echo "$file_path" | tr -d '\n\r')

    if [[ -n "${old_index_map[$trimmed_file_path]+x}" ]]; then
      local old_track_json="${old_index_map[$trimmed_file_path]}"
      local old_mtime=$(echo "$old_track_json" | jq -r '.mtime // ""')
      local old_size=$(echo "$old_track_json" | jq -r '.size // ""')

      if [[ "$current_mtime" == "$old_mtime" && "$current_size" == "$old_size" ]]; then
        track_json_to_add="$old_track_json"
      else
        log_debug "(Modified: $(basename "$file_path"))"
        local raw_metadata_output=$(get_audio_metadata "$file_path")
        IFS=$'\t' read -r -a metadata_array <<< "$raw_metadata_output"

        local title="${metadata_array[0]}"
        local artist="${metadata_array[1]}"
        local album="${metadata_array[2]}"
        local genre="${metadata_array[3]}"
        local file_ext="${file_path##*.}"
        file_ext="${file_ext,,}"
        local media_type="UNKNOWN"

        if [[ " ${AUDIO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
          media_type="audio"
        elif [[ " ${VIDEO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
          media_type="video"
        elif [[ " ${PLAYLIST_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
          media_type="playlist"
        fi

        track_json_to_add=$(jq -cn \
          --arg path "$trimmed_file_path" --arg title "$title" --arg artist "$artist" --arg album "$album" --arg genre "$genre" \
          --arg mtime "$current_mtime" --arg size "$current_size" --arg media_type "$media_type" \
          '{path: $path, title: $title, artist: $artist, album: $album, genre: $genre, mtime: $mtime, size: $size, media_type: $media_type}')
      fi
    else
      log_debug "(New: $(basename "$file_path"))"
      local raw_metadata_output=$(get_audio_metadata "$file_path")
      IFS=$'\t' read -r -a metadata_array <<< "$raw_metadata_output"

      local title="${metadata_array[0]}"
      local artist="${metadata_array[1]}"
      local album="${metadata_array[2]}"
      local genre="${metadata_array[3]}"
      local file_ext="${file_path##*.}"
      file_ext="${file_ext,,}"
      local media_type="UNKNOWN"

      if [[ " ${AUDIO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
        media_type="audio"
      elif [[ " ${VIDEO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
        media_type="video"
      elif [[ " ${PLAYLIST_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then
      media_type="playlist"
      fi

      track_json_to_add=$(jq -cn \
        --arg path "$trimmed_file_path" --arg title "$title" --arg artist "$artist" --arg album "$album" --arg genre "$genre" \
        --arg mtime "$current_mtime" --arg size "$current_size" --arg media_type "$media_type" \
        '{path: $path, title: $title, artist: $artist, album: $album, genre: $genre, mtime: $mtime, size: $size, media_type: $media_type}')
    fi

    # Append the resulting JSON object (as a single line) to our temp file.
    if [[ -n "$track_json_to_add" ]]; then
      echo "$track_json_to_add" >> "$new_index_lines"
    fi

  done
  printf "\rScanning and updating complete: %d/%d files processed.\n" "$total" "$total" >&2

  local current_indexed_dirs_json_array="[]"
  for dir_path in "${music_dirs[@]}"; do
    if [[ -d "$dir_path" ]]; then
      local trimmed_dir_path=$(echo "$dir_path" | tr -d '\n\r')
      local dir_mtime=$(get_mtime "$dir_path" || echo "")
      local dir_json=$(jq -n --arg path "$trimmed_dir_path" --arg mtime "$dir_mtime" '{path: $path, mtime: $mtime}')
      current_indexed_dirs_json_array=$(echo "$current_indexed_dirs_json_array" | jq --argjson new_dir "$dir_json" '. + [$new_dir]')
    fi
  done

  # --- OPTIMIZATION: EFFICIENT JSON ASSEMBLY ---
  # Save the state separate from tracks
  echo "$current_indexed_dirs_json_array" > "$DIRS_STATE_FILE"
  mv "$new_index_lines" "$MUSIC_INDEX_FILE"

  msg_success "Index updated and saved to $MUSIC_INDEX_FILE"

  # --- CLEANUP LEGACY JSON ---
    local legacy_index="${MUSIC_INDEX_FILE%.jsonl}.json"
    if [[ -f "$legacy_index" ]]; then
        rm "$legacy_index"
        log_verbose "Removed legacy index file: $(basename "$legacy_index")"
    fi
}
