# mpv-music

[![version](https://img.shields.io/github/v/release/FurqanHun/mpv-music?include_prereleases&color=blue)](https://github.com/FurqanHun/mpv-music/releases)
[![mpv-music build status](https://github.com/FurqanHun/mpv-music/actions/workflows/release.yml/badge.svg)](https://github.com/FurqanHun/mpv-music/actions/workflows/release.yml)

**mpv-music** is a blazing-fast, terminal-native music player and library browser. Originally a Bash hybrid, it has been completely rewritten in Rust for maximum performance, safety, and a seamless TUI experience.

It indexes your music collection into a lightning-fast library, providing fuzzy searching (via `skim`), metadata-rich previews, and deep integration with `mpv` for high-quality playback.

> [!NOTE]
> **This is the documentation for the Rust-native rewrite (v0.24+).**
> mpv-music has migrated from the legacy Bash hybrid system to a unified Rust core.
>
> If you are looking for the archived **Bash-based version**, see:
> [mpv-music-sh-archive](https://github.com/FurqanHun/mpv-music/tree/mpv-music-sh-archive)

---

## Table of Contents

- [Key changes in Rust Rewrite](#key-changes-in-rust-rewrite)
- [Features](#features)
- [Dependencies](#dependencies)
- [Installation](#installation)
- [Usage](#usage)
- [Indexing](#indexing)
- [Configuration](#configuration)
- [Development](#development)
- [License](#license)
- [GenAI Disclosure](#genai-disclosure)

---

## Key changes in Rust Rewrite

- **Single Binary:** No more managing a Bash script + a separate indexer binary. Everything is now one efficient executable.
- **Native TUI:** Replaced external fzf calls with an integrated Rust TUI based on skim, allowing for deeper UI customization and better performance.
- **Static Linking:** Linux releases are now built with musl, making them "portable"—they run on almost any distribution without worrying about GLIBC versions.
- **Faster Scans:** The indexing engine is now part of the main app, leveraging Rust's multi-threading to scan thousands of files in milliseconds.
- **Leaner Dependencies:** Significantly reduced the number of external tools required to run.

## Features

- **Lightning-Fast Indexing:** Automatically scans your music directories and caches metadata (Artist, Album, Title, Genre) into a JSONL index.
- **Advanced Fuzzy Search:** Instant, interactive searching through your entire library.
- **Self-Healing Index:** Automatically validates index integrity on startup. It detects corruption (e.g., from power loss), surgically repairs broken lines to save your library, or triggers a smart rebuild to prevent crashes. In a blink of an eye.
* **Rich Metadata Previews:** View song title, artist, album, and genre directly in the skim preview window.
* **Interactive Selection with Multiple Modes:**
  * **Directory Mode:** Navigate folders with clean names instead of full paths.
  * **Track Mode:** Fuzzy-search individual tracks with metadata previews.
  * **Playlist Mode:** Find and play your saved `.m3u` or `.pls` playlists.
  * **Tag Filter Mode:** Drill down by genre, artist, album, or title interactively.
  * **Play All:** Instantly play your entire indexed library.
  * **Search & Stream URL:** Search YouTube or stream URLs directly from the menu.
  * **Settings:** Manage mpv-music settings directly from the menu.
* **Direct File/URL Playback:** Instantly play local audio/video files or URLs (YouTube, streams) without going through the menu.
* **Custom Directory Support:** Pass a folder path to browse and filter only that directory instead of your full library.
* **CLI Filtering:** Use flags like `--genre`, `--artist`, `--album`, `--title` for direct filtering. Pass a value or omit it to open an interactive picker.
* **Smart Matching:** CLI filters attempt exact matches first, then fall back to partial matches with disambiguation.
* **Configurable File Types:** Support for both audio and video extensions, easily tweakable.
* **Custom MPV Flags:** Pass mpv flags directly or set defaults in the config.
* **Video Toggle:** `--video-ok` lets you include videos in your library scans.
* **YouTube Auto-Config:** Automatically detects JS runtimes (Deno, Node, QuickJS, Bun) for yt-dlp YouTube playback.
* **Enhanced Logging:** Verbose/debug modes with log rotation and configurable log file size.

---

## Dependencies

#### Required:
- **mpv** - https://mpv.io

#### Optional (but recommended):
- **yt-dlp** - for playing URLs.
  https://github.com/yt-dlp/yt-dlp
- **JS Runtime** - for YouTube playback (Deno, Node.js, QuickJS, or Bun). Deno is recommended.

> [!NOTE]
> YouTube playback now requires a JS runtime to bypass anti-bot protections.
> `mpv-music` automatically detects and configures **Deno**, **Node**, **QuickJS**, or **Bun** for `yt-dlp`.
>
> If you installed yt-dlp via a package manager (apt/dnf/pacman) instead of the official GitHub binary, it might be missing components. Try enabling the remote fix in your config (`mpv-music --config`):
> `ytdlp_ejs_remote_github = true`

---

## Installation

### Supported Systems

* **Linux:** Native. The script is built and tested primarily for Linux (GNU tools).
* **WSL (Windows Subsystem for Linux):** Fully Supported. This is the recommended way to run it on Windows.
* **macOS / BSD:** It should work fine on macOS and BSD systems (haven't tested it, please do... any feedback is appreciated).
* **Windows (Native/Git Bash):** Not Supported. As one of the libraries used `skim` (which as of version 2.0.2) doesn't support windows. According to [Issue #293](https://github.com/lotabout/skim/issues/293) They're working on it. Or at least will be a priority after tthe ratatui rewrite. Until then `mpv-music` will not work on Windows.

> [!Tip]
> You can use WSL (Windows Subsystem for Linux) to run `mpv-music` on Windows.

### Pre-built Binaries (Recommended)

1. Download the latest binary for your architecture from the [Releases](https://github.com/FurqanHun/mpv-music/releases) page.
2. Make it executable and move it to your path:
    ```bash
    chmod +x mpv-music
    mv mpv-music ~/.local/bin/
    ```
or Alternatively, you can use the followiing command and let the script handle the process

```bash
curl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.sh | bash -s -- --dev
```
    
### From Source
Requires Rust **1.93.0+**.
```bash
git clone [https://github.com/FurqanHun/mpv-music.git](https://github.com/FurqanHun/mpv-music.git)
cd mpv-music
cargo build --release
# Binary will be at target/release/mpv-music
```

### First run (setup):
```bash
mpv-music
```

> [!IMPORTANT]
> Running `mpv-music` for the first time will automatically index `$HOME/Music`.
> It is **recommended** that you first run `mpv-music --manage-dirs` to customize music directories before indexing (unless you only keep your music in `$HOME/Music`). And if your music is on an HDD, you may want to run `--serial` or set `SERIAL_MODE=true` in your config, using `mpv-music --config`.

That creates:

- Config: `~/.config/mpv-music/config.toml`

- Index: `~/.local/share/mpv-music/music_index.jsonl`

- Logs: `~/.local/share/mpv-music/mpv-music.log`

The project now respects XDG standards. and only uses config folder to dump all as a fallback. And by `directories` library used does support config/data dirs in windows/mac.

---

## Usage

```bash
mpv-music [PATH_OR_URL_OR_DIR] [OPTIONS]
mpv-music [FILTER_FLAGS] [--play-all]
```

### Arguments:

* **No args:** Runs interactive selection on your configured music directories.
* **File or URL:** Plays it instantly.
* **Folder path:** Runs interactive search using just that folder.

### Options:

| Option | Description |
| :--- | :--- |
| `[TARGET]` | Directly play a file, directory, or URL |
| `-r`, `--refresh-index` | Update index (incremental scan). Detects new/changed files. |
| `--reindex` | Force a full re-scan of the library. |
| `-u`, `--update` | Update the application. |
| `--add-dir <PATH>...` | Add directory (e.g. `--add-dir /music /other`). |
| `--remove-dir <PATH>...` | Remove directory (aliases: `--rm-dir`). |
| `--manage-dirs` | Open the Interactive Directory Manager. |
| `-c`, `--config [<EDITOR>]` | Edit config file. |
| `--remove-config` | Delete config file (Reset) (aliases: `--rm-conf`). |
| `--log [<PAGER>]` | View logs. |
| `--remove-log` | Delete log file (aliases: `--rm-log`). |
| `-p`, `--play-all` | Play all tracks immediately. |
| `-l`, `--playlist [<VAL>]` | Open Playlist Mode. Opens picker if no value given. |
| `--video-ok` | Allow video files. |
| `--loop [<LOOP_ARG>]` | Enable looping (`inf`, `no`, `track`, or a NUMBER). |
| `--no-loop` | Disable all looping. |
| `--repeat` | Loop the current track (Repeat One). |
| `-e`, `--ext <EXT_LIST>` | Override allowed extensions (e.g. `-e mp3,flac`). |
| `-g`, `--genre [<GENRE>]` | Filter by Genre (e.g. `-g 'Pop,Rock'`). |
| `-a`, `--artist [<ARTIST>]` | Filter by Artist (e.g. `-a 'ado,gentle'`). |
| `-b`, `--album [<ALBUM>]` | Filter by Album. |
| `-t`, `--title [<TITLE>]` | Filter by Title (Partial). Opens Track Mode if no value given. |
| `-v`, `--verbose` | Display Verbose Information. |
| `-d`, `--debug` | Debug mode. |
| `--volume <VOLUME>` | Set volume (0-100). |
| `-s`, `--shuffle` | Shuffle. |
| `--no-shuffle` | No Shuffle. |
| `--serial` | Force serial (single-threaded) processing. |
| `--search [<SEARCH>]` | Search YouTube directly (aliases: `--yt`). |
| `-h`, `--help` | Print help. |
| `-V`, `--version` | Print version. |

Any mpv flag also works: `--no-video`, `--volume=50`, `--shuffle`, etc.

Instead of using the log rotation method now log is overwritten each time the program is run. And you can turn the logging off by setting `enable_file_logging = false` in your config.

### Examples:

```bash
mpv-music                              # full interactive menu
mpv-music /path/to/music               # interactive in a specific folder
mpv-music ~/Music/track.flac           # plays file instantly
mpv-music "https://youtube.com/watch..." # plays URL instantly
mpv-music /path/to/folder -a           # pick artist from that folder only
mpv-music --genre="Rock" --play-all    # play all rock tracks
mpv-music --artist="Ado"               # fuzzy search by artist
mpv-music -p -a ado                     # play all tracks by Ado
mpv-music -g -a "Daft Punk" -p         # pick genre, then play all Daft Punk
mpv-music --volume=50 --shuffle        # custom mpv flags
mpv-music --reindex                    # rebuild the index from scratch
mpv-music --debug                      # run with full logging enabled
mpv-music --verbose                    # prints verbose messages
mpv-music --add-dir /path/to/music /path/to/music2 # Add multiple directories
mpv-music --remove-dir /path/to/music /path/to/music2 # Remove multiple directories
mpv-music --manage-dirs                  # Manage directories
```

---

## Indexing

Your music library is indexed to:

- Where ever our sysetm has defined program data should go, for modern linux systems it should be at `~/.local/share/mpv-music/music_index.jsonl`

### Why?

Searching the filesystem with find every time is slow, especially if you have a large music collection. So mpv-music caches an index using JSONL (JSON Lines) for:

- Fast filtering
- Instant append updates
- Metadata previews
- Offline-friendly behavior

### Maintaining:

* `--reindex` - Full rebuild
* `--refresh-index` or `-r` - Smart update (only processes new/modified files)

> [!TIP]
> Run these flags alongside `--video-ok` to include video files in the index.
> Example: `mpv-music --video-ok --reindex`
> Or you can set `video_ok = true` in your config file.

---

## Configuration

If a config file does not exist, mpv-music will create one at startup. To customize the behavior:

```
mpv-music --config
```
**Options:**

```toml
# --- General Playback ---
shuffle = true
loop_mode = "inf"  # Options: "playlist" (same as inf), "track", "no", "inf", "5" (number of loops)
volume = 100

# --- Library Management ---
music_dirs = [
    "/home/user/Music",
    "/mnt/storage/songs",
]
video_ok = false    # Set to true to include video files in the index
serial_mode = false # Set to true to force single-threaded scanning (better for HDDs)

# --- YT-DLP / Networking ---
# Set to true if you installed yt-dlp via package manager (apt/pacman). 
# Keep false if you downloaded the binary directly from GitHub.
ytdlp_ejs_remote_github = false 
ytdlp_useragent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/114.0"

# --- Logging ---
# If true, INFO/WARN logs are saved to file. 
# If false, logs are only shown on screen when running with --verbose or --debug.
enable_file_logging = true

# --- File Extensions ---
audio_exts = [
    "mp3",
    "flac",
    "wav",
    "m4a",
    "aac",
    "ogg",
    "opus",
    "wma",
    "alac",
    "aiff",
    "amr",
]
video_exts = [
    "mp4",
    "mkv",
    "webm",
    "avi",
    "mov",
    "flv",
    "wmv",
    "mpeg",
    "mpg",
    "3gp",
    "ts",
    "vob",
    "m4v",
]
playlist_exts = [
    "m3u",
    "m3u8",
    "pls",
]

# --- MPV Arguments ---
# These flags are passed directly to the mpv process.
mpv_default_args = [
    "--no-video",
    "--audio-display=no",
    "--msg-level=cplayer=warn",
    "--display-tags=",
    "--no-term-osd-bar",
    # Custom Now Playing UI
    "--term-playing-msg=╔══  MPV-MUSIC  ══╗",
    "--term-status-msg=▶ ${?metadata/artist:${metadata/artist} - }${?metadata/title:${metadata/title}}${!metadata/title:${media-title}} • ${time-pos} / ${duration} • (${percent-pos}%)",
]

```

---

## Development

- **Source Code:** Located in `src/`.
  * **`main.rs`**: Entry point. Handles CLI argument parsing, **TUI orchestration** (skim), and the main application loop.
  * **`config.rs`**: Manages configuration loading, validation, and defaults (Toml).
  * **`indexer.rs`**: The core library scanner. Uses `walkdir`, `rayon` (parallelism), and `lofty` for metadata.
  * **`player.rs`**: Wraps the `mpv` process, handling playback control and status flags.
  * **`search.rs`**: **YouTube Backend.** Wraps `yt-dlp` to fetch search results and stream URLs.
  * **`dep_check.rs`**: Validates runtime dependencies (mpv, yt-dlp) and environment health.

---

## License

MIT License. See [LICENSE](LICENSE).

---

## GenAI Disclosure

Generative AI (specifically Google Gemini, and sometimes others) was and is used for maintenance and development as an assistive tool.
