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

# --- Temporary Index Build Function ---
# Builds a temporary index for a given directory.
build_temp_index() {
    local custom_dir="$1"
    local -n temp_index_ref=$2
    local ext_filter=("${EXT_FILTER[@]}")

    log_verbose "Temporarily indexing files from '$custom_dir' for selection..."

    create_temp_file temp_index_ref
    create_temp_file temp_json_lines

    # Detect if find supports -printf (Linux/GNU)
    local use_printf=false
    if find /dev/null -printf "" >/dev/null 2>&1; then use_printf=true; fi

    local count=0
    local total_files=0

    # PRE-FLIGHT COUNT: Calculate total files first for progress bar
    if [[ "$use_printf" == true ]]; then
         # Count dots (one per file)
         total_files=$(find "$custom_dir" -type f \( "${ext_filter[@]}" \) -printf '.' | wc -c)
    else
         # Fallback count
         total_files=$(find "$custom_dir" -type f \( "${ext_filter[@]}" \) | wc -l)
    fi

    # --- OPTIMIZATION: FAST PATH (Linux) vs SLOW PATH (Mac/BSD) ---
    if [[ "$use_printf" == true ]]; then
        # Linux: Get path, size, mtime in one pass
        while IFS= read -r -d '' file_path && IFS= read -r -d '' size && IFS= read -r -d '' mtime_full; do
            count=$((count + 1))

            # PROGRESS BAR
            if [[ "$VERBOSE" == true ]]; then
                 local fname=$(basename "$file_path")
                 if [[ ${#fname} -gt 30 ]]; then fname="${fname:0:27}..."; fi
                 printf "\rIndexing: %d/%d (%s)          " "$count" "$total_files" "$fname" >&2
            else
                 printf "\rIndexing: %d/%d          " "$count" "$total_files" >&2
            fi

            local mtime=${mtime_full%%.*}
            local raw_metadata_output
            raw_metadata_output=$(get_audio_metadata "$file_path")
            IFS=$'\t' read -r title artist album genre <<< "$raw_metadata_output"

            local file_ext="${file_path##*.}"
            file_ext="${file_ext,,}"
            local media_type="UNKNOWN"
            if [[ " ${AUDIO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="audio"
            elif [[ " ${VIDEO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="video"
            elif [[ " ${PLAYLIST_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="playlist"; fi

            echo "{\"path\":\"$(json_escape "$file_path")\",\"title\":\"$(json_escape "$title")\",\"artist\":\"$(json_escape "$artist")\",\"album\":\"$(json_escape "$album")\",\"genre\":\"$(json_escape "$genre")\",\"mtime\":\"$mtime\",\"size\":\"$size\",\"media_type\":\"$media_type\"}" >> "$temp_json_lines"

        done < <(find "$custom_dir" -type f \( "${ext_filter[@]}" \) -printf "%p\0%s\0%T@\0")
    else
        # Fallback for systems without -printf
        local all_files=()
        while IFS= read -r -d '' file; do all_files+=("$file"); done < <(find "$custom_dir" -type f \( "${ext_filter[@]}" \) -print0)
        # Total is already known here since we used array
        total_files=${#all_files[@]}

        for file_path in "${all_files[@]}"; do
            count=$((count + 1))

            # PROGRESS BAR RESTORED
            if [[ "$VERBOSE" == true ]]; then
                 local fname=$(basename "$file_path")
                 if [[ ${#fname} -gt 30 ]]; then fname="${fname:0:27}..."; fi
                 printf "\rIndexing: %d/%d (%s)          " "$count" "$total_files" "$fname" >&2
            else
                 printf "\rIndexing: %d/%d          " "$count" "$total_files" >&2
            fi

            local mtime=$(get_mtime "$file_path" || echo "")
            local size=$(get_size "$file_path" || echo "")
            local raw_metadata_output=$(get_audio_metadata "$file_path")
            IFS=$'\t' read -r title artist album genre <<< "$raw_metadata_output"

            local file_ext="${file_path##*.}"
            file_ext="${file_ext,,}"
            local media_type="UNKNOWN"
            if [[ " ${AUDIO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="audio"
            elif [[ " ${VIDEO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="video"
            elif [[ " ${PLAYLIST_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="playlist"; fi

            echo "{\"path\":\"$(json_escape "$file_path")\",\"title\":\"$(json_escape "$title")\",\"artist\":\"$(json_escape "$artist")\",\"album\":\"$(json_escape "$album")\",\"genre\":\"$(json_escape "$genre")\",\"mtime\":\"$mtime\",\"size\":\"$size\",\"media_type\":\"$media_type\"}" >> "$temp_json_lines"
        done
    fi

    echo "" >&2

    if [[ "$count" -eq 0 ]]; then
        msg_warn "No music files found in '$custom_dir'."
        : > "$temp_index_ref"
        rm "$temp_json_lines"
        return 1
    fi

    mv "$temp_json_lines" "$temp_index_ref"
    log_verbose "Temporary index created at '$temp_index_ref'."
}

# --- Music Library Indexing Function ---
# Builds or rebuilds the entire music index.
build_music_index() {
  local music_dirs=("${MUSIC_DIRS_ARRAY[@]}")
  local ext_filter=("${EXT_FILTER[@]}")
  local indexed_dirs_json_array="[]"

  log_verbose "Indexing music library for the first time..."

  create_temp_file temp_json_lines

  # Detect if find supports -printf
  local use_printf=false
  if find /dev/null -printf "" >/dev/null 2>&1; then use_printf=true; fi

  # PRE-FLIGHT COUNT: We must count ALL files across ALL dirs first for accurate X/Y
  local total_files=0
  log_verbose "Counting files..."
  for dir_path in "${music_dirs[@]}"; do
      if [[ -d "$dir_path" ]]; then
          local dir_count=0
          if [[ "$use_printf" == true ]]; then
             dir_count=$(find "$dir_path" -type f \( "${ext_filter[@]}" \) -printf '.' | wc -c)
          else
             dir_count=$(find "$dir_path" -type f \( "${ext_filter[@]}" \) | wc -l)
          fi
          total_files=$((total_files + dir_count))
      fi
  done

  local count=0

  # Populate all_music_files and indexed_dirs_json_array
  for dir_path in "${music_dirs[@]}"; do
    if [[ -d "$dir_path" ]]; then
      # Fix for newline: Ensure dir_path is trimmed before passing to jq
      local trimmed_dir_path
      trimmed_dir_path="$dir_path"
      local dir_mtime
      dir_mtime=$(get_mtime "$dir_path" || echo "")
      local dir_json
      dir_json=$(jq -n --arg path "$trimmed_dir_path" --arg mtime "$dir_mtime" '{path: $path, mtime: $mtime}')
      indexed_dirs_json_array=$(echo "$indexed_dirs_json_array" | jq --argjson new_dir "$dir_json" '. + [$new_dir]')

      # --- OPTIMIZED INDEXING LOOP ---
      if [[ "$use_printf" == true ]]; then
          while IFS= read -r -d '' file_path && IFS= read -r -d '' size && IFS= read -r -d '' mtime_full; do
              count=$((count + 1))

              # PROGRESS BAR
              if [[ "$VERBOSE" == true ]]; then
                   local fname=$(basename "$file_path")
                   if [[ ${#fname} -gt 30 ]]; then fname="${fname:0:27}..."; fi
                   printf "\rIndexing: %d/%d (%s)          " "$count" "$total_files" "$fname" >&2
              else
                   printf "\rIndexing: %d/%d          " "$count" "$total_files" >&2
              fi

              local mtime=${mtime_full%%.*}
              local raw_metadata_output="$(get_audio_metadata "$file_path")"
              IFS=$'\t' read -r -a metadata_array <<< "$raw_metadata_output"

              local title="${metadata_array[0]}"
              local artist="${metadata_array[1]}"
              local album="${metadata_array[2]}"
              local genre="${metadata_array[3]}"

              local file_ext="${file_path##*.}"
              file_ext="${file_ext,,}"
              local media_type="UNKNOWN"
              if [[ " ${AUDIO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="audio"
              elif [[ " ${VIDEO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="video"
              elif [[ " ${PLAYLIST_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="playlist"; fi

              echo "{\"path\":\"$(json_escape "$file_path")\",\"title\":\"$(json_escape "$title")\",\"artist\":\"$(json_escape "$artist")\",\"album\":\"$(json_escape "$album")\",\"genre\":\"$(json_escape "$genre")\",\"mtime\":\"$mtime\",\"size\":\"$size\",\"media_type\":\"$media_type\"}" >> "$temp_json_lines"
          done < <(find "$dir_path" -type f \( "${ext_filter[@]}" \) -printf "%p\0%s\0%T@\0")
      else
          # Fallback
          local all_music_files=()
          while IFS= read -r -d '' file; do all_music_files+=("$file"); done < <(find "$dir_path" -type f \( "${ext_filter[@]}" \) -print0)

          for file_path in "${all_music_files[@]}"; do
              count=$((count + 1))

              # PROGRESS BAR
              if [[ "$VERBOSE" == true ]]; then
                   local fname=$(basename "$file_path")
                   if [[ ${#fname} -gt 30 ]]; then fname="${fname:0:27}..."; fi
                   printf "\rIndexing: %d/%d (%s)          " "$count" "$total_files" "$fname" >&2
              else
                   printf "\rIndexing: %d/%d          " "$count" "$total_files" >&2
              fi

              local mtime=$(get_mtime "$file_path" || echo "")
              local size=$(get_size "$file_path" || echo "")
              local raw_metadata_output="$(get_audio_metadata "$file_path")"
              IFS=$'\t' read -r -a metadata_array <<< "$raw_metadata_output"

              local title="${metadata_array[0]}"
              local artist="${metadata_array[1]}"
              local album="${metadata_array[2]}"
              local genre="${metadata_array[3]}"

              local file_ext="${file_path##*.}"
              file_ext="${file_ext,,}"
              local media_type="UNKNOWN"
              if [[ " ${AUDIO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="audio"
              elif [[ " ${VIDEO_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="video"
              elif [[ " ${PLAYLIST_EXTS_ARRAY[*]} " =~ " ${file_ext} " ]]; then media_type="playlist"; fi

              echo "{\"path\":\"$(json_escape "$file_path")\",\"title\":\"$(json_escape "$title")\",\"artist\":\"$(json_escape "$artist")\",\"album\":\"$(json_escape "$album")\",\"genre\":\"$(json_escape "$genre")\",\"mtime\":\"$mtime\",\"size\":\"$size\",\"media_type\":\"$media_type\"}" >> "$temp_json_lines"
          done
      fi
    else
      msg_warn "Configured music directory '$dir_path' does not exist. Skipping."
    fi
  done

  # save directory state to its own file
  echo "$indexed_dirs_json_array" > "$DIRS_STATE_FILE"

  echo "" >&2
  if [[ "$count" -eq 0 ]]; then
    msg_warn "No music files found in configured directories. Index will be empty."
    : > "$MUSIC_INDEX_FILE"
    rm "$temp_json_lines"
    return 0
  fi

  # --- EFFICIENT JSON ASSEMBLY ---
  # Just move the JSONL file to the final location.
  mv "$temp_json_lines" "$MUSIC_INDEX_FILE"

  log_verbose "Index saved to $MUSIC_INDEX_FILE"

  # --- CLEANUP LEGACY JSON ---
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

  log_verbose "Updating music library index..."

  if [[ -f "$MUSIC_INDEX_FILE" ]]; then
    # Read line-by-line to build the map. Fast and low memory.
    # OPTIMIZATION: Dual-Stream Reader (jq + raw file)
    # Avoids spawning a subshell for every line.
    while IFS= read -r path <&3 && IFS= read -r line <&4; do
        if [[ -n "$path" && "$path" != "null" ]]; then
            old_index_map["$path"]="$line"
        fi
    done 3< <(jq -r .path "$MUSIC_INDEX_FILE") 4< "$MUSIC_INDEX_FILE"
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

  local cnt_new=0
  local cnt_mod=0
  local cnt_unchanged=0

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
    local trimmed_file_path="$file_path"

    if [[ -n "${old_index_map[$trimmed_file_path]+x}" ]]; then
      local old_track_json="${old_index_map[$trimmed_file_path]}"
      local old_mtime=$(echo "$old_track_json" | jq -r '.mtime // ""')
      local old_size=$(echo "$old_track_json" | jq -r '.size // ""')

      if [[ "$current_mtime" == "$old_mtime" && "$current_size" == "$old_size" ]]; then
        track_json_to_add="$old_track_json"
        cnt_unchanged=$((cnt_unchanged + 1))
        log_debug "Unchanged: $(basename "$file_path")"
      else
        log_debug "File modified: $(basename "$file_path") (Old Mtime: $old_mtime, New: $current_mtime)"
        cnt_mod=$((cnt_mod + 1))
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

        track_json_to_add="{\"path\":\"$(json_escape "$trimmed_file_path")\",\"title\":\"$(json_escape "$title")\",\"artist\":\"$(json_escape "$artist")\",\"album\":\"$(json_escape "$album")\",\"genre\":\"$(json_escape "$genre")\",\"mtime\":\"$current_mtime\",\"size\":\"$current_size\",\"media_type\":\"$media_type\"}"
      fi
    else
      log_verbose "New file detected: $(basename "$file_path")"
      cnt_new=$((cnt_new + 1))
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

      track_json_to_add="{\"path\":\"$(json_escape "$trimmed_file_path")\",\"title\":\"$(json_escape "$title")\",\"artist\":\"$(json_escape "$artist")\",\"album\":\"$(json_escape "$album")\",\"genre\":\"$(json_escape "$genre")\",\"mtime\":\"$current_mtime\",\"size\":\"$current_size\",\"media_type\":\"$media_type\"}"
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
      local trimmed_dir_path="$dir_path"
      local dir_mtime=$(get_mtime "$dir_path" || echo "")
      local dir_json=$(jq -n --arg path "$trimmed_dir_path" --arg mtime "$dir_mtime" '{path: $path, mtime: $mtime}')
      current_indexed_dirs_json_array=$(echo "$current_indexed_dirs_json_array" | jq --argjson new_dir "$dir_json" '. + [$new_dir]')
    fi
  done

  # --- OPTIMIZATION: EFFICIENT JSON ASSEMBLY ---
  # Save the state separate from tracks
  echo "$current_indexed_dirs_json_array" > "$DIRS_STATE_FILE"
  mv "$new_index_lines" "$MUSIC_INDEX_FILE"

  # SUMMARY REPORT
  msg_success "Index refreshed."
  msg_info "Stats: $total Scanned | $cnt_new New | $cnt_mod Modified | $cnt_unchanged Unchanged"
  log_verbose "Index updated and saved to $MUSIC_INDEX_FILE"

  # --- CLEANUP LEGACY JSON ---
    local legacy_index="${MUSIC_INDEX_FILE%.jsonl}.json"
    if [[ -f "$legacy_index" ]]; then
        rm "$legacy_index"
        log_verbose "Removed legacy index file: $(basename "$legacy_index")"
    fi
}
