# --- Helper Functions (Defined before main execution logic) ---

# --- Help Function ---
show_help() {
  cat <<EOF
🦍 MPV Music Script – Monke Wrapper 🍌 (v$VERSION)

Usage:
  mpv-music [PATH_OR_URL_OR_DIR] [OPTIONS]
  mpv-music [FILTER_FLAGS] [--play-all]

Examples:
  # Interactive Modes
  mpv-music                    # Start interactive menu to pick mode
  mpv-music /path/to/music     # Interactively pick tracks/albums from a specific directory
  mpv-music -l                 # Go directly to interactive Playlist Mode

  # Direct Play & Filtering
  mpv-music ~/Music/song.mp3   # Play a specific local file
  mpv-music --genre="Rock"       # Interactively select from all Rock artists
  mpv-music --artist="ado"      # Find all albums by Ado, then choose
  mpv-music --album="cozy" --play-all # Play the album "Cozy" directly
  mpv-music --title="beach"   # Display all tracks containing "Beach"
  mpv-music -g "Electronic" -a "Daft Punk" -p # Play all tracks by Daft Punk in Electronic genre

Options:
  -h, --help          Show this help message and exit
  -v, --version       Show version and exit
  -p, --play-all      Play all tracks matching filters directly, skipping selection.
  -l, --playlist      Go directly to interactive Playlist Mode.
  -g, --genre [val]   Filter by genre. Opens interactive picker if no value is given.
  -a, --artist [val]  Filter by artist. Opens interactive picker if no value is given.
  -b, --album [val]   Filter by album. Opens interactive picker if no value is given.
  -t, --title [val]   Filter by title.
  --config[=EDITOR]   Open config file in an editor.
  --video-ok          Include video file formats in scans.
  --update            Update mpv-music to the latest version
  --reindex           Force a complete rebuild of the music index.
  --refresh-index     Update index with file changes (smarter, faster).
  -V, --verbose       Increase verbosity level.
  --debug             Print debug messages (saves to log with -V).

Interactive Modes (when run with no arguments):
  1) Directory Mode:    Pick entire folders to play.
  2) Track Mode:        Pick individual song files from your library.
  3) Playlist Mode:     Pick and play saved playlists (.m3u, etc.).
  4) Tag Filter Mode:   Filter by Genre, then Artist, then Album.
  5) Play All Mode:     Instantly play all indexed tracks.

Note: Requires GNU find, fzf, jq, and ffprobe.
EOF
}




# --- Play All Music Function ---
play_all_music() {
    log_verbose "🎵 Getting all tracks from the index..."

    # Check if index exists and is not empty
    if [[ ! -s "$MUSIC_INDEX_FILE" ]]; then
        echo "Error: Music index is empty or not found. Cannot play all." >&2
        echo "Try running --reindex first." >&2
        exit 1
    fi

    mapfile -t FILES < <(jq -r '.tracks[].path' "$MUSIC_INDEX_FILE")

    if [[ ${#FILES[@]} -eq 0 ]]; then
        echo "No tracks found in the index. 🙊" >&2
        exit 1
    fi

    log_verbose "🎶 Loading all ${#FILES[@]} track(s)..."
    mpv "${MPV_ARGS[@]}" "${FILES[@]}"
    exit 0
}

# --- Interactive Filter Helper ---
# Prompts the user to select a value for a given metadata key (e.g., genre, artist).
interactive_filter() {
    local filter_key="$1"
    local fzf_prompt="$2"
    local temp_filter_list
    create_temp_file temp_filter_list

    # OPTIMIZATION: Pre-calculate counts and samples for all tags in a single jq pass.
    # This avoids calling jq repeatedly inside the fzf preview command.
    jq -r --arg key "$filter_key" '
        [.tracks[] | select(.[$key] != null and .[$key] != "" and .[$key] != "Playlist")] |
        group_by(.[$key]) | .[] |
        [
            .[0][$key],                           # Field 1: Tag value (e.g., "Rock")
            length,                               # Field 2: Track count for this tag
            ([.[0:5][].title] | join(" | "))      # Field 3: Sample list for preview
        ] | @tsv
    ' "$INDEX_TO_USE" | sort -f > "$temp_filter_list"

    # OPTIMIZATION: Preview now uses fzf's fast internal substitutions.
    local selected_lines
    selected_lines=$(cat "$temp_filter_list" | \
        fzf --multi \
            --delimiter="\t" \
            --with-nth=1 \
            --prompt="$fzf_prompt" \
            --preview='echo -e "\033[1;36mTag:\033[0m {1}\n\033[1;33mTracks:\033[0m {2}\n\033[1;32mSample:\033[0m {3}"' \
            --preview-window=top:5)

    if [[ -z "$selected_lines" ]]; then
        echo "No selection made. Exiting." >&2
        exit 1
    fi

    # Output only the tag value (the first column)
    echo "$selected_lines" | cut -d$'\t' -f1
}

# --- Modes: Functions ---
run_dir_mode() {
    if [[ ! -f "$INDEX_TO_USE" || ! -s "$INDEX_TO_USE" ]]; then #
        echo "Error: Index file is missing or empty. Cannot proceed." #
        exit 1 #
    fi

    local temp_folder_list
    create_temp_file temp_folder_list

    # OPTIMIZATION: Use a single jq command to group tracks by directory,
    # count them, and create a preview list. This is much faster than the shell loop.
    jq -r '
        .tracks | group_by(.path | split("/")[:-1] | join("/")) | .[] |
        (.[0].path | split("/")[:-1] | join("/")) as $dir_path |
        [
            ($dir_path | split("/") | .[-1]), # Field 1: Directory base name
            $dir_path,                        # Field 2: Full directory path
            length,                           # Field 3: Number of tracks in dir
            ([.[0:5][].title] | join(", "))    # Field 4: Sample track list
        ] | @tsv
    ' "$INDEX_TO_USE" > "$temp_folder_list"

    if [[ ! -s "$temp_folder_list" ]]; then #
        echo "No playable music folders found in the selection. Please check your source and try again."
        exit 1
    fi

    # OPTIMIZATION: The preview now uses fzf's internal field substitution,
    # which is instantaneous and avoids calling external commands.
    local SELECTED
    SELECTED=$(cat "$temp_folder_list" | fzf --multi \
        --delimiter="\t" \
        --with-nth=1 \
        --prompt="📁 Pick folder(s) (TAB to multi-select): " \
        --preview='echo -e "Folder: {1}\nPath: {2}\nTracks: {3}\nSample: {4}"' \
        --preview-window=top:6 | cut -d$'\t' -f2) || {
        echo "🚶 No folders picked."
        exit 1
    }

    mapfile -t FOLDERS <<< "$SELECTED"
    log_verbose "📦 Selected ${#FOLDERS[@]} folder(s)."

    local FILES=() #
    for DIR in "${FOLDERS[@]}"; do
        # Use jq to find all tracks whose paths start with the selected directory's path
        local TRACK_PATHS
        TRACK_PATHS=$(jq -r --arg dir_prefix "${DIR}/" '.tracks[] | select(.path | startswith($dir_prefix)) | .path' "$INDEX_TO_USE")
        while IFS= read -r TRACK_FILE; do #
            [[ -n "$TRACK_FILE" ]] && FILES+=("$TRACK_FILE")
        done <<< "$TRACK_PATHS"
    done

    [[ ${#FILES[@]} -eq 0 ]] && echo "No music found in those folders. Monke hear nothing 🙊" && exit 1
    log_verbose "🎶 Found ${#FILES[@]} file(s) total."
    mpv "${MPV_ARGS[@]}" "${FILES[@]}" #
}

run_track_mode() {
    if [[ ! -f "$INDEX_TO_USE" || ! -s "$INDEX_TO_USE" ]]; then #
        echo "Error: Index file is missing or empty. Cannot proceed." #
        exit 1
    fi

    local temp_track_list
    create_temp_file temp_track_list

    # OPTIMIZATION: Create the full data line for fzf in a single jq call.
    jq -r '.tracks[] |
      select(.artist != "Playlist") | #
      [
          (if .media_type == "video" then "🎬 " else "🎵 " end) + (.title // "[NO TITLE]"), # Field 1
          .title // "[NO TITLE]",    # Field 2
          .artist // "[NO ARTIST]",  # Field 3
          .album // "[NO ALBUM]",    # Field 4
          .genre // "[NO GENRE]",    # Field 5
          .media_type // "UNKNOWN",  # Field 6
          .path                      # Field 7 (The final value we need)
      ] | @tsv' "$INDEX_TO_USE" > "$temp_track_list"

    # OPTIMIZATION: Use fzf to parse the columns passed to it.
    local SELECTED
    SELECTED=$(cat "$temp_track_list" | fzf --multi \
      --prompt="🎵 Pick your tracks (TAB to multi-select): " \
      --delimiter="\t" \
      --with-nth=1 \
      --preview='echo -e "\033[1;36mTitle:\033[0m {2}\n\033[1;33mArtist:\033[0m {3}\n\033[1;32mAlbum:\033[0m {4}\n\033[1;35mGenre:\033[0m {5}\n\033[1;34mType:\033[0m {6}"' \
      --preview-window=top:6 | awk -F'\t' '{print $NF}')

    mapfile -t FILES <<< "$SELECTED"
    [[ ${#FILES[@]} -eq 0 ]] && echo "No tracks picked. Monke walk away. 🚶" && exit 1
    log_verbose "🎶 Selected ${#FILES[@]} track(s)."
    mpv "${MPV_ARGS[@]}" "${FILES[@]}"
}

run_playlist_mode() {
    local temp_playlist_list
    create_temp_file temp_playlist_list

    # MODIFICATION: Changed the output to be Tab-Separated Values (@tsv) for consistency.
    jq -r '.tracks[] | select(.artist == "Playlist") |
      [
          "📜 " + .title, # Field 1: Display name with icon
          .path          # Field 2: The file path
      ] | @tsv' "$INDEX_TO_USE" > "$temp_playlist_list"

    if [[ ! -s "$temp_playlist_list" ]]; then #
        echo "No playlists found in the index. 🐒" >&2
        echo "Try running --reindex to add them." >&2
        exit 1
    fi

    local SELECTED_PATHS
    SELECTED_PATHS=$(cat "$temp_playlist_list" | fzf --multi \
        --delimiter="\t" \
        --with-nth=1 \
        --prompt="📜 Pick playlist(s) (TAB to multi-select): " \
        --preview-window=top:5 \
        --preview='cat {2}') || {
        echo "No playlist picked. Monke sad. 🍌" #
        exit 1
    }

    mapfile -t FILES < <(echo "$SELECTED_PATHS" | cut -d$'\t' -f2)

    if [[ ${#FILES[@]} -eq 0 ]]; then
        echo "No playlists picked. Monke walk away. 🚶"
        exit 1
    fi

    log_verbose "🎶 Loading ${#FILES[@]} playlist(s)."
    mpv "${MPV_ARGS[@]}" "${FILES[@]}"
}

run_tag_mode() {
    echo "🔎 Filter by:"
    echo "1) Genre"
    echo "2) Artist"
    echo "3) Album"
    read -rp "Enter choice [1/2/3]: " FILTER_CHOICE

    filter_key=""
    fzf_prompt=""

    case "$FILTER_CHOICE" in
        1) filter_key="genre"; fzf_prompt="🎶 Pick genre(s) (TAB to select multiple): ";;
        2) filter_key="artist"; fzf_prompt="🎤 Pick artist(s) (TAB to select multiple): ";;
        3) filter_key="album"; fzf_prompt="💿 Pick album(s) (TAB to select multiple): ";;
        *) echo "Invalid choice. Exiting."; exit 1;;
    esac

    # Get one or more selections from the user
    selected_filter_values_str=$(interactive_filter "$filter_key" "$fzf_prompt")
    mapfile -t selected_values_array <<< "$selected_filter_values_str"

    # Create a JSON array (e.g., ["Rock", "Electronic"]) for jq
    jq_values_array=$(printf '%s\n' "${selected_values_array[@]}" | jq -R . | jq -s .)

    log_verbose "Filtering by $filter_key: ${selected_values_array[*]}..."

    # Create a temporary index with tracks matching ANY of the selected values
    create_temp_file filtered_index_file
    jq --arg key "$filter_key" --argjson values "$jq_values_array" \
        '{tracks: [.tracks[] | select(.[$key] as $k | $values | index($k))]}' "$INDEX_TO_USE" > "$filtered_index_file"

    track_count=$(jq '.tracks | length' "$filtered_index_file")

    if [[ "$track_count" -eq 0 ]]; then
        echo "No tracks found matching that filter. 🙊" >&2
        exit 1
    fi

    log_verbose "Found $track_count matching tracks. What's next?"
    echo "1) Play all $track_count tracks"
    echo "2) Select individual tracks from this list"
    read -rp "Enter choice [1/2]: " PLAY_CHOICE

    case "$PLAY_CHOICE" in
        1) # Play All
            mapfile -t FILES < <(jq -r '.tracks[].path' "$filtered_index_file")
            log_verbose "🎶 Loading all ${#FILES[@]} track(s)..."
            mpv "${MPV_ARGS[@]}" "${FILES[@]}"
            ;;
        2) # Select Individual
            create_temp_file temp_track_list
            jq -r '.tracks[] |
                  (if .media_type == "video" then "🎬 " else "🎵 " end) +
                  (.title // "[NO TITLE]") + " " + "|" +
                  (.title // "[NO TITLE]") + "|" +
                  (.artist // "[NO ARTIST]") + "|" +
                  (.album // "[NO ALBUM]") + "|" +
                  (.genre // "[NO GENRE]") + "|" +
                  (.media_type // "UNKNOWN") + "|" +
                  .path' "$filtered_index_file" > "$temp_track_list"

            SELECTED=$(cat "$temp_track_list" | fzf --multi \
              --prompt="🎵 Pick your filtered tracks (TAB to multi-select): " \
              --delimiter="|" \
              --with-nth=1 \
              --preview='echo -e "\033[1;36mTitle:\033[0m {2}\n\033[1;33mArtist:\033[0m {3}\n\033[1;32mAlbum:\033[0m {4}\n\033[1;35mGenre:\033[0m {5}\n\033[1;34mType:\033[0m {6}"' \
              --preview-window=top:5 | awk -F'|' '{print $NF}')

            mapfile -t FILES <<< "$SELECTED"
            [[ ${#FILES[@]} -eq 0 ]] && echo "No tracks picked. Monke walk away. 🚶" && exit 1

            log_verbose "🎶 Selected ${#FILES[@]} track(s)."
            mpv "${MPV_ARGS[@]}" "${FILES[@]}"
            ;;
        *)
            echo "Invalid choice. Exiting."
            exit 1
            ;;
    esac
}

run_play_all_mode() {
    play_all_music
}
