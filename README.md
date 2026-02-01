# mpv-music

[![version](https://img.shields.io/github/v/release/FurqanHun/mpv-music?include_prereleases&color=blue)](https://github.com/FurqanHun/mpv-music/releases)
[![mpv-music-indexer build status](https://github.com/FurqanHun/mpv-music/actions/workflows/indexer-build.yml/badge.svg)](https://github.com/FurqanHun/mpv-music/actions/workflows/indexer-build.yml)

**mpv-music** is a blazing-fast terminal music player and library browser built on mpv.  
Provides instant playback, fuzzy searching (fzf), metadata-rich previews, and fully configurable CLI controls with no background daemon needed.

In short, it focuses on library indexing for super-fast access to your music collection with a clean terminal UI, passing your selection to mpv with config-defined arguments.


---

## Table of Contents

* [Features](#features)
* [Dependencies](#dependencies)
* [Installation](#installation)
* [Usage](#usage)
* [Indexing](#indexing)
* [Configuration](#configuration)
* [Development](#development)
* [License](#license)
* [GenAI Disclosure](#genai-disclosure)

---

## Features

* **Blazing-Fast Indexed Searching:** Automatically indexes your music library into a JSONL (JSON Lines) file for lightning-fast search using fzf. If the index does not exist, it will be created on first run.
* **Hybrid Indexing Engine:** Uses a high-performance **Rust binary** for millisecond-speed scanning of massive libraries. Automatically falls back to a robust Bash+ffprobe method if the binary is missing or incompatible.
* **Self-Healing Index:** Automatically validates index integrity on startup. It detects corruption (e.g., from power loss), surgically repairs broken lines to save your library, or triggers a smart rebuild to prevent crashes. In a blink of an eye.
* **Rich Metadata Previews:** View song title, artist, album, and genre directly in the fzf preview window.
* **Interactive Selection with Multiple Modes:**
  * **Directory Mode:** Navigate folders with clean names instead of full paths.
  * **Track Mode:** Fuzzy-search individual tracks with metadata previews.
  * **Playlist Mode:** Find and play your saved `.m3u` or `.pls` playlists.
  * **Tag Filter Mode:** Drill down by genre, artist, album, or title interactively.
  * **Play All:** Instantly play your entire indexed library.
  * **URL Mode:** Paste YouTube or stream URLs directly from the menu.
  * **Manage Directories:** Add or remove music folders directly from the UI without editing config files.
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
* **mpv** - https://mpv.io
* **fzf** - https://github.com/junegunn/fzf
* **jq** - for parsing the index.
  Install: `sudo apt install jq` or `brew install jq`
* **ffmpeg** - ffprobe is used to extract metadata.
  https://ffmpeg.org
* **GNU find** - not BSD find, script checks this at startup.

#### Optional (but recommended):
* **yt-dlp** - for playing URLs.
  https://github.com/yt-dlp/yt-dlp
* **mediainfo** - fallback metadata reader.
  Install: `sudo apt install mediainfo` or `brew install mediainfo`
* **JS Runtime** - for YouTube playback (Deno, Node.js, QuickJS, or Bun). Deno is recommended.

> [!NOTE]
  > YouTube playback now requires a JS runtime to bypass anti-bot protections.
  > `mpv-music` automatically detects and configures **Deno**, **Node**, **QuickJS**, or **Bun** for `yt-dlp`.

---

## Installation

### Supported Systems

* **Linux:** Native. The script is built and tested primarily for Linux (GNU tools).
* **WSL (Windows Subsystem for Linux):** Fully Supported. This is the recommended way to run it on Windows.
* **macOS / BSD:** Experimental. These systems use BSD variants of standard tools (sed, find, readlink), which differ from the GNU versions used in this script. You may need to install GNU tools (coreutils, findutils, gnu-sed) and ensure they are in your PATH.
* **Windows (Native/Git Bash):** Not Supported. Native path handling (`C:\` vs `/`) prevents this from working.

> [!Tip]
> You can use WSL (Windows Subsystem for Linux) to run `mpv-music` on Windows.

### Option 1: Quick Install (Recommended)

Run this command to install the latest stable release automatically.  
It will check dependencies and ask you where to install the script.

```bash
curl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.sh | bash
```

### Option 2: Manual Install

1. Download the latest `mpv-music` script from the [Releases page](https://github.com/FurqanHun/mpv-music/releases).
2. (Optional but Recommended) Download the binary for your architecture (`x86_64`, `aarch64`, or `armv7`) and rename it to `mpv-music-indexer`.

> [!NOTE]
>  You can compile the [mpv-music-indexer](https://github.com/FurqanHun/mpv-music/tree/master/crates/mpv-music-indexer) yourself.

3. Make it executable:
```bash
chmod +x mpv-music mpv-music-indexer
```

4. Move to your PATH:
```bash
mkdir -p ~/.local/bin
mv mpv-music ~/.local/bin/
mv mpv-music-indexer ~/.local/bin/  # Optional
```

### First run (setup):
```bash
mpv-music
```

> [!IMPORTANT]
> Running `mpv-music` for the first time will automatically index `$HOME/Music`.
> It is **recommended** that you first run `mpv-music --manage-dirs` to customize music directories before indexing (unless you only keep your music in `$HOME/Music`). And if your music is on an HDD, you may want to run `--serial` or set `SERIAL_MODE=true` in your config, using `mpv-music --config`.

That creates:
- `~/.config/mpv-music/mpv-music.conf` (the actual config)
- `~/.config/mpv-music/music_index.jsonl` (your indexed library)
- `~/.config/mpv-music/mpv-music.log` (verbose/debug logs file)

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
|--------|-------------|
| `-h, --help` | Show the help message and exit |
| `-v, --version` | Show the script version and exit |
| `--config` | Open config file in text editor (nano/vi) |
| `--config=editor` | Open config in specified editor |
| `--remove-config`, `--rm-conf` | Remove config file |
| `--video-ok` | Include video files in scans |
| `--volume` | Set the initial volume |
|`--serial` | Force indexer to run serially (better for hdds) |
| `--ext=mp3,ogg` | Override file extensions |
| `--update` | Update the script to the latest version |
| `--reindex` | Force rebuild the full index |
| `--refresh-index` | Update index without wiping it |
| `--manage-dirs` | Open the directory management UI directly |
| `--add-dir [path]` | Add a/multiple new music directories |
| `--remove-dir [path]` | Remove a/multiple music directories |
| `-V, --verbose` | Increase verbosity, printing additional information |
| `--debug` | Print detailed debug messages |
| `-g, --genre [val]` | Filter by genre. Opens picker if no value given |
| `-a, --artist [val]` | Filter by artist. Opens picker if no value given |
| `-b, --album [val]` | Filter by album. Opens picker if no value given |
| `-t, --title [val]` | Filter by track title |
| `-p, --play-all` | Play all tracks matching filters directly |
| `-l, --playlist` | Go directly to playlist mode |

Any mpv flag also works: `--no-video`, `--volume=50`, `--shuffle`, etc.

The log file automatically rotates after reaching the configured size (default: 5120KB).
If `LOG_MAX_SIZE_KB` is set to `0` in your config, log messages will only be displayed and not saved.

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
mpv-music --verbose --debug            # run with full logging enabled
mpv-music --add-dir /path/to/music /path/to/music2 # Add multiple directories
mpv-music --remove-dir /path/to/music /path/to/music2 # Remove multiple directories
mpv-music --manage-dirs                  # Manage directories
```

---

## Indexing

Your music library is indexed to:

```
~/.config/mpv-music/music_index.jsonl
```

### Why?

Searching the filesystem with find every time is slow, especially if you have a large music collection. So mpv-music caches an index using JSONL (JSON Lines) for:

- Fast filtering
- Instant append updates
- Metadata previews
- Offline-friendly behavior

### Hybrid Engine

`mpv-music` employs a dual-engine approach:

1. Rust Indexer (Priority): If the mpv-music-indexer binary is found, it is used to scan your library in parallel. This is incredibly fast.
2. Bash/FFprobe (Fallback): If the binary is missing or fails, the script seamlessly switches to the legacy method.

### Maintaining:

* `--reindex` - Full rebuild
* `--refresh-index` - Smart update (only processes new/modified files)

> [!NOTE]
> `--refresh-index` currently uses the Bash logic to append new files. The Rust indexer is so fast that it simply rebuilds the entire index (equivalent to `--reindex`) to ensure consistency.

> [!TIP]
> Run these flags alongside `--video-ok` to include video files in the index.
> Example: `mpv-music --video-ok --reindex`
> Or you can set `VIDEO_OK=true` in your config file.

---

## Configuration

If a config file does not exist, mpv-music will create one at startup. To customize the behavior, edit:

```
~/.config/mpv-music/mpv-music.conf
```

**Options:**

```bash
# mpv-music configuration

# --- Visual Customization ---
# Banner text (displayed at start of track)
# Uses ANSI escape codes or simple text, which is directly passed to mpv
BANNER_TEXT='\n╔══  MPV-MUSIC  ══╗\n'

# Status Bar Logic (Complex MPV variables)
# Uses single quotes to prevent early expansion.
STATUS_MSG='▶ ${?metadata/artist:${metadata/artist} - }${?metadata/title:${metadata/title}}${!metadata/title:${filename}} • ${time-pos} / ${duration} • (${percent-pos}%)'

# --- Feature Flags ---
# Set to true to include video files in library scans by default.
VIDEO_OK=false

# Set to true to force single-threaded indexing (good for HDDs).
SERIAL_MODE=false

# Default playback volume (0-130).
VOLUME=100

# --- MPV Arguments ---
# Defined as a Bash Array for cleaner formatting and safety.
MPV_DEFAULT_ARGS=(
    --loop-playlist=inf
    --shuffle
    --no-video
    --audio-display=no
    --msg-level=cplayer=warn
    --display-tags=
    --no-term-osd-bar
    "--term-playing-msg=$(tput clear)$BANNER_TEXT"
    "--term-status-msg=$STATUS_MSG"
)

# File Extensions
AUDIO_EXTS="mp3 flac wav m4a aac ogg opus wma alac aiff amr"
VIDEO_EXTS="mp4 mkv webm avi mov flv wmv mpeg mpg 3gp ts vob m4v"
PLAYLIST_EXTS="m3u m3u8 pls"

# Log Rotation (set to 0 to disable file logging)
LOG_MAX_SIZE_KB=5120

# Music Directories (double quotes separated)
MUSIC_DIRS=(
    "$HOME/Music"
    "/mnt/media/audios"
)

```

---

## Development

This project is developed using a modular (more of a faux module) source structure.
* **Installer**: Located in root `install.sh`
* **Updater**: Located in root `mpv-music-updater`
* **Rust Source (Crates)**:
  * `crates/mpv-music-indexer/`: The source code for the high-performance indexing binary.
* **Source Code:** Located in `src/`.
  * `01_vars.sh` Configuration variables.
  * `02_utils.sh` Utility functions (i.e, logging, updater etc)
  * `03_config.sh` Config handling and dependency checks.
  * `04_metadata/` Modular metadata extraction logic (Helpers, Legacy, Rust Wrappers).
  * `05_ui.sh` FZF UI and selection modes.
  * `06_main.sh` Main script logic and argument parsing.
* **Building:** Run `./build.sh` to compile the modules into the final `mpv-music` executable.

> [!NOTE]
> `build.sh` only concatenates the Bash modules. To build the Rust indexer, you must use `cargo build --release` inside the crates/mpv-music-indexer directory.

* *Requirement:* To build the Rust indexer, you need Rust **1.85.0+** installed.

---

## License

MIT License. See [LICENSE](LICENSE).

---

## GenAI Disclosure

Generative AI (specifically Google Gemini, and sometimes others) was and is used for maintenance and development as an assistive tool.
