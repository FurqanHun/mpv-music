# mpv-music
![version](assets/version.svg)

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
* **Self-Healing Index:** Automatically validates index integrity on startup. It detects corruption (e.g., from power loss), tries to repair by removing the last line, or triggers a smart rebuild to prevent crashes. In a blink of an eye.
* **Rich Metadata Previews:** View song title, artist, album, and genre directly in the fzf preview window.
* **Interactive Selection with Multiple Modes:**
  * **Directory Mode:** Navigate folders with clean names instead of full paths.
  * **Track Mode:** Fuzzy-search individual tracks with metadata previews.
  * **Playlist Mode:** Find and play your saved .m3u or .pls playlists.
  * **Tag Filter Mode:** Drill down by genre, artist, album, or title interactively.
  * **Play All:** Instantly play your entire indexed library.
  * **URL Mode:** Paste YouTube or stream URLs directly from the menu.
* **Direct File/URL Playback:** Instantly play local audio/video files or URLs (YouTube, streams) without going through the menu.
* **Custom Directory Support:** Pass a folder path to browse and filter only that directory instead of your full library.
* **CLI Filtering:** Use flags like --genre, --artist, --album, --title for direct filtering. Pass a value or omit it to open an interactive picker.
* **Smart Matching:** CLI filters attempt exact matches first, then fall back to partial matches with disambiguation.
* **Configurable File Types:** Support for both audio and video extensions, easily tweakable.
* **Custom MPV Flags:** Pass mpv flags directly or set defaults in the config.
* **Video Toggle:** --video-ok lets you include videos in your library scans.
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
* **Windows (Native/Git Bash):** Not Supported.

> [!CAUTION]
> Native path handling (`C:\` vs `/`) prevents this from working. **Please use WSL.**

### Option 1: Quick Install (Recommended)

Run this command to install the latest stable release automatically.  
It will check dependencies and ask you where to install the script.

```bash
curl -sL https://raw.githubusercontent.com/FurqanHun/mpv-music/master/install.sh | bash
```

### Option 2: Manual Install

1. Download the latest `mpv-music` script from the [Releases page](https://github.com/FurqanHun/mpv-music/releases).

2. Make it executable:
```bash
chmod +x mpv-music
```

3. Move to your PATH:
```bash
mkdir -p ~/.local/bin
mv mpv-music ~/.local/bin/
```

### First run (setup):
```bash
mpv-music
```

> [!IMPORTANT]
> Running `mpv-music` for the first time will automatically index `$HOME/Music`.
> It is **recommended** that you first run `mpv-music --config` to customize your settings and music directories before indexing (unless you only keep your music in `$HOME/Music`).

That creates:
- `~/.config/mpv-music/mpv-music.conf`
- `~/.config/mpv-music/music_index.jsonl` (your indexed library)
- `~/.config/mpv-music/dirs_state.json` (directory state cache)

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
| `--video-ok` | Include video files in scans |
| `--ext=mp3,ogg` | Override file extensions |
| `--update` | Update the script to the latest version |
| `--reindex` | Force rebuild the full index |
| `--refresh-index` | Update index without wiping it |
| `-V, --verbose` | Increase verbosity, printing additional information |
| `--debug` | Print detailed debug messages |
| `-g, --genre [val]` | Filter by genre. Opens picker if no value given |
| `-a, --artist [val]` | Filter by artist. Opens picker if no value given |
| `-b, --album [val]` | Filter by album. Opens picker if no value given |
| `-t, --title [val]` | Filter by track title |
| `-p, --play-all` | Play all tracks matching filters directly |
| `-l, --playlist` | Go directly to playlist mode |

Any mpv flag also works: `--no-video`, `--volume=50`, `--shuffle`, etc.

When both `--verbose` and `--debug` are enabled together, logs will be saved to `~/.config/mpv-music/mpv-music.log`.
The log file automatically rotates after reaching the configured size (default: 1024KB).
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

### Maintaining:

* `--reindex` - Full rebuild
* `--refresh-index` - Smart update (only processes new/modified files)

> [!TIP]
> Run these flags alongside `--video-ok` to include video files in the index.
> Example: `mpv-music --video-ok --reindex`
> As of right now you may also have to run `--video-ok` when running `refresh-index`. I will expose the `VIDEO_OK` variable in the config file, in future.

---

## Configuration

If a config file does not exist, mpv-music will create one at startup. To customize the behavior, edit:

```
~/.config/mpv-music/mpv-music.conf
```

**Options:**

```bash
# mpv-music configuration

# Music Directories (Space-separated)
MUSIC_DIRS="$HOME/Music /mnt/media/audio"

# --- Visual Customization ---
# Banner text (displayed at start of track)
# Uses ANSI escape codes or simple text, which is directly passed to mpv
BANNER_TEXT='\n╔══  MPV-MUSIC  ══╗\n'

# Status Bar Logic (Complex MPV variables)
# Uses single quotes to prevent early expansion.
STATUS_MSG='▶ ${?metadata/artist:${metadata/artist} - }${?metadata/title:${metadata/title}}${!metadata/title:${filename}} • ${time-pos} / ${duration} • (${percent-pos}%)'

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
LOG_MAX_SIZE_KB=1024
```

---

## Development

This project is developed using a modular (more of a faux module) source structure.
* **Source Code:** Located in `src/`.
  * `01_vars.sh` contains config vars
  * `02_utils.sh` contains utility functions (i.e, logging, updater etc)
  * `03_config.sh` contains config file handling and dependency checks
  * `04_metadata.sh` contains metadata extraction and index building
  * `05_ui.sh` contains fzf UI and selection modes
  * `06_main.sh` contains main script logic and argument parsing
* **Building:** Run `./build.sh` to compile the modules into the final `mpv-music` executable.

> [!NOTE]
> Compiling here just means concatenating the source files into a single file (mpv-music) with proper shebang and permissions.

---

## License

MIT License. See [LICENSE](LICENSE).

---

## GenAI Disclosure

Generative AI (specifically Google Gemini, and sometimes others) was and is used for maintenance and development as an assistive tool.
