# mpv-music

[![version](https://img.shields.io/github/v/release/FurqanHun/mpv-music?include_prereleases&label=release&labelColor=27303D&color=blue&logo=github&logoColor=FFFFFF&style=flat)](https://github.com/FurqanHun/mpv-music/releases)
[![mpv-music build status](https://img.shields.io/github/actions/workflow/status/FurqanHun/mpv-music/release.yml?label=build&labelColor=27303D&logo=github&logoColor=FFFFFF&style=flat)](https://github.com/FurqanHun/mpv-music/actions/workflows/release.yml)
[![crates.io](https://img.shields.io/crates/v/mpv-music?label=crates.io&labelColor=27303D&color=orange&logo=rust&logoColor=FFFFFF&style=flat)](https://crates.io/crates/mpv-music)

[![github downloads](https://img.shields.io/github/downloads/FurqanHun/mpv-music/total?label=github%20downloads&labelColor=27303D&color=0D1117&logo=github&logoColor=FFFFFF&style=flat)](https://github.com/FurqanHun/mpv-music/releases)
[![crates.io downloads](https://img.shields.io/crates/d/mpv-music?label=crates.io%20downloads&labelColor=27303D&color=0D1117&logo=rust&logoColor=FFFFFF&style=flat)](https://crates.io/crates/mpv-music)


**mpv-music** is a blazing-fast, terminal-native music player and library browser written in Rust.

It indexes your music collection into a lightning-fast library, providing fuzzy searching (via `skim`), metadata-rich previews, and deep integration with `mpv` for high-quality playback.

*(Looking for the legacy Bash version? See [mpv-music-sh-archive](https://github.com/FurqanHun/mpv-music/tree/mpv-music-sh-archive))*

---

## Table of Contents

- [Features](#features)
- [Dependencies](#dependencies)
- [Installation](#installation)
- [Usage](#usage)
- [Indexing](#indexing)
- [Configuration](#configuration)
- [FAQ](#faq)
- [Development](#development)
- [License](#license)
- [GenAI Disclosure](#genai-disclosure)

---

## Features

- **Native TUI:** Integrated Rust TUI (powered by `skim`) for deep UI customization and instant fuzzy searching.
- **Lightning-Fast Indexing:** Automatically scans your music directories utilizing multi-threading, caching metadata (Artist, Album, Title, Genre) into a JSONL index.
- **Self-Healing Index:** Automatically validates index integrity on startup. It detects corruption (e.g., from power loss), surgically repairs broken lines to save your library, or triggers a smart rebuild to prevent crashes. In a blink of an eye.
* **Rich Metadata Previews:** View song title, artist, album, and genre directly in the skim preview window.
* **Interactive Selection with Multiple Modes:**
  * **Directory Mode:** Navigate folders with clean names instead of full paths.
  * **Track Mode:** Fuzzy-search individual tracks with metadata previews.
  * **Playlist Mode:** Find and play your saved `.m3u` or `.pls` playlists.
  * **Tag Filter Mode:** Drill down by genre, artist, album, or title interactively.
  * **Play All:** Instantly play your entire indexed library.
  * **Search & Stream URL:** Search YouTube or stream URLs directly from the menu.
  * **Radio Mode**: Built-in support for diverse internet radio stations. Stations are sourced from respected, ad-free streams (discovered via the open [Radio-Browser.info](https://www.radio-browser.info/) database, aside from the official LISTEN.moe), including:
    * **LISTEN.moe** (J-Pop / K-Pop) - Includes live WebSocket metadata synchronization ([listen.moe](https://listen.moe)).
    * **SomaFM** (Ambient / Metal) - Listener-supported, commercial-free radio from San Francisco ([somafm.com](https://somafm.com)).
    * **Lofi 24/7** (Lofi Hip Hop) - Provided via fastcast4u.
    * **Vocaloid Radio** (Vocaloid) - Independent SHOUTcast community stream ([vocaloidradio.com](https://vocaloidradio.com)).

> [!NOTE]
> Please consider donating directly to these independent stations through their websites to help keep their servers running!

  * **Settings:** Manage mpv-music settings directly from the menu.
* **Direct File/URL Playback:** Instantly play local audio/video files or URLs (YouTube, streams) without going through the menu.
* **Custom Directory Support:** Pass a folder path to browse and filter only that directory instead of your full library.
* **CLI Filtering:** Use flags like `--genre`, `--artist`, `--album`, `--title` for direct filtering. Pass a value or omit it to open an interactive picker.
* **Smart Matching:** CLI filters attempt exact matches first, then fall back to partial matches with disambiguation.
* **Configurable File Types:** Support for both audio and video extensions, easily tweakable.
* **Custom MPV Flags:** Pass mpv flags directly or set defaults in the config.
* **Video Toggle:** `--video-ok` lets you include videos in your library scans.
* **Visual Playback:** `--watch (-w)` forces the MPV window to open, allowing you to watch videos or see cover art/visualizations during playback.
* **YouTube Auto-Config:** Automatically detects JS runtimes (Deno, Node, QuickJS, Bun) for yt-dlp YouTube playback.
* **Clean Logging:** Verbose (`-v`) and debug (`-d`) modes, multi-session logging (`logs/session_<timestamp>_<pid>.log`) with configurable retention, and built-in interactive log viewer (`--log`).

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

* **Linux:** Native. The app is built and tested primarily for Linux.
* **WSL (Windows Subsystem for Linux):** Fully Supported.
* **macOS / BSD:** Should work fine on macOS and BSD systems. Check [FAQ](#platform-support).
* **Windows (Native/Git Bash):** Fully Supported. Check [FAQ](#platform-support).

### From crates.io (Recommended)

If you have Rust installed:

```bash
# Full installation with update checker
cargo install mpv-music --features update

# Minimal installation (smaller binary, no --update flag)
cargo install mpv-music
```

### Pre-built Binaries (Recommended)

> [!TIP]
> Unsure which binary is right for your system? Visit the interactive **[Downloads Page](https://furqanhun.github.io/mpv-music/downloads)** to download the exact version compiled for your OS and architecture.

1. Download the latest binary for your architecture from the [Releases](https://github.com/FurqanHun/mpv-music/releases) page.
2. Make it executable and move it to your path:
    ```bash
    chmod +x mpv-music
    mv mpv-music ~/.local/bin/
    ```
or Alternatively, you can use the automated installer scripts to handle the process:

**Linux / macOS:**
```bash
curl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.sh | bash
```

**Windows (PowerShell):**
```powershell
iwr https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.ps1 -UseBasicParsing | iex
```

### From Source
Requires Rust **1.93.0+**.
```bash
git clone https://github.com/FurqanHun/mpv-music.git
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
> It is **recommended** that you first run `mpv-music --manage-dirs` to customize music directories before indexing (unless you only keep your music in `$HOME/Music`). And if your music is on an HDD, you may want to run `--serial` or set `serial_mode = true` in your config, using `mpv-music --config`.

That creates:

- Config: `~/.config/mpv-music/config.toml`

- Index: `~/.local/share/mpv-music/music_index.jsonl`

- Logs: `~/.local/share/mpv-music/logs/` (e.g. `session_<timestamp>_<pid>.log`)

The project respects XDG standards and uses the `directories` crate to automatically support proper config/data paths across Linux, Windows, and macOS.

> [!TIP]
> **Optional: Clean Terminal Icons (Nerd Fonts)**
> By default, `mpv-music` uses standard emojis that work everywhere out of the box. If you prefer crisp monochrome icons that automatically follow your terminal's color theme, set `nerd_fonts` in `config.toml`:
> ```toml
> # Options: "none" (default), "mono", "normal"
> nerd_fonts = "mono"
> ```
> * **Font Recommendation**: You don't need to replace your system font. Simply download [Symbols Nerd Font](https://github.com/ryanoasis/nerd-fonts/tree/master/patched-fonts/NerdFontsSymbolsOnly) (`SymbolsNerdFontMono-Regular.ttf` for `"mono"`, or `SymbolsNerdFont-Regular.ttf` for `"normal"`) as a fallback font.

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

> [!TIP]
> **Menu Navigation:**  
> Use arrow keys or type to fuzzy-filter items, and `ENTER` to select. Press `ESC` anywhere to instantly go back / cancel (or select the `q) Back` option with `ENTER`).

### Options:

| Option | Description |
| :--- | :--- |
| `[TARGET]` | Directly play a file, directory, or URL |
| `-r`, `--refresh-index` | Update index (incremental scan). Detects new/changed files. |
| `--reindex` | Force a full re-scan of the library. |
| `-u`, `--update` | Check for application updates. |
| `-y`, `--yes` | Auto-confirm update prompts (use with `--update`). |
| `--add-dir <PATH>...` | Add directory (e.g. `--add-dir /music /other`). |
| `--remove-dir <PATH>...` | Remove directory (aliases: `--rm-dir`). |
| `--manage-dirs` | Open the Interactive Directory Manager. |
| `-c`, `--config [<EDITOR>]` | Edit config file. |
| `--remove-config` | Delete config file (Reset) (aliases: `--rm-conf`). |
| `--log [<PAGER>]` | View session logs (opens interactive selector if multiple logs exist). |
| `--remove-log [<COUNT>]` | Delete all session logs, or the oldest N logs (aliases: `--rm-log`). |
| `-p`, `--play-all` | Play all tracks immediately. |
| `-l`, `--playlist [<VAL>]` | Open Playlist Mode. Opens picker if no value given. |
| `--video-ok` | Allow video files. |
| `--no-video` | Negates `--video-ok`, and overrides it in config. |
| `--watch (-w)` | Play with video window enabled (forces visual mode). |
| `--no-watch` | Disable video window (forces audio mode, which is the default). |
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
| `--player <BIN>` | Specify media player binary or compatible fork (defaults to 'mpv'). |
| `--ytdlp <BIN>` | Specify the yt-dlp binary or compatible fork (defaults to 'yt-dlp'). |
| `--search [<SEARCH>]` | Search YouTube directly (aliases: `--yt`). |
| `-h`, `--help` | Print help. |
| `-V`, `--version` | Print version. |
| `--radio [<STATION>]` | Open Radio Mode directly, or play a station (e.g., `jpop`, `lofi`, `vocaloid`). |
| `--mpv-args <ARGS>`	| Pass raw, unparsed arguments straight to the mpv engine. |

Any mpv flag also works: `--no-video`, `--volume=50`, `--shuffle`, etc.

Session logs are saved to `~/.local/share/mpv-music/logs/` (`session_<timestamp>_<pid>.log`). By default, the most recent 3 sessions are preserved (configurable via `max_log_sessions`). You can disable file logging by setting `enable_file_logging = false` in your config.

### Examples:

```bash
mpv-music                              # full interactive menu
mpv-music /path/to/music               # interactive in a specific folder
mpv-music ~/Music/track.flac           # plays file instantly
mpv-music "https://youtube.com/watch..." # plays URL instantly
mpv-music --yt "lofi"          # search YouTube directly from CLI (alias: --search)
mpv-music --radio jpop                 # play a radio station directly (or --radio for picker)
mpv-music /path/to/folder -a           # pick artist from that folder only
mpv-music --genre="Rock" --play-all    # play all rock tracks
mpv-music -g "Rock,Pop" --play-all     # multi-genre comma filtering
mpv-music --artist="Ado"               # fuzzy search by artist
mpv-music -p -a ado                     # play all tracks by Ado
mpv-music -g -a "Daft Punk" -p         # pick genre, then play all Daft Punk
mpv-music -w ~/Music/video.mp4         # play with video window enabled (watch mode)
mpv-music --volume=50 --shuffle        # custom mpv flags
mpv-music --reindex                    # rebuild the index from scratch
mpv-music --debug                      # run with full logging enabled
mpv-music --verbose                    # prints verbose messages
mpv-music --add-dir /path/to/music /path/to/music2 # Add multiple directories
mpv-music --remove-dir /path/to/music /path/to/music2 # Remove multiple directories
mpv-music --manage-dirs                  # Manage directories
mpv-music --log                          # View recent session logs
mpv-music --remove-log                   # Delete all session logs (or --remove-log 1 for oldest)
```

---

## Indexing

Your music library is indexed to:

- Wherever your system defines program data should go; on modern Linux systems, this defaults to `~/.local/share/mpv-music/music_index.jsonl`

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
> **Indexing vs. Watching:**
> * Use `--video-ok` (or set `video_ok = true`) to **scan** and include video files in your library.
> * Use `--watch` (or set `watch = true`) when playing to actually **show** the video window. 
>
> Example: `mpv-music --video-ok --reindex` to scan, then `mpv-music -w` to watch.

---

## Configuration

If a config file does not exist, mpv-music will create one at startup. If an existing config file is missing newer options, they are automatically populated with their defaults. To customize the behavior:

```
mpv-music --config
```
**Options:**

```toml
# --- General Playback ---
shuffle = true
loop_mode = "inf"  # Options: "playlist" (same as inf), "track", "no", "inf", "5" (number of loops)
volume = 100

# --- Appearance ---
nerd_fonts = "none"       # Options: "none" (standard emojis), "mono", "normal"

# --- Library Management ---
music_dirs = [
    "/home/user/Music",
    "/mnt/storage/songs",
]
video_ok = false         # Set to true to include video files in the index
watch = false            # Set to true to actually show the video window when playing
serial_mode = false      # Set to true to force single-threaded scanning (better for HDDs)
scan_hidden_dirs = false # Set to true to allow indexing of hidden directories (e.g. .music)

# --- Media Player & External Binaries ---
player = "mpv"            # Media player binary, fork, or path (e.g. "mpv", "mpvnet", "/usr/bin/mpv")
ytdlp = "yt-dlp"           # yt-dlp binary, fork, or path (e.g. "yt-dlp", "yt-dlp-nightly")
# Set to true if you installed yt-dlp via package manager (apt/pacman). 
# Keep false if you downloaded the binary directly from GitHub.
ytdlp_ejs_remote_github = false 
ytdlp_useragent = "default"

# --- Logging ---
# If true, INFO/WARN logs are saved to file. 
# If false, logs are only shown on screen when running with --verbose or --debug.
enable_file_logging = true
max_log_sessions = 3     # Number of recent session logs to preserve in logs/ directory

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
# Additional flags to pass to the player (e.g. ["--gapless-audio=yes", "--af=scaletempo2"]).
# Any flag specified here will override built-in defaults.
# Built-in defaults applied automatically:
#   --no-video, --audio-display=no (automatically ignored if --watch is used)
#   --msg-level=cplayer=warn
#   --display-tags=
#   --no-term-osd-bar
#   --term-playing-msg (shows " ──  MPV-MUSIC ──" or " ╔══  MPV-MUSIC  ══╗" based on nerd_fonts)
#   --term-status-msg (shows "  ..." or " ▶ ..." based on nerd_fonts)
mpv_args = []
```

### Configuration Reference:

| Key | Type | Default | CLI Equivalent | Description |
| :--- | :--- | :--- | :--- | :--- |
| `player` | String | `"mpv"` | `--player <BIN>` | Media player binary, fork, or path (e.g. `"mpv"`, `"mpvnet"`). |
| `shuffle` | Boolean | `true` | `-s`, `--shuffle` / `--no-shuffle` | Enable or disable random shuffle by default. |
| `loop_mode` | String / Integer | `"inf"` | `--loop`, `--no-loop`, `--repeat` | Looping mode: `"inf"` (or `"playlist"`), `"track"` (or `"file"`), `"no"` (or `"off"`), or loop count (e.g. `5`). |
| `volume` | Integer | `100` | `--volume <0-130>` | Playback volume percentage (`0` to `130`). Capped at 130 max. |
| `nerd_fonts` | String / Boolean | `"none"` | — | Icon set: `"none"` (emojis), `"mono"` (monospace Nerd Fonts), `"normal"` (proportional/symbols), or `true`/`false`. |
| `music_dirs` | Array of Strings | `["$HOME/Music"]` | `--manage-dirs`, `--add-dir`, `--remove-dir` | List of folder paths to index and search. |
| `video_ok` | Boolean | `false` | `--video-ok` / `--no-video` | Scan and index video files alongside audio tracks. |
| `watch` | Boolean | `false` | `-w`, `--watch` / `--no-watch` | Open MPV's video window during playback (shows video or album art). |
| `serial_mode` | Boolean | `false` | `--serial` | Force single-threaded indexing. Recommended for mechanical HDDs to prevent thrashing. |
| `scan_hidden_dirs` | Boolean | `false` | — | Allow indexing inside hidden directories (e.g. `.music`). |
| `ytdlp` | String | `"yt-dlp"` | `--ytdlp <BIN>` | Custom `yt-dlp` binary, fork, or path (e.g. `"yt-dlp-nightly"`). |
| `ytdlp_ejs_remote_github` | Boolean | `false` | — | Enables remote PhantomJS/EJS solver fallback for package manager builds of `yt-dlp`. |
| `ytdlp_useragent` | String | `"default"` | — | Custom User-Agent header for `yt-dlp` requests (`"default"` uses modern Firefox UA). |
| `enable_file_logging` | Boolean | `true` | `--log`, `--debug` | Save logs to `~/.local/share/mpv-music/logs/`. When false, logs only output to stderr if `--verbose` or `--debug` is used. |
| `max_log_sessions` | Integer | `3` | `--remove-log` | Maximum number of recent session logs to preserve in `logs/` (minimum: 1). Older sessions are pruned on startup. |
| `audio_exts` | Array of Strings | `["mp3", "flac", ...]` | `-e`, `--ext <LIST>` | List of recognized audio extensions. |
| `video_exts` | Array of Strings | `["mp4", "mkv", ...]` | — | List of recognized video extensions (active when `video_ok = true`). |
| `playlist_exts` | Array of Strings | `["m3u", "m3u8", "pls"]` | — | List of recognized playlist extensions. |
| `mpv_args` | Array of Strings | `[]` | `--mpv-args <ARGS>` | Custom raw arguments passed to mpv. Overrides built-in defaults. (Aliases legacy `mpv_default_args`). |

---

## FAQ

<details>
<summary><strong>Q. Why Rust?</strong></summary>

The Bash version was a hack that grew too big. It was parsing hell, ngl. That being said, the reason for Rust is simple: I first thought about Zig, C, or Rust, but having read a couple of Rust chapters few months or years ago, reading Rust code felt way more natural. Plus, with the compiler keeping things in check, I could make sure LLM suggestions wouldn't fuh me up in places I wasn't even aware of.

</details>

<details>
<summary><strong>Q. Why not use a "real" database like SQLite?</strong></summary>

Because you don't need it. Your music library is likely under 100,000 tracks. A flat JSONL (JSON Lines) file is:

1. **Human readable:** You can `cat` or `grep` it.
2. **Fast enough:** We load and parse 20,000 lines in milliseconds.
3. **Corruption proof:** If one line gets corrupted, you only lose one song, not the whole database. And it is easily detectable—plus with `refresh-index`, it's automatically fixed.

</details>

<a name="platform-support"></a>
<details>
<summary><strong>Q. How well supported are Windows, macOS, and BSD?</strong></summary>

- **Windows:** Well, despite me testing it a bit on a vm on ma potato laptop, I cannot guarantee no bugs on it, as I test it like I am touching acid. I didn't test it extensively, more than launching mpv correctly and reading just one song/updating and spawning the mpv, and radio mode. It's no longer in beta, but in my head it will always be, let's say that. Windows is still Windows.
- **macOS / BSD:** It compiles and follows Unix standards, XDG paths, and standard CLI conventions, so it should work fine on macOS and BSD systems. But since I don't daily-drive a Mac or BSD machine, I haven't tested it extensively either.

> I really need someone to test it on Windows (and Mac/BSD) tho, fr. Any feedback or bug reports are super appreciated!

</details>

<details>
<summary><strong>Q. YouTube playback isn't working!</strong></summary>

That is likely not a bug in `mpv-music`. YouTube is constantly fighting `yt-dlp`.

1. Update `yt-dlp` (`yt-dlp -U`).
2. Make sure you have a JS runtime (Node, Deno, Bun) installed. YouTube now requires executing JavaScript to decipher video signatures. `mpv-music` tries to auto-detect this, but it can't perform miracles.
3. If you installed `yt-dlp` via package managers (apt, dnf, pacman) instead of official GitHub binaries, consider enabling `ytdlp_ejs_remote_github = true` in `config.toml`.
4. Or you can try changing the `ytdlp_useragent` in config.

</details>

<details>
<summary><strong>Q. mpv-music isn't picking up metadata for video files or some other audio formats?</strong></summary>

The original `mpv-music` utilized `ffprobe` to parse metadata from everything. The Rust version uses `lofty` (native Rust library) for metadata parsing, which is infinitely faster (in some sense) but supports fewer formats (mostly Audio). However, they should be enough for 99% of people.

Currently supported formats by `lofty`:

| File Format | Metadata Format(s)           |
|-------------|------------------------------|
| AAC (ADTS)  | `ID3v2`, `ID3v1`             |
| Ape         | `APE`, `ID3v2`\*, `ID3v1`    |
| AIFF        | `ID3v2`, `Text Chunks`       |
| FLAC        | `Vorbis Comments`, `ID3v2`\* |
| MP3         | `ID3v2`, `ID3v1`, `APE`      |
| MP4         | `iTunes-style ilst`          |
| MPC         | `APE`, `ID3v2`\*, `ID3v1`\*  |                        
| Opus        | `Vorbis Comments`            |
| Ogg Vorbis  | `Vorbis Comments`            |
| Speex       | `Vorbis Comments`            |
| WAV         | `ID3v2`, `RIFF INFO`         |
| WavPack     | `APE`, `ID3v1`               |

For unsupported formats, the indexer falls back to filename parsing. I may implement an opt-in `ffprobe` fallback in the future if there is demand, but right now there's me and one other person I know of that actually uses this and we don't need it. Actually, there are now an estimated 30–50+ users, but no one has complained so far since the vast majority just use Opus, FLAC, WAV, or MP3 these days.
</details>

---

## Development

- **Source Code:** Located in `src/`.
  * **`main.rs`**: Entry point. Minimal bootstrap delegating to `app::run`.
  * **`ui.rs`**: Centralized terminal UI badges (`[Info]`, `[Success]`, `[Warning]`, `[Error]`), user prompts, and logging forwarding.
  - **`app/`**: Application lifecycle and execution engine.
    * **`mod.rs`**: Top-level coordinator and mode dispatch.
    * **`flags.rs`**: Utility flag handling (`--log`, `--config`), runtime CLI overrides, and directory operations.
    * **`logging.rs`**: Multi-session file logging (`flexi_logger`), retention pruning, and stderr formatting.
    * **`library.rs`**: Track loading, session directory scanning, and index syncing.
    * **`filter.rs`**: Multi-stage CLI track filtering, comma tag matching, and interactive disambiguation.
  * **`cli.rs`**: Defines the command-line interface arguments and flags (using `clap`).
  - **`tui/`**: The Terminal User Interface module.
    * **`mod.rs`**: Core orchestration, main & settings menus, and public API facade.
    * **`runner.rs`**: Skim runner primitives (fuzzy single selection, multi-selection, and text prompt).
    * **`tracks.rs`**: Library exploration modes (Track, Directory, and Playlist views).
    * **`dirs.rs`**: Interactive music directory manager (add, remove, and path validation loops).
    * **`tags.rs`**: Interactive tag filtering (Genre, Artist, Album) and post-filter actions.
    * **`search.rs`**: Interactive YouTube search and URL streaming flow.
    * **`radio.rs`**: Interactive internet radio station picker and filtering.
    * **`filter.rs`**: CLI track filtering helper (exact and partial disambiguation).
    * **`items.rs`**: Data structures and Skim preview adapters (Tracks, Directories, Playlists, Tags, Search results).
    * **`icons.rs`**: Universal emoji and Symbols Nerd Font (Mono/Normal) abstractions.
  * **`config.rs`**: Manages configuration loading, validation, and defaults (Toml).
  * **`indexer.rs`**: The core library scanner. Uses `walkdir`, `rayon` (parallelism), and `lofty` for metadata.
  - **`player/`**: Wraps the media player process, handling command construction, playback control, queue generation, and temporary file cleanup.
    * **`mod.rs`**: Public playback entrypoints (`play`, `play_files`, `play_radio`), process execution, signal handling, and drop guards.
    * **`builder.rs`**: `MpvCommandBuilder` for deterministic command generation, flag assembly, and terminal banner/status formatting.
    * **`target.rs`**: `TargetKind` enum, target classification, priority scanning, and playlist inspection.
    * **`ytdlp.rs`**: JS runtime probing (Deno/Node/Bun) and yt-dlp health checking.
    * **`radio.rs`**: Radio IPC socket setup and Listen.moe WebSocket synchronization.
  * **`search.rs`**: **YouTube Backend.** Wraps `yt-dlp` to fetch search results and stream URLs.
  * **`dep_check.rs`**: Validates runtime dependencies (mpv, yt-dlp versions) and environment health.
  * **`update.rs`**: Handles version comparison (SemVer) and checks GitHub for releases.
  * **`radio/`**: Radio Backend.
    * **`mod.rs`**: Defines the list of available internet radio stations and their stream URLs.
    * **`listen_moe.rs`**: Manages WebSocket connections to LISTEN.moe and synchronizes live metadata via IPC.

- **Automated Tests:**
  The codebase features an automated test suite spanning in-module unit tests and end-to-end user flow integration tests:
  ```bash
  cargo test --all-features
  ```
  * **In-Module Unit & Edge-Case Tests (`src/`):** White-box tests verifying config serialization, boundary clamping, auto-healing missing keys, legacy migrations, multi-session retention pruning, filename heuristics, JSONL corruption recovery, command construction, and WebSocket metadata parsing.
  * **End-to-End User Flow Tests (`tests/`):** Black-box integration tests executing the compiled binary in isolated sandboxes to verify real CLI workflows (`--add-dir`, `--rm-dir`, `--remove-log`, `--rm-conf`, `--play-all`, and filter flags).

---

## License

MIT License. See [LICENSE](LICENSE).

---

## GenAI Disclosure

Generative AI (specifically Google Gemini, and sometimes others) was and is used for maintenance and development as an assistive tool.
