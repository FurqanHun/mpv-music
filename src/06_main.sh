# --- Global Variables (Initial values) ---
VIDEO_OK=false
CUSTOM_EXTS=""
MPV_ARGS=()
DIRECT_PLAY_TARGET="" # Will store the file path or URL if provided
CUSTOM_MUSIC_DIR="" # Will store a custom music directory if provided
declare -a GENRE_FILTERS=()
declare -a ARTIST_FILTERS=()
declare -a ALBUM_FILTERS=()
declare -a TITLE_FILTERS=()
CLI_FILTER_ACTIVE=false
PLAY_ALL=false
CLI_PLAYLIST_MODE=false


# --- Early Argument Handling ---
for arg in "$@"; do
  case "$arg" in
    -h|--help)
      show_help
      exit 0
      ;;
    -v|--version)
      echo "mpv-music v$VERSION"
      exit 0
      ;;
    --config|--config=*)
      if [[ "$arg" == "--config="* ]]; then
        EDITOR="${arg#--config=}"
      else
        if command -v nano &>/dev/null; then EDITOR="nano"
        elif command -v vi &>/dev/null; then EDITOR="vi"
        else msg_error "Neither nano nor vi found."; exit 1
        fi
      fi
      "$EDITOR" "$CONFIG_FILE"
      exit 0
      ;;
  esac
done

# --- Music Library Indexing Check ---
# Build EXT_FILTER here as it's needed by both build_music_index and update_music_index.
# Moved EXT_FILTER building here to ensure it's always available before indexing calls.
build_ext_filter
# EXT_FILTER is now always built here.

# Logic for handling custom directory vs. default index
INDEX_TO_USE="$MUSIC_INDEX_FILE" # Default to the main index
if [[ -n "$CUSTOM_MUSIC_DIR" ]]; then
    build_temp_index "$CUSTOM_MUSIC_DIR" temp_index_file
    INDEX_TO_USE="$temp_index_file"
else
    if [[ ! -f "$MUSIC_INDEX_FILE" ]]; then
        msg_info "Index file '$MUSIC_INDEX_FILE' not found. Building index from scratch."
        build_music_index
    fi
fi

# --- Argument Parsing ---
# Iterate through all arguments to identify direct play target, script options, or mpv flags
while [[ $# -gt 0 ]]; do
  case "$1" in
    -V|--verbose) VERBOSE=true; shift;;
    --debug) DEBUG=true; shift;;
    --video-ok) VIDEO_OK=true; shift;;
    --update) invoke_updater;;
    --ext=*) CUSTOM_EXTS="${1#--ext=}"; shift;;
    --refresh-index) build_ext_filter; update_music_index; exit 0;;
    --reindex) build_ext_filter; log_verbose "Forcing a complete rebuild of the music index."; build_music_index; exit 0;;
    -p|--play-all) PLAY_ALL=true; shift;;
    -l|--playlist) CLI_PLAYLIST_MODE=true; shift;;

    # --- NEW INTELLIGENT FILTER PARSING ---
    -g|--genre|-g=*|--genre=*)
        value=""
        if [[ "$1" == *=* ]]; then value="${1#*=}"; # Handle --genre=value
        elif [[ -n "${2:-}" && "$2" != -* ]]; then value="$2"; shift; fi # Handle --genre value

        if [[ -n "$value" ]]; then
            mapfile -t GENRE_FILTERS < <(echo "$value" | tr ',' '\n' | xargs -n 1)
        else # No value provided, so launch interactive picker
            selected_values=$(interactive_filter "genre" "🎶 Pick genre(s): ")
            mapfile -t GENRE_FILTERS <<< "$selected_values"
        fi
        CLI_FILTER_ACTIVE=true; shift;;

    -a|--artist|-a=*|--artist=*)
        value=""
        if [[ "$1" == *=* ]]; then value="${1#*=}";
        elif [[ -n "${2:-}" && "$2" != -* ]]; then value="$2"; shift; fi

        if [[ -n "$value" ]]; then
            mapfile -t ARTIST_FILTERS < <(echo "$value" | tr ',' '\n' | xargs -n 1)
        else
            selected_values=$(interactive_filter "artist" "🎤 Pick artist(s): ")
            mapfile -t ARTIST_FILTERS <<< "$selected_values"
        fi
        CLI_FILTER_ACTIVE=true; shift;;

    -b|--album|-b=*|--album=*)
        value=""
        if [[ "$1" == *=* ]]; then value="${1#*=}";
        elif [[ -n "${2:-}" && "$2" != -* ]]; then value="$2"; shift; fi

        if [[ -n "$value" ]]; then
            mapfile -t ALBUM_FILTERS < <(echo "$value" | tr ',' '\n' | xargs -n 1)
        else
            selected_values=$(interactive_filter "album" "💿 Pick album(s): ")
            mapfile -t ALBUM_FILTERS <<< "$selected_values"
        fi
        CLI_FILTER_ACTIVE=true; shift;;

    -t|--title|-t=*|--title=*)
        value=""
        if [[ "$1" == *=* ]]; then value="${1#*=}";
        elif [[ -n "${2:-}" && "$2" != -* ]]; then value="$2"; shift; fi

        if [[ -n "$value" ]]; then
            mapfile -t TITLE_FILTERS < <(echo "$value" | tr ',' '\n' | xargs -n 1)
        else
            run_track_mode
            exit 0
        fi
        CLI_FILTER_ACTIVE=true; shift;;

    *)
      if [[ -z "$DIRECT_PLAY_TARGET" && -z "$CUSTOM_MUSIC_DIR" ]]; then
        if [[ "$1" =~ ^https?://|^ftp://|^file://|^yt-dlp:// ]]; then DIRECT_PLAY_TARGET="$1"; shift; continue;
        elif [[ -f "$1" ]]; then DIRECT_PLAY_TARGET="$1"; shift; continue;
        elif [[ -d "$1" ]]; then CUSTOM_MUSIC_DIR="$1"; shift; continue;
        fi
      fi
      MPV_ARGS+=("$1"); shift;;
  esac
done

if [[ ${#MPV_ARGS[@]} -eq 0 ]]; then
  MPV_ARGS=("${MPV_DEFAULT_ARGS_ARRAY[@]}")
fi

if [[ -n "$DIRECT_PLAY_TARGET" ]]; then
  log_verbose "▶️ Playing: $DIRECT_PLAY_TARGET"
  mpv "${MPV_ARGS[@]}" "$DIRECT_PLAY_TARGET"
  exit 0
fi

# --- Handle --play-all without other filters ---
if [[ "$PLAY_ALL" == true && "$CLI_FILTER_ACTIVE" == false && "$CLI_PLAYLIST_MODE" == false ]]; then
    play_all_music
fi

# --- Handle --playlist flag ---
if [[ "$CLI_PLAYLIST_MODE" == true ]]; then
    run_playlist_mode
    exit 0
fi

# --- Handle CLI Filtering if active ---
if [[ "$CLI_FILTER_ACTIVE" == true ]]; then
    apply_filter() {
        local mode="$1" key="$2"
        shift 2
        local -a values=("$@")
        local jq_values_array
        jq_values_array=$(printf '%s\n' "${values[@]}" | jq -R . | jq -s .)

        local jq_filter
        if [[ "$mode" == "exact" ]]; then
            # Case-insensitive WHOLE WORD match (e.g., finds "Rock" in "Alternative Rock")
            jq_filter='($values | join("|")) as $regex | select(.[$key] | test("\\b(" + $regex + ")\\b"; "i"))'
        else # "contains"
            # Case-insensitive CONTAINS match
            jq_filter='($values | join("|")) as $regex | select(.[$key] | test($regex; "i"))'
        fi

        # Apply the filter to the .tracks array and reconstruct the main object
        jq --arg key "$key" --argjson values "$jq_values_array" \
            "{tracks: [.tracks[] | $jq_filter]}"
    }

    # --- Stage 1: Attempt an Exact (Whole Word) Match ---
    log_verbose "Trying smart match..."

    final_filtered_json=$(cat "$MUSIC_INDEX_FILE")
    if [[ ${#GENRE_FILTERS[@]} -gt 0 ]]; then final_filtered_json=$(echo "$final_filtered_json" | apply_filter "exact" "genre" "${GENRE_FILTERS[@]}"); fi
    if [[ ${#ARTIST_FILTERS[@]} -gt 0 ]]; then final_filtered_json=$(echo "$final_filtered_json" | apply_filter "exact" "artist" "${ARTIST_FILTERS[@]}"); fi
    if [[ ${#ALBUM_FILTERS[@]} -gt 0 ]]; then final_filtered_json=$(echo "$final_filtered_json" | apply_filter "exact" "album" "${ALBUM_FILTERS[@]}"); fi
    if [[ ${#TITLE_FILTERS[@]} -gt 0 ]]; then final_filtered_json=$(echo "$final_filtered_json" | apply_filter "exact" "title" "${TITLE_FILTERS[@]}"); fi

    track_count=$(echo "$final_filtered_json" | jq '.tracks | length')

    if [[ "$track_count" -eq 0 ]]; then
        log_verbose "No smart match found. Searching for partial matches..."
        active_filter_key=""
        active_filter_values=()

        if [[ ${#ARTIST_FILTERS[@]} -gt 0 ]]; then
            active_filter_key="artist"; active_filter_values=("${ARTIST_FILTERS[@]}")
        elif [[ ${#GENRE_FILTERS[@]} -gt 0 ]]; then
            active_filter_key="genre"; active_filter_values=("${GENRE_FILTERS[@]}")
        elif [[ ${#ALBUM_FILTERS[@]} -gt 0 ]]; then
            active_filter_key="album"; active_filter_values=("${ALBUM_FILTERS[@]}")
        elif [[ ${#TITLE_FILTERS[@]} -gt 0 ]]; then
            active_filter_key="title"; active_filter_values=("${TITLE_FILTERS[@]}")
        fi

        if [[ -n "$active_filter_key" ]]; then
            contains_match_json=$(cat "$MUSIC_INDEX_FILE" | apply_filter "contains" "$active_filter_key" "${active_filter_values[@]}")
            mapfile -t clarification_options < <(echo "$contains_match_json" | jq -r --arg key "$active_filter_key" '.tracks[].[$key]' | sort -fu)

            if [[ ${#clarification_options[@]} -eq 1 ]]; then
                log_verbose "Found one likely match: '${clarification_options[0]}'"
                final_filtered_json=$(cat "$MUSIC_INDEX_FILE" | apply_filter "exact" "$active_filter_key" "${clarification_options[0]}")
            elif [[ ${#clarification_options[@]} -gt 1 ]]; then
                mapfile -t clarified_values < <(printf '%s\n' "${clarification_options[@]}" | fzf --multi --prompt="Which ${active_filter_key} did you mean? ")
                if [[ ${#clarified_values[@]} -gt 0 ]]; then
                    final_filtered_json=$(cat "$MUSIC_INDEX_FILE" | apply_filter "exact" "$active_filter_key" "${clarified_values[@]}")
                fi
            fi
        fi
    fi

    # --- Stage 3: Play Results or Ask for Next Action ---
    track_count=$(echo "$final_filtered_json" | jq '.tracks | length')

    if [[ "$track_count" -eq 0 ]]; then
        msg_error "No matching tracks found."
        exit 1
    fi

    msg_success "Found $track_count matching track(s)."

    if [[ "$PLAY_ALL" == true || "$track_count" -eq 1 ]]; then
        # If --play-all is used OR if there's only one result, play directly
        mapfile -t FILES < <(echo "$final_filtered_json" | jq -r '.tracks[].path')
        log_verbose "🎶 Playing all ${#FILES[@]} track(s)..."
        mpv "${MPV_ARGS[@]}" "${FILES[@]}"
    else
        # Otherwise, ask the user what to do
        echo "What's next?"
        echo "1) Play all $track_count tracks"
        echo "2) Select individual tracks from this list"
        read -rp "Enter choice [1/2]: " PLAY_CHOICE </dev/tty

        case "$PLAY_CHOICE" in
            1)
                mapfile -t FILES < <(echo "$final_filtered_json" | jq -r '.tracks[].path')
                log_verbose "🎶 Loading all ${#FILES[@]} track(s)..."
                mpv "${MPV_ARGS[@]}" "${FILES[@]}"
                ;;
            2) # FIXED DELIMITER: Using @tsv instead of pipes
                create_temp_file temp_track_list
                echo "$final_filtered_json" | jq -r '.tracks[] |
                        [
                            (if .media_type == "video" then "🎬 " else "🎵 " end) + (.title // "[NO TITLE]"),
                            .title // "[NO TITLE]",
                            .artist // "[NO ARTIST]",
                            .album // "[NO ALBUM]",
                            .genre // "[NO GENRE]",
                            .media_type // "UNKNOWN",
                            .path
                        ] | @tsv' > "$temp_track_list"

                SELECTED=$(cat "$temp_track_list" | fzf --multi \
                    --prompt="🎵 Pick your filtered tracks (TAB to multi-select): " \
                    --delimiter="\t" \
                    --with-nth=1 \
                    --preview='echo -e "\033[1;36mTitle:\033[0m {2}\n\033[1;33mArtist:\033[0m {3}\n\033[1;32mAlbum:\033[0m {4}\n\033[1;35mGenre:\033[0m {5}\n\033[1;34mType:\033[0m {6}"' \
                    --preview-window=top:5 | awk -F'\t' '{print $NF}')

                mapfile -t FILES <<< "$SELECTED"
                [[ ${#FILES[@]} -eq 0 ]] && msg_warn "No tracks picked." && exit 1
                log_verbose "🎶 Selected ${#FILES[@]} track(s)."
                mpv "${MPV_ARGS[@]}" "${FILES[@]}"
                ;;
            *) msg_error "Invalid choice. Exiting."; exit 1;;
            esac
        fi
        exit 0
    fi

# --- Interactive Mode Selection ---
echo -e "${CYAN}🎧 Pick mode:${NC}"
echo "1) Play entire Directory(s)"
echo "2) Pick individual tracks"
echo "3) Play a saved playlist"
echo "4) Filter by Tag..."
echo "5) Play All Music"
# read -rp "Enter choice [1/2]: " MODE
# Use -t with a very long timeout instead of blocking read
# This allows the SIGINT to be caught by our trap
read -t 31536000 -rp "Enter choice [1/2/3/4/5]: " MODE || {
    echo -e "\nRead interrupted. Exiting."
    exit 1
}

if [[ "$MODE" == "1" ]]; then
    run_dir_mode

elif [[ "$MODE" == "2" ]]; then
    run_track_mode

elif [[ "$MODE" == "3" ]]; then
    run_playlist_mode

elif [[ "$MODE" == "4" ]]; then
    run_tag_mode

elif [[ "$MODE" == "5" ]]; then
    run_play_all_mode

else
  msg_error "Invalid input."
  exit 1
fi
