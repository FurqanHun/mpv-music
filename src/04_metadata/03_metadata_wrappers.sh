# --- Music Library Indexing Function ---
try_rust_indexer() {
    local indexer_bin="$CONFIG_DIR/mpv-music-indexer"

    if [[ ! -x "$indexer_bin" ]]; then
        return 1
    fi

    log_verbose "Rust Indexer found. Engaging Rust Indexer..."

    # Prepare Arguments
    local args=()
    args+=("--audio-exts" "${AUDIO_EXTS[*]}")
    args+=("--video-exts" "${VIDEO_EXTS[*]}")
    args+=("--playlist-exts" "${PLAYLIST_EXTS[*]}")

    if [[ "$VIDEO_OK" == true ]]; then
        args+=("--video")
    fi

    # Directories must be last
    args+=("${MUSIC_DIRS_ARRAY[@]}")

    # Run into a temp file first (Safe Write)
    local temp_rust_out="${MUSIC_INDEX_FILE}.rust.tmp"

    if "$indexer_bin" "${args[@]}" > "$temp_rust_out"; then
        mv "$temp_rust_out" "$MUSIC_INDEX_FILE"
        log_verbose "Rust Indexing Complete."

        # Cleanup legacy .json if it exists
        local legacy_file="${MUSIC_INDEX_FILE%.jsonl}.json"
        [[ -f "$legacy_file" ]] && rm "$legacy_file"

        return 0
    else
        log_verbose "Rust indexer failed (Exit: $?). Falling back..."
        rm -f "$temp_rust_out"
        return 1
    fi
}

build_temp_index() {
    local custom_dir="$1"
    # We pass the NAME of the variable holding the temp file path (for nameref)
    local output_var_name="$2"

    local indexer_bin="$CONFIG_DIR/mpv-music-indexer"

    if [[ -x "$indexer_bin" ]]; then
        log_verbose "Rust Indexer Found! Fast indexing '$custom_dir'..."

        # We need to resolve the output file path from the nameref passed in $2
        # Use a local nameref to get the value, or just create a new temp file if needed.
        # However, the caller expects $2 to be populated with the filename.

        # Create a temp file and assign it to the variable passed as $2
        create_temp_file "$output_var_name"
        local -n out_ref="$output_var_name"

        # Prepare Arguments
        local args=()
        args+=("--audio-exts" "${AUDIO_EXTS[*]}")
        args+=("--video-exts" "${VIDEO_EXTS[*]}")
        args+=("--playlist-exts" "${PLAYLIST_EXTS[*]}")

        if [[ "$VIDEO_OK" == true ]]; then
            args+=("--video")
        fi

        # Target Directory
        args+=("$custom_dir")

        # Execute
        if "$indexer_bin" "${args[@]}" > "$out_ref"; then
            return 0
        fi

        log_verbose "Rust indexer failed. Triggering legacy temp build..."
    fi

    legacy_build_temp_index "$custom_dir" "$output_var_name"
}

build_music_index() {
    reload_config_state

    if try_rust_indexer; then
        return 0
    fi

    log_verbose "Rust indexer not found. Triggering legacy build..."
    # Fallback
    legacy_build_music_index
}

update_music_index() {
    reload_config_state

    if try_rust_indexer; then
        return 0
    fi

    log_verbose "Rust indexer not found. Triggering legacy update..."

    # Fallback
    legacy_update_music_index
}
