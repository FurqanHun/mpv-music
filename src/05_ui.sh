# --- Helper Functions (Defined before main execution logic) ---

# --- Help Function ---
show_help() {
  cat <<EOF
MPV Music (v$VERSION)

Usage:
  mpv-music
  mpv-music [PATH_OR_URL_OR_DIR] [OPTIONS]
  mpv-music [FILTER_FLAGS] [--play-all]

Options:
  -h, --help          Show this help message and exit
  -v, --version       Show version and exit
  -p, --play-all      Play all tracks matching filters directly, skipping selection.
  -l, --playlist      Go directly to interactive Playlist Mode.
  -g, --genre [val]   Filter by genre. Opens interactive picker if no value is given.
  -a, --artist [val]  Filter by artist. Opens interactive picker if no value is given.
  -b, --album [val]   Filter by album. Opens interactive picker if no value is given.
  -t, --title [val]   Filter by title.
  --config [EDITOR]   Open config file in an editor.
  --log [VIEWER]      Set log level.
  --remove-config     Remove config file.
  --remove-log        Remove log file.
  --video-ok          Include video file formats in scans.
  --update            Update mpv-music to the latest version
  --reindex           Force a complete rebuild of the music index.
  --refresh-index     Update index with file changes (smarter, faster).
  -V, --verbose       Increase verbosity level.
  --debug             Print debug messages and enable system tracing.

Interactive Modes (when run with no arguments):
  1) Directory Mode:    Pick entire folders to play.
  2) Track Mode:        Pick individual song files from your library.
  3) Playlist Mode:     Pick and play saved playlists (.m3u, etc.).
  4) Tag Filter Mode:   Filter by Genre, then Artist, then Album.
  5) Play All Mode:     Instantly play all indexed tracks.
  6) Play URL (Stream)  Stream a URL to MPV using yt-dlp.
  7) Manage Directories Manage music directories to index.

Note: Requires GNU find, fzf, jq, and ffprobe.
EOF
}

# --- Play All Music Function ---
play_all_music() {
    log_verbose "Getting all tracks from the index..."

    # Check if index exists
    if [[ ! -s "$INDEX_TO_USE" ]]; then
        msg_error "Music index is empty or not found. Cannot play all."
        msg_note "Try running --reindex first."
        exit 1
    fi

    msg_success "Streaming all tracks to MPV..."

    jq -r '.path' "$INDEX_TO_USE" | mpv "${MPV_ARGS[@]}" --playlist=-

    exit 0
}

# --- Interactive Filter Helper ---
# Prompts the user to select a value for a given metadata key (e.g., genre, artist).
interactive_filter() {
    local filter_key="$1"
    local fzf_prompt="$2"
    local temp_filter_list
    create_temp_file temp_filter_list

    local icon=""
    case "$filter_key" in
        "genre")  icon="🎶 " ;;
        "artist") icon="🎤 " ;;
        "album")  icon="💿 " ;;
    esac

    # GENERATE LIST
    # Col 1: ID
    # Col 2: Visual Tag (Icon + Name) this is what we see in the list.
    # Col 3: Raw Tag (Name Only)     what we return to the script.
    # Col 4: Count                   hidden from list, shown in preview.
    # Col 5: Samples                 hidden from list, shown in preview.
    jq -rs --arg key "$filter_key" --arg icon "$icon" '
        map(select(.[$key] != null and .[$key] != "" and .[$key] != "Playlist")) |
        map({
            tag: (.[$key] | tostring | gsub("[,/]"; ",") | split(",") | .[] | sub("^\\s+";"") | sub("\\s+$";"")),
            track: .
        }) |
        group_by(.tag) | .[] |
        [
            ($icon + .[0].tag),                           # Field 2: Visual
            .[0].tag,                                     # Field 3: Raw
            length,                                       # Field 4: Count
            ([.[0:5][] | .track.title] | join(" | "))     # Field 5: Samples
        ] | @tsv
    ' "$INDEX_TO_USE" | sort -f | nl -w1 -s$'\t' > "$temp_filter_list"

    # SELECT
    local selected_lines
        selected_lines=$(cat "$temp_filter_list" | \
            fzf --multi \
                --delimiter="\t" \
                --with-nth=2 \
                --prompt="$fzf_prompt" \
                --preview="
                    RAW=\$(sed -n {1}p '$temp_filter_list');
                    IFS=\$'\t' read -r _ visual raw count samples <<< \"\$RAW\";
                    echo -e \"\033[1;36mTag:\033[0m \${raw}\";
                    echo -e \"\033[1;33mTracks:\033[0m \${count}\";
                    echo -e \"\033[1;32mSamples:\033[0m \${samples}\";
                " \
                --preview-window=top:5) || true  # <--- Added || true

        if [[ -z "$selected_lines" ]]; then
            msg_warn "No selection made. Exiting."
            exit 1
        fi

        echo "$selected_lines" | cut -d$'\t' -f3
}

# --- Modes: Functions ---
run_dir_mode() {
    if [[ ! -f "$INDEX_TO_USE" || ! -s "$INDEX_TO_USE" ]]; then
        msg_error "Index file is missing or empty. Cannot proceed."
        exit 1
    fi

    local temp_folder_list
    create_temp_file temp_folder_list

    # OPTIMIZATION: Group tracks by directory.
    # slurp and map() cuz group_by needs an array.
    jq -rs '
        map(select(.path != null)) |
        group_by(.path | split("/")[:-1] | join("/")) | .[] |
        (.[0].path | split("/")[:-1] | join("/")) as $dir_path |
        [
            "📁 " + ($dir_path | split("/") | .[-1]), # Field 2: Visual (Has Emoji)
            $dir_path,                                # Field 3: Full Path (Clean)
            length,                                   # Field 4: Count
            ([.[0:5][].title] | join(", "))           # Field 5: Samples
        ] | @tsv
    ' "$INDEX_TO_USE" | nl -w1 -s$'\t' > "$temp_folder_list"

    if [[ ! -s "$temp_folder_list" ]]; then
        msg_warn "No playable music folders found in the selection."
        exit 1
    fi

    local SELECTED
    SELECTED=$(cat "$temp_folder_list" | fzf --multi \
        --delimiter="\t" \
        --with-nth=2 \
        --prompt="📁 Pick folder(s) (TAB to multi-select): " \
        --preview="
            RAW=\$(sed -n {1}p '$temp_folder_list');

            # We read 'visual' but we IGNORE it for the echo.
            # We use 'path' to get the clean name.
            IFS=\$'\t' read -r _ visual path count samples <<< \"\$RAW\";

            # CLEAN NAME: Strip path to just the folder name
            CLEAN_NAME=\"\${path##*/}\";

            echo -e \"\033[1;36mFolder:\033[0m \${CLEAN_NAME}\";
            echo -e \"\033[1;34mPath:\033[0m \${path}\";
            echo -e \"\033[1;33mTracks:\033[0m \${count}\";
            echo -e \"\033[1;32mSample:\033[0m \${samples}\";
        " \
        --preview-window=top:6) || {
        msg_warn "No folders picked."
        exit 1
    }

    [[ -z "$SELECTED" ]] && msg_warn "No folders picked." && exit 1

    # Extract Paths (Column 3)
    mapfile -t FOLDERS < <(echo "$SELECTED" | cut -d$'\t' -f3)
    log_verbose "Selected ${#FOLDERS[@]} folder(s)."

    local FILES=()
    for DIR in "${FOLDERS[@]}"; do
        local TRACK_PATHS
        # Use jq to find files in this dir.
        TRACK_PATHS=$(jq -r --arg dir_prefix "${DIR}/" 'select(.path | startswith($dir_prefix)) | .path' "$INDEX_TO_USE")
        while IFS= read -r TRACK_FILE; do
            [[ -n "$TRACK_FILE" ]] && FILES+=("$TRACK_FILE")
        done <<< "$TRACK_PATHS"
    done

    [[ ${#FILES[@]} -eq 0 ]] && msg_warn "No music found in those folders." && exit 1

    msg_success "Playing ${#FILES[@]} file(s)..."
    mpv "${MPV_ARGS[@]}" "${FILES[@]}"
}

run_track_mode() {
    if [[ ! -f "$INDEX_TO_USE" || ! -s "$INDEX_TO_USE" ]]; then
        msg_error "Index file is missing or empty. Cannot proceed."
        exit 1
    fi

    local temp_track_list
    create_temp_file temp_track_list

    # LINE NUMBERS
    jq -r 'select(.media_type != "playlist") |
      [
          (if .media_type == "video" then "🎬 " else "🎵 " end) + (.title // "[NO TITLE]"),
          .title // "[NO TITLE]",
          .artist // "[NO ARTIST]",
          .album // "[NO ALBUM]",
          .genre // "[NO GENRE]",
          .media_type // "UNKNOWN",
          .path
      ] | @tsv' "$INDEX_TO_USE" | nl -w1 -s$'\t' > "$temp_track_list"

    local SELECTED
    SELECTED=$(cat "$temp_track_list" | fzf --multi \
      --prompt="🎵 Pick your tracks (TAB to multi-select): " \
      --delimiter="\t" \
      --with-nth=2 \
      --preview="
          RAW=\$(sed -n {1}p '$temp_track_list');
          IFS=\$'\t' read -r _ visual title artist album genre type path <<< \"\$RAW\";
          echo -e \"\033[1;36mTitle:\033[0m \${title}\";
          echo -e \"\033[1;33mArtist:\033[0m \${artist}\";
          echo -e \"\033[1;32mAlbum:\033[0m \${album}\";
          echo -e \"\033[1;35mGenre:\033[0m \${genre}\";
          echo -e \"\033[1;34mPath:\033[0m \${path}\";
      " \
      --preview-window=top:6 | awk -F'\t' '{print $NF}') || true

    if [[ -z "$SELECTED" ]]; then
        msg_warn "No tracks picked."
        exit 1
    fi

    mapfile -t FILES <<< "$SELECTED"

    msg_success "Playing ${#FILES[@]} track(s)..."
    mpv "${MPV_ARGS[@]}" "${FILES[@]}"
}

run_playlist_mode() {
    local temp_playlist_list
    create_temp_file temp_playlist_list

    jq -r 'select(.media_type == "playlist") |
      [
          "📜 " + (.title // (.path | split("/") | .[-1])),
          .path
      ] | @tsv' "$INDEX_TO_USE" | nl -w1 -s$'\t' > "$temp_playlist_list"

    if [[ ! -s "$temp_playlist_list" ]]; then
        msg_warn "No playlists found in the index."
        return
    fi

    local SELECTED
    SELECTED=$(cat "$temp_playlist_list" | fzf --multi \
        --delimiter="\t" \
        --with-nth=2 \
        --prompt="📜 Select playlist(s) (TAB to multi-select): " \
        --preview="
            RAW=\$(sed -n {1}p '$temp_playlist_list');
            IFS=\$'\t' read -r _ title path <<< \"\$RAW\";

            # REMOVE EMOJI: Strip everything up to the first space
            CLEAN_TITLE=\"\${title#* }\";

            echo -e \"\033[1;34mPlaylist:\033[0m \${CLEAN_TITLE}\";
            if [[ -f \"\$path\" ]]; then
                echo -e \"\033[1;33mContents (Top 10):\033[0m\";
                head -n 10 \"\$path\";
            else
                echo -e \"\033[1;31mError: File not found.\033[0m\";
            fi
        " \
        --preview-window=top:10 | cut -d$'\t' -f3) || true

    [[ -z "$SELECTED" ]] && msg_warn "No playlist picked." && return

    mapfile -t PLAYLISTS <<< "$SELECTED"
    mpv "${MPV_ARGS[@]}" "${PLAYLISTS[@]}"
}

run_tag_mode() {
    echo -e "${CYAN}🔎 Filter by:${NC}"
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
        *) msg_error "Invalid choice. Exiting."; exit 1;;
    esac

    # Get one or more selections from the user
    selected_filter_values_str=$(interactive_filter "$filter_key" "$fzf_prompt")
    mapfile -t selected_values_array <<< "$selected_filter_values_str"

    # Create a JSON array
    jq_values_array=$(printf '%s\n' "${selected_values_array[@]}" | jq -R . | jq -s .)

    log_verbose "Filtering by $filter_key: ${selected_values_array[*]}..."

    # Create a temporary index with tracks matching ANY of the selected values
    create_temp_file filtered_index_file

# stream filter to a temporary JSONL file.
    # LOGIC:
    # 1. Turn "Rock, Pop" into ["Rock", "Pop"]
    # 2. Check if the User's Selection is EXACTLY inside that list.
    jq -c --arg key "$filter_key" --argjson values "$jq_values_array" '
        select(
            # 1. CLEAN & SPLIT the track tags
            (.[$key] // "" | tostring | gsub("[,/]"; ",") | split(",") | map(sub("^\\s+";"") | sub("\\s+$";""))) as $track_tags |

            # 2. EXACT MATCH CHECK
            # We check if ANY of the user selections ($values) exist strictly inside $track_tags
            # index($v) returns a number (found) or null (not found).
            # In jq, numbers are truthy, null is falsy.
            $values | any( . as $v | ($track_tags | index($v)) )
        )
    ' "$INDEX_TO_USE" > "$filtered_index_file"

    # wc -l because it is a file of lines, not a JSON array
    track_count=$(wc -l < "$filtered_index_file")

    if [[ "$track_count" -eq 0 ]]; then
        msg_warn "No tracks found matching that filter."
        exit 1
    fi

    msg_success "Found $track_count matching track(s)."
    echo "What's next?"
    echo "1) Play all $track_count tracks"
    echo "2) Select individual tracks"
    read -rp "Enter choice [1/2]: " PLAY_CHOICE

    case "$PLAY_CHOICE" in
        1) # Play All
            mapfile -t FILES < <(jq -r '.path' "$filtered_index_file")
            msg_success "Playing all..."
            mpv "${MPV_ARGS[@]}" "${FILES[@]}"
            ;;
        2) # Select Individual
            create_temp_file temp_track_list

            # line number here too
            jq -r '
                  [
                      (if .media_type == "video" then "🎬 " else "🎵 " end) + (.title // "[NO TITLE]"),
                      .title // "[NO TITLE]",
                      .artist // "[NO ARTIST]",
                      .album // "[NO ALBUM]",
                      .genre // "[NO GENRE]",
                      .media_type // "UNKNOWN",
                      .path
                  ] | @tsv' "$filtered_index_file" | nl -w1 -s$'\t' > "$temp_track_list"

            # safe preview
            SELECTED=$(cat "$temp_track_list" | fzf --multi \
              --prompt="🎵 Pick your filtered tracks (TAB to multi-select): " \
              --delimiter="\t" \
              --with-nth=2 \
              --preview="
                  RAW=\$(sed -n {1}p '$temp_track_list');
                  IFS=\$'\t' read -r _ display title artist album genre type path <<< \"\$RAW\";
                  echo -e \"\033[1;36mTitle:\033[0m \${title}\";
                  echo -e \"\033[1;33mArtist:\033[0m \${artist}\";
                  echo -e \"\033[1;32mAlbum:\033[0m \${album}\";
                  echo -e \"\033[1;35mGenre:\033[0m \${genre}\";
                  echo -e \"\033[1;34mType:\033[0m \${type}\";
              " \
              --preview-window=top:6 | awk -F'\t' '{print $NF}')

            [[ -z "$SELECTED" ]] && msg_warn "No tracks picked." && exit 1
            mapfile -t FILES <<< "$SELECTED"

            msg_success "Playing ${#FILES[@]} track(s)..."
            mpv "${MPV_ARGS[@]}" "${FILES[@]}"
            ;;
        *)
            msg_warn "Invalid choice. Exiting."
            exit 1
            ;;
    esac
}

run_play_all_mode() {
    play_all_music
}

# --- Management Mode ---
run_manage_dirs_mode() {
    while true; do
        echo -e "\n${CYAN}📂 Managed Directories:${NC}"

        # FREEEEESH DATA (Populates MUSIC_DIRS_ARRAY)
        reload_config_state

        if [[ ${#MUSIC_DIRS_ARRAY[@]} -eq 0 ]]; then
             msg_warn "No directories configured"
        else
             local i=1
             for d in "${MUSIC_DIRS_ARRAY[@]}"; do
                 echo "   $i) $d"
                 ((i++))
             done
        fi

        echo ""
        echo "   [A] Add Directory"
        echo "   [R] Remove Directory"
        echo "   [q] Quit"

        read -rp "Action: " ACTION

        case "${ACTION,,}" in
            a|add)
                read -rp "Enter path to add: " NEW_PATH
                # Tilde expansion hack
                NEW_PATH="${NEW_PATH/#\~/$HOME}"
                if [[ -n "$NEW_PATH" ]]; then
                    config_add_dir "$NEW_PATH"
                fi
                ;;
            r|remove)
                if [[ ${#MUSIC_DIRS_ARRAY[@]} -eq 0 ]]; then
                    msg_warn "Nothing to remove."
                    continue
                fi

                read -rp "Enter NUMBER to remove: " IDX

                # Validate input is a number
                if [[ "$IDX" =~ ^[0-9]+$ ]]; then
                    # Adjust for 0-based array
                    local array_idx=$((IDX-1))

                    # Check bounds against MUSIC_DIRS_ARRAY length
                    if [[ "$array_idx" -ge 0 && "$array_idx" -lt "${#MUSIC_DIRS_ARRAY[@]}" ]]; then
                        local path_to_remove="${MUSIC_DIRS_ARRAY[$array_idx]}"
                        config_remove_dir "$path_to_remove"
                    else
                        msg_error "Invalid number ($IDX). Please pick 1-${#MUSIC_DIRS_ARRAY[@]}."
                    fi
                else
                    msg_error "Please enter a valid number."
                fi
                ;;
            q|quit)
                return
                ;;
            *)
                msg_warn "Invalid option. Press 'r' then the number."
                ;;
        esac
    done
}
