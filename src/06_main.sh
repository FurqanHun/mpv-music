# --- Global Variables (Initial values) ---
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

# Deferred interactive filter flags
GENRE_INTERACTIVE=false
ARTIST_INTERACTIVE=false
ALBUM_INTERACTIVE=false
TITLE_INTERACTIVE=false

# --- Helper Function: Handle Direct Play ---
handle_direct_play() {
  local target="$1"
  local has_url=false
  local has_youtube=false
  local ytdl_opts=""

  # Scan primary target
  if [[ "$target" =~ ^https?://|^ftp://|^yt-dlp:// ]]; then
      has_url=true
      if [[ "$target" =~ (youtube\.com|youtu\.be) ]]; then has_youtube=true; fi
      # Force playlist if 'list=' is present
      # "yes-playlist=" is required by mpv for boolean flags
      if [[ "$target" == *"list="* ]]; then ytdl_opts+="yes-playlist=,"; fi
  fi

  # Scan other args in case user passed multiple URLs
  for arg in "${MPV_ARGS[@]}"; do
      if [[ "$arg" =~ ^https?://|^ftp://|^yt-dlp:// ]]; then
          has_url=true
          if [[ "$arg" =~ (youtube\.com|youtu\.be) ]]; then has_youtube=true; fi
          if [[ "$arg" == *"list="* ]]; then ytdl_opts+="yes-playlist=,"; fi
      fi
  done

  if [[ "$has_url" == true ]]; then
      msg_info "Resolving stream..."
      MPV_ARGS+=("--msg-level=ytdl_hook=info")
  else
      log_verbose "Playing local file: $target"
  fi

  # --- YouTube JS Runtime Auto-Config ---
  # yt-dlp now mandates a JS runtime for YouTube.
  if [[ "$has_youtube" == true ]]; then
      if [[ "$VIDEO_OK" == false ]]; then
          MPV_ARGS+=("--ytdl-format=bestaudio/best")
      fi

      log_verbose "YouTube URL detected. Checking for JS runtimes..."

      if command -v deno &>/dev/null; then
         log_verbose "Deno found (yt-dlp default)."

      elif command -v node &>/dev/null; then
         log_verbose "Using Node.js fallback."
         ytdl_opts+="js-runtimes=node,"

      elif command -v qjs &>/dev/null || command -v quickjs &>/dev/null; then
         log_verbose "Using QuickJS fallback."
         ytdl_opts+="js-runtimes=quickjs,"

      elif command -v bun &>/dev/null; then
         log_verbose "Using Bun fallback."
         ytdl_opts+="js-runtimes=bun,"

      else
         echo ""
         msg_warn "No supported JS runtime found (Deno, Node, QuickJS, Bun)!"
         msg_warn "YouTube playback requires a JS runtime to bypass new anti-bot protections."
         msg_warn "Playback will likely fail with HTTP 403 Forbidden."
         msg_note "Please install 'deno' (recommended) or 'nodejs'."
         echo ""
      fi
  fi

  # Apply accumulated ytdl options (JS runtime + Playlist fix)
  if [[ -n "$ytdl_opts" ]]; then
      log_verbose "Applying yt-dlp options: ${ytdl_opts%,}"
      # Remove trailing comma
      MPV_ARGS+=("--ytdl-raw-options=${ytdl_opts%,}")
  fi

  log_verbose "Executing MPV with target: $target"
  log_debug "MPV Args: ${MPV_ARGS[*]}"
  exit 0
}

# --- Early Argument Handling ---
for arg in "$@"; do
  case "$arg" in
    -h|--help)
      show_help
      exit 0
      ;;
  esac
done

# --- Music Library Indexing Check ---
# Build EXT_FILTER here as it's needed by both build_music_index and update_music_index.
# Moved EXT_FILTER building here to ensure it's always available before indexing calls.
build_ext_filter
# EXT_FILTER is now always built here.

# This prevents double-indexing when the file is missing AND --reindex is passed.
SKIP_AUTO_INDEX=false
for arg in "$@"; do
    if [[ "$arg" == "--reindex" || "$arg" == "--refresh-index" ]]; then
        SKIP_AUTO_INDEX=true
        break
    fi
done

INDEX_TO_USE="$MUSIC_INDEX_FILE" # Default to the main index

# --- Argument Parsing ---
# Iterate through all arguments to identify direct play target, script options, or mpv flags
while [[ $# -gt 0 ]]; do
  case "$1" in
    -V|--verbose) VERBOSE=true; shift;;
    --debug) DEBUG=true; shift;;
    --video-ok) VIDEO_OK=true; shift;;
    --ext=*) CUSTOM_EXTS="${1#--ext=}"; shift;;
    --serial) SERIAL_MODE=true; shift;;
    --refresh-index) ensure_index_integrity; build_ext_filter; update_music_index; exit 0;;
    --reindex) build_ext_filter; log_verbose "Forcing a complete rebuild of the music index."; build_music_index; exit 0;;
    --vol=*|--volume=*) VOLUME="${1#*=}"; shift;;
    --vol|--volume)
        if [[ -n "${2:-}" && "$2" != -* ]]; then
            VOLUME="$2"
            shift 2
        else
            msg_error "Missing value for --volume."
            exit 1
        fi
        ;;
    --add-dir)
        # Check if we have at least one valid argument
        if [[ -z "${2:-}" || "${2:-}" == -* ]]; then
            msg_error "Missing argument for --add-dir."
            exit 1
        fi

        # Loop to consume all subsequent arguments until a flag is found
        shift # Skip the --add-dir flag itself
        while [[ $# -gt 0 && "$1" != -* ]]; do
            TARGET="${1/#\~/$HOME}"
            config_add_dir "$TARGET" "true"
            changes_made=true
            shift
        done
        if [[ "$changes_made" == true ]]; then
            log_verbose "Applying batch changes..."
            update_music_index
        fi
        exit 0
        ;;

    --remove-dir)
        if [[ -z "${2:-}" || "${2:-}" == -* ]]; then
            msg_error "Missing argument for --remove-dir."
            exit 1
        fi

        shift
        while [[ $# -gt 0 && "$1" != -* ]]; do
            TARGET="${1/#\~/$HOME}"
            config_remove_dir "$TARGET" "true"
            changes_made=true
            shift
        done
        if [[ "$changes_made" == true ]]; then
            log_verbose "Applying batch changes..."
            update_music_index
        fi
        exit 0
        ;;

    --manage-dirs)
        run_manage_dirs_mode
        exit 0
        ;;
    -p|--play-all) PLAY_ALL=true; shift;;
    -l|--playlist) CLI_PLAYLIST_MODE=true; shift;;

    # --- INTELLIGENT FILTER PARSING (deferred interactive) ---
    -g|--genre|-g=*|--genre=*)
        value=""
        if [[ "$1" == *=* ]]; then value="${1#*=}";
        elif [[ -n "${2:-}" && "$2" != -* ]]; then value="$2"; shift; fi

        if [[ -n "$value" ]]; then
            mapfile -t GENRE_FILTERS < <(echo "$value" | tr ',' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | grep -v '^$')
        else
            GENRE_INTERACTIVE=true
        fi
        CLI_FILTER_ACTIVE=true; shift;;

    -a|--artist|-a=*|--artist=*)
        value=""
        if [[ "$1" == *=* ]]; then value="${1#*=}";
        elif [[ -n "${2:-}" && "$2" != -* ]]; then value="$2"; shift; fi

        if [[ -n "$value" ]]; then
            mapfile -t ARTIST_FILTERS < <(echo "$value" | tr ',' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | grep -v '^$')
        else
            ARTIST_INTERACTIVE=true
        fi
        CLI_FILTER_ACTIVE=true; shift;;

    -b|--album|-b=*|--album=*)
        value=""
        if [[ "$1" == *=* ]]; then value="${1#*=}";
        elif [[ -n "${2:-}" && "$2" != -* ]]; then value="$2"; shift; fi

        if [[ -n "$value" ]]; then
            mapfile -t ALBUM_FILTERS < <(echo "$value" | tr ',' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | grep -v '^$')
        else
            ALBUM_INTERACTIVE=true
        fi
        CLI_FILTER_ACTIVE=true; shift;;

    -t|--title|-t=*|--title=*)
        value=""
        if [[ "$1" == *=* ]]; then value="${1#*=}";
        elif [[ -n "${2:-}" && "$2" != -* ]]; then value="$2"; shift; fi

        if [[ -n "$value" ]]; then
            mapfile -t TITLE_FILTERS < <(echo "$value" | tr ',' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | grep -v '^$')
        else
            TITLE_INTERACTIVE=true
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

rotate_log
ensure_index_integrity

# Logic for handling custom directory vs. default index

if [[ -n "$CUSTOM_MUSIC_DIR" ]]; then
    if ! build_temp_index "$CUSTOM_MUSIC_DIR" temp_index_file; then
        exit 1
    fi
    INDEX_TO_USE="$temp_index_file"
elif [[ "$SKIP_AUTO_INDEX" == false ]]; then
    # Only auto-build if we aren't about to do it manually in the arg parser
    if [[ ! -f "$MUSIC_INDEX_FILE" ]]; then
        msg_info "Index file '$MUSIC_INDEX_FILE' not found. Building index from scratch."
        build_music_index
    fi
fi

# --- Run deferred interactive filters (now INDEX_TO_USE is correct) ---
if [[ "$GENRE_INTERACTIVE" == true ]]; then
    selected_values=$(interactive_filter "genre" "🎶 Pick genre(s): ")
    mapfile -t GENRE_FILTERS <<< "$selected_values"
fi
if [[ "$ARTIST_INTERACTIVE" == true ]]; then
    selected_values=$(interactive_filter "artist" "🎤 Pick artist(s): ")
    mapfile -t ARTIST_FILTERS <<< "$selected_values"
fi
if [[ "$ALBUM_INTERACTIVE" == true ]]; then
    selected_values=$(interactive_filter "album" "💿 Pick album(s): ")
    mapfile -t ALBUM_FILTERS <<< "$selected_values"
fi
if [[ "$TITLE_INTERACTIVE" == true ]]; then
    run_track_mode
    exit 0
fi

# MERGE DEFAULTS: Always prepend defaults so user args don't wipe them out
MPV_ARGS=("${MPV_DEFAULT_ARGS_ARRAY[@]}" "${MPV_ARGS[@]}")
MPV_ARGS+=("--volume=$VOLUME")

if [[ -n "$DIRECT_PLAY_TARGET" ]]; then
  handle_direct_play "$DIRECT_PLAY_TARGET"
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

    # apply_cli_filter now works on FILES, not strings.
    # it reads $file_to_filter, applies jq logic, and overwrites the file with results.
    apply_cli_filter() {
        local file_to_filter="$1"
        local mode="$2"
        local key="$3"
        shift 3
        local -a values=("$@")
        local -a escaped_values=()

        # Escape special regex characters: . ^ $ * + ? ( ) [ ] { } | \
        for val in "${values[@]}"; do
            escaped_values+=("$(echo "$val" | sed 's/\\/\\\\/g; s/[][(){}^$*+?.|]/\\&/g')")
        done

        local jq_values_array
        jq_values_array=$(printf '%s\n' "${escaped_values[@]}" | jq -R . | jq -s .)

        local jq_filter
        if [[ "$mode" == "exact" ]]; then
            # Case-insensitive WHOLE WORD match
            jq_filter='($values | join("|")) as $regex | select(.[$key] | test("\\b(" + $regex + ")\\b"; "i"))'
        else # "contains"
            # Case-insensitive CONTAINS match
            jq_filter='($values | join("|")) as $regex | select(.[$key] | test($regex; "i"))'
        fi

        log_debug "Applying Filter ($mode) on '$key'"
        log_debug "Values: ${values[*]}"
        log_debug "JQ Query: $jq_filter"

        local temp_out
        create_temp_file temp_out

        # Apply filter stream -> temp
        jq -c --arg key "$key" --argjson values "$jq_values_array" \
            "$jq_filter" "$file_to_filter" > "$temp_out"

        # Move back to original file path (update in place)
        mv "$temp_out" "$file_to_filter"
    }

    # --- Stage 1: Attempt an Exact (Whole Word) Match ---
    log_verbose "Trying smart match..."

    # Create a working copy of the index so we don't destroy the original
    create_temp_file working_subset
    cp "$INDEX_TO_USE" "$working_subset"

    if [[ ${#GENRE_FILTERS[@]} -gt 0 ]]; then apply_cli_filter "$working_subset" "exact" "genre" "${GENRE_FILTERS[@]}"; fi
    if [[ ${#ARTIST_FILTERS[@]} -gt 0 ]]; then apply_cli_filter "$working_subset" "exact" "artist" "${ARTIST_FILTERS[@]}"; fi
    if [[ ${#ALBUM_FILTERS[@]} -gt 0 ]]; then apply_cli_filter "$working_subset" "exact" "album" "${ALBUM_FILTERS[@]}"; fi
    if [[ ${#TITLE_FILTERS[@]} -gt 0 ]]; then apply_cli_filter "$working_subset" "exact" "title" "${TITLE_FILTERS[@]}"; fi

    # wc -l for counting lines
    track_count=$(wc -l < "$working_subset")

    if [[ "$track_count" -eq 0 ]]; then
        log_verbose "No smart match found. Searching for partial matches..."

        # Reset working set to full index
        cp "$INDEX_TO_USE" "$working_subset"

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
            # Apply "contains" filter to the working subset
            apply_cli_filter "$working_subset" "contains" "$active_filter_key" "${active_filter_values[@]}"

            # Check for ambiguity by looking at the filtered results
            mapfile -t clarification_options < <(jq -r --arg key "$active_filter_key" '.[$key]' "$working_subset" | sort -fu)

            if [[ ${#clarification_options[@]} -eq 1 ]]; then
                log_verbose "Found one likely match: '${clarification_options[0]}'"
                # We already filtered by "contains", but let's be strict if needed.
                # the "contains" result is already in working_subset.
                cp "$INDEX_TO_USE" "$working_subset"
                apply_cli_filter "$working_subset" "exact" "$active_filter_key" "${clarification_options[0]}"

            elif [[ ${#clarification_options[@]} -gt 1 ]]; then
                clarified_str=$(printf '%s\n' "${clarification_options[@]}" | fzf --multi --prompt="Which ${active_filter_key} did you mean? ")
                [[ -z "$clarified_str" ]] && msg_warn "No selection made." && exit 1
                mapfile -t clarified_values <<< "$clarified_str"

                # Reset and re-apply exact on specific value
                cp "$INDEX_TO_USE" "$working_subset"
                apply_cli_filter "$working_subset" "exact" "$active_filter_key" "${clarified_values[@]}"
            fi
        fi
    fi

    # --- Stage 3: Play Results or Ask for Next Action ---
    track_count=$(wc -l < "$working_subset")

    if [[ "$track_count" -eq 0 ]]; then
        msg_error "No matching tracks found."
        exit 1
    fi

    msg_success "Found $track_count matching track(s)."

    if [[ "$PLAY_ALL" == true || "$track_count" -eq 1 ]]; then
        # If --play-all is used OR if there's only one result, play directly
        msg_success "Playing $track_count track(s)..."
        jq -r '.path' "$working_subset" | mpv "${MPV_ARGS[@]}" --playlist=-
    else
        # Otherwise, ask the user what to do
        echo "What's next?"
        echo "1) Play all $track_count tracks"
        echo "2) Select individual tracks from this list"
        read -rp "Enter choice [1/2]: " PLAY_CHOICE </dev/tty

        case "$PLAY_CHOICE" in
            1)
                msg_success "Streaming all tracks to MPV..."
                jq -r '.path' "$working_subset" | mpv "${MPV_ARGS[@]}" --playlist=-
                ;;
            2)
                create_temp_file temp_track_list

                # we add line no for security
                jq -r '
                        [
                            (if .media_type == "video" then "🎬 " else "🎵 " end) + (.title // "[NO TITLE]"),
                            .title // "[NO TITLE]",
                            .artist // "[NO ARTIST]",
                            .album // "[NO ALBUM]",
                            .genre // "[NO GENRE]",
                            .media_type // "UNKNOWN",
                            .path
                        ] | @tsv' "$working_subset" | nl -w1 -s$'\t' > "$temp_track_list"

                SELECTED=$(cat "$temp_track_list" | fzf --multi \
                    --prompt="🎵 Pick filtered tracks: " \
                    --delimiter="\t" \
                    --with-nth=2 \
                    --preview="
                        RAW=\$(sed -n {1}p '$temp_track_list');
                        # Capture visual col in dummy var so others align correctly
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

                log_verbose "🎶 Selected ${#FILES[@]} track(s)."
                mpv "${MPV_ARGS[@]}" "${FILES[@]}"
                ;;
            *) msg_error "Invalid choice. Exiting."; exit 1;;
            esac
        fi
        exit 0
    fi

# --- Interactive Mode Selection ---
while true; do

    echo -e "${CYAN}🎧 Pick mode:${NC}"
    echo "1) Play entire Directory(s)"
    echo "2) Pick individual tracks"
    echo "3) Play a saved playlist"
    echo "4) Filter by Tag..."
    echo "5) Play All Music"
    echo "6) Play URL (Stream)"
    echo "7) Manage Directories"
    echo "q) Quit"

    # Read with prompt. Supports ESC+Enter to quit.
    read -rp "Enter choice [1-7/q]: " MODE || exit 0

    case "$MODE" in
        1) run_dir_mode; exit 0 ;;
        2) run_track_mode; exit 0 ;;
        3) run_playlist_mode; exit 0 ;;
        4) run_tag_mode; exit 0 ;;
        5) run_play_all_mode; exit 0 ;;
        6)
            echo ""
            echo "Paste URL(s) (separated by space):"
            read -rp "> " -a USER_URLS
            if [[ ${#USER_URLS[@]} -gt 0 ]]; then
                TARGET="${USER_URLS[0]}"
                [[ ${#USER_URLS[@]} -gt 1 ]] && MPV_ARGS+=("${USER_URLS[@]:1}")
                handle_direct_play "$TARGET"
            else
                msg_error "No URL provided."
            fi
            exit 0
            ;;
        q|Q|$'\e')
            echo "Bye Bye!"
            exit 0
            ;;
        7) run_manage_dirs_mode; exit 0;;
        *)
            msg_warn "Invalid input. Please pick 1-6 or q."
            sleep 0.5
            echo ""
            ;;
    esac
done
