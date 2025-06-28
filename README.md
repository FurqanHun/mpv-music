# mpv-music
A blazing-fast MPV wrapper for music playback, featuring fuzzy search, metadata-rich previews, direct playback, and full config customization.

---

## Table of Contents

* [Features](#features)
* [Dependencies](#dependencies)
* [Installation](#installation)
* [Usage](#usage)
* [Indexing](#indexing)
* [Configuration](#configuration)
* [License](#license)

---

## Features

* **Blazing-Fast Indexed Searching:** Automatically indexes your music library into a JSON file for lightning-fast search using `fzf`. (If the `music_index.json` doesn't exist, else load the existing index.)
* **Rich Metadata Previews:** In track mode, view song title, artist, album, and genre directly in the `fzf` preview window.
* **Interactive Selection (fzf) -- Two Playback Modes:**
  * **Track Mode:** Fuzzy-search individual tracks with metadata previews.
  * **Album Mode:** Navigate folders with clean names instead of full paths.
* **Direct File/URL Playback:** Instantly play local audio/video files or URLs (e.g., YouTube) without going through the menu.
* **Configurable File Types:** Support for both audio and video extensions — easily tweakable.
* **Custom MPV Flags:** Pass `mpv` flags directly or set defaults in the config.
* **Video Toggle:** `--video-ok` lets you include videos in your library without playing visuals unless you want to.

---

## Dependencies

#### Required:
* **`mpv`** – [https://mpv.io](https://mpv.io)
* **`fzf`** – [https://github.com/junegunn/fzf](https://github.com/junegunn/fzf)
* **`jq`** – for parsing the index.
  🧪 `sudo apt install jq` or `brew install jq`
* **`ffmpeg`** – `ffprobe` is used to extract metadata.
  🎵 [https://ffmpeg.org](https://ffmpeg.org)
* **GNU `find`** – not BSD `find`, script checks this at startup.

#### Optional (but recommended):
* **`yt-dlp`** – for playing URLs.
  🔗 [https://github.com/yt-dlp/yt-dlp](https://github.com/yt-dlp/yt-dlp)
* **`mediainfo`** – fallback metadata reader.
  🧾 `sudo apt install mediainfo` or `brew install mediainfo`

---

## Installation

1. **Clone the repo:**
    ```bash
    git clone https://github.com/FurqanHun/mpv-music.git
    cd mpv-music
    ```

2. **Make it executable:**
    ```bash
    chmod +x mpv-music
    ```

3. **Move to your PATH:**
    ```bash
    mv mpv-music ~/.local/bin/ # Or /usr/local/bin/
    ```

4. **First run (setup):**
    ```bash
    mpv-music
    ```
    That creates:
    - `~/.config/mpv-music/mpv-music.conf`
    - `~/.config/mpv-music/music_index.json` (your indexed library)

---

## Usage

```bash
mpv-music [PATH_OR_URL_OR_DIR] [OPTIONS]
```

### Arguments:

* **No args:** Runs interactive selection on your configured music directories.
* **File or URL:** Plays it instantly.
* **Folder path:** Runs interactive search using just that folder.

### Options:

* `--no-video`, `--volume=50`, etc → any `mpv` flag works
* `--video-ok` → include video files
* `--ext=mp3,ogg` → override file extensions
* `--reindex` → force rebuild the full index
* `--refresh-index` → update index without wiping it

### Examples:

```bash
mpv-music                            # interactive mode (track/folder)
mpv-music /path/to/music            # interactive mode in specific folder
mpv-music ~/Music/track.flac        # plays file instantly
mpv-music https://yt.link/video     # plays URL instantly
mpv-music --shuffle --no-video      # with custom mpv flags
mpv-music --reindex                 # rebuild the index from scratch
```

---

## Indexing

Your music library is indexed to:

```
~/.config/mpv-music/music_index.json
```

### Why?

Searching the filesystem with `find` every time is **slow af**, specially if you have a large music collection. So `mpv-music` caches an index for:
- fast filtering
- metadata previews
- offline-friendly behavior (may introduce caching for URLs in future)

### Maintaining:

* `--reindex` → Full rebuild
* `--refresh-index` → Smart update

---

## Configuration
If a config file doesn't exist, `mpv-music` will create one at startup. If you want to customize the behavior, edit this:

```
~/.config/mpv-music/mpv-music.conf
```

**Options:**

```bash
# mpv-music configuration

MUSIC_DIRS="$HOME/Music /mnt/media/audio"
MPV_DEFAULT_ARGS="--loop-playlist=inf --shuffle --no-video --volume=50"
AUDIO_EXTS="mp3 flac wav m4a aac ogg opus"
VIDEO_EXTS="mp4 mkv webm avi"
```

---

## License

MIT License — do whatever the hell you want.
See [LICENSE](LICENSE) for formal stuff.

---
