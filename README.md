# mpv-music

[![version](https://img.shields.io/github/v/release/FurqanHun/mpv-music?include_prereleases&color=blue)](https://github.com/FurqanHun/mpv-music/releases)
[![mpv-music build status](https://github.com/FurqanHun/mpv-music/actions/workflows/release.yml/badge.svg)](https://github.com/FurqanHun/mpv-music/actions/workflows/release.yml)

**mpv-music** is a blazing-fast, terminal-native music player and library browser. Originally a Bash hybrid, it has been completely rewritten in Rust for maximum performance, safety, and a seamless TUI experience.

It indexes your music collection into a lightning-fast library, providing fuzzy searching (via `skim`), metadata-rich previews, and deep integration with `mpv` for high-quality playback.

> [!WARNING]
> This README has not yet been fully updated for the new Rust-native architecture (v0.24+).
> mpv-music has now migrated from the legacy Bash hybrid system to a unified Rust core.
> For the archived Bash-based version, see:
> https://github.com/FurqanHun/mpv-music/tree/mpv-music-sh-archive

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
* **Windows (Native/Git Bash):** Not Supported. As one of the libraries used `skim` (which as of version 2.0.2) doesn't support windows.

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

The project now respects XDG standards. and only uses config folder to dump all as a fallback.

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
| `--config [editor]` | Open config file (default: nano/vi, or pass `nvim`/`zed`) |
| `--remove-config`, `--rm-conf` | Remove config file |
| `--log [viewer]` | Open log file (default: less, or pass `nvim`/`cat`) |
| `--rm-log` | Remove log file |
| `--video-ok` | Include video files in scans |
| `--volume [value]` | Set the initial volume |
| `--shuffle` | Enable shuffling |
| `--no-shuffle` | Disable shuffling |
| `--loop` | Enable looping |
| `--no-loop` | Disable looping |
| `--repeat` | Loop current track (Repeat One) |
| `--serial` | Force indexer to run serially (better for hdds) |
| `--ext=mp3,ogg` | Override file extensions |
| `--update` | Update the script to the latest version |
| `--reindex` | Force rebuild the full index |
| `--refresh-index` | Update index without wiping it |
| `--manage-dirs` | Open the directory management UI directly |
| `--add-dir [path]` | Add a/multiple new music directories |
| `--remove-dir [path]` | Remove a/multiple music directories |
| `-V, --verbose` | Increase verbosity, printing additional information |
| `--debug` | Print full system trace and detailed debug messages |
| `-g, --genre [val]` | Filter by genre. Opens picker if no value given |
| `-a, --artist [val]` | Filter by artist. Opens picker if no value given |
| `-b, --album [val]` | Filter by album. Opens picker if no value given |
| `-t, --title [val]` | Filter by track title |
| `-p, --play-all` | Play all tracks matching filters directly |
| `-l, --playlist` | Go directly to playlist mode |

_There are some flags missing in the rewrite now, but i'll soon add them. Please run `mpv-music --help` to see the available flags._

Any mpv flag also works: `--no-video`, `--volume=50`, `--shuffle`, etc.

Instead of using hte log rotation method now log is overwritten each time the program is run. And you can turn the logging off by setting `enable_file_logging = false` in your config.

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

- Where ever our sysetm has defined i should go, for modern linux systems it should be at `~/.config/mpv-music/music_index.jsonl`

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
shuffle = true
loop_mode = "inf"
volume = 60
music_dirs = [
    "/home/qan/Music",
]
video_ok = false
serial_mode = true
ytdlp_ejs_remote_github = true
ytdlp_useragent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/114.0"
enable_file_logging = false
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
mpv_default_args = [
    "--no-video",
    "--audio-display=no",
    "--msg-level=cplayer=warn",
    "--display-tags=",
    "--no-term-osd-bar",
    "--term-playing-msg=╔══  MPV-MUSIC  ══╗",
    "--term-status-msg=▶ ${?metadata/artist:${metadata/artist} - }${?metadata/title:${metadata/title}}${!metadata/title:${media-title}} • ${time-pos} / ${duration} • (${percent-pos}%)",
]

```

---

## Development

- **Source Code:** Located in `src/`.
  * `config.rs`
  * `dep_check.rs`
  * `indexer.rs`
  * `main.rs`
  * `player.rs`
  * `search.rs`
---

## License

MIT License. See [LICENSE](LICENSE).

---

## GenAI Disclosure

Generative AI (specifically Google Gemini, and sometimes others) was and is used for maintenance and development as an assistive tool.
