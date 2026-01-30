# --- Temporary Index Build Function ---
# Builds a temporary index for a given directory.
legacy_build_temp_index() {
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

# Builds or rebuilds the entire music index.
legacy_build_music_index() {
  reload_config_state
  local music_dirs=("${MUSIC_DIRS_ARRAY[@]}")
  local ext_filter=("${EXT_FILTER[@]}")

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

  # Populate all_music_files
  for dir_path in "${music_dirs[@]}"; do
    if [[ -d "$dir_path" ]]; then

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
legacy_update_music_index() {
  reload_config_state
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

  # --- OPTIMIZATION: EFFICIENT JSON ASSEMBLY ---
  mv "$new_index_lines" "$MUSIC_INDEX_FILE"

  # SUMMARY REPORT
  log_verbose "Index refreshed."
  log_verbose "Stats: $total Scanned | $cnt_new New | $cnt_mod Modified | $cnt_unchanged Unchanged"
  log_verbose "Index updated and saved to $MUSIC_INDEX_FILE"

  # --- CLEANUP LEGACY JSON ---
    local legacy_index="${MUSIC_INDEX_FILE%.jsonl}.json"
    if [[ -f "$legacy_index" ]]; then
        rm "$legacy_index"
        log_verbose "Removed legacy index file: $(basename "$legacy_index")"
    fi
}
