# mpv-music-indexer

[![indexer version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2FFurqanHun%2Fmpv-music%2Fmaster%2Fcrates%2Fmpv-music-indexer%2FCargo.toml&query=%24.package.version&label=version&color=blue)](https://github.com/FurqanHun/mpv-music/blob/master/crates/mpv-music-indexer/Cargo.toml)
[![mpv-music-indexer build status](https://github.com/FurqanHun/mpv-music/actions/workflows/indexer-build.yml/badge.svg)](https://github.com/FurqanHun/mpv-music/actions/workflows/indexer-build.yml)

The high-performance, multi-threaded metadata extraction engine for `mpv-music`.

Built with **Rust**, this binary replaces the legacy Bash/ffprobe indexing method. It is designed to scan thousands of audio files, extract metadata (ID3, Vorbis, etc.), and serialize them to JSON Lines in milliseconds.

## Features

* **Parallel Scanning:** Uses `rayon` to utilize all available CPU cores for directory traversal and metadata extraction.
* **HDD-Aware:** Includes a `--serial` flag to force single-threaded mode, preventing disk thrashing on mechanical hard drives.
* **Fast Metadata:** Leverages `lofty` for low-overhead tag reading (skips ffmpeg overhead).
* **Stdout Optimization:** Implements a "Buffer & Dump" locking strategy to ensure high-throughput JSON generation without thread contention.
* **JSON Lines Output:** Streams strictly formatted JSON objects to stdout for easy parsing by `jq` or other tools.
* **Binary Size:** Heavily optimized release profile (LTO enabled, symbols stripped, panic=abort).

## Installation

**Requirements:** Rust 1.85+ (Edition 2024).

### From Source

```bash
# Navigate to the crate directory
cd crates/mpv-music-indexer

# Build for release
cargo build --release

# The binary will be located at:
# target/release/mpv-music-indexer

```

## Usage

The indexer outputs JSON to `stdout` and progress information to `stderr`.

```bash
mpv-music-indexer [OPTIONS] <DIRECTORIES>...

```

### Arguments

| Flag | Description | Default |
| --- | --- | --- |
| `<DIRECTORIES>` | Space-separated list of paths to scan. | **Required** |
| `--video` | Include video files in the scan. | `false` |
| `--serial` | Force single-threaded mode (Recommended for HDDs). | `false` |
| `--audio-exts` | Comma-separated list of audio extensions. | `mp3,flac,wav...` |
| `--video-exts` | Comma-separated list of video extensions. | `mp4,mkv,webm...` |
| `--playlist-exts` | Comma-separated list of playlist extensions. | `m3u,m3u8,pls` |

### Examples

**Standard Parallel Scan (SSD):**

```bash
mpv-music-indexer ~/Music /mnt/External/Audio > music_index.jsonl

```

**Serial Scan (Mechanical HDD):**

```bash
mpv-music-indexer --serial /mnt/HDD/Music > music_index.jsonl

```

**Include Videos:**

```bash
mpv-music-indexer --video ~/Videos/MusicVideos > music_index.jsonl

```

## Output Format

The tool outputs one JSON object per line (JSONL).

```json
{"path":"/home/user/Music/Song.mp3","title":"Song Title","artist":"Artist Name","album":"Album Name","genre":"Rock","mtime":1706702400,"size":4096000,"media_type":"audio"}
{"path":"/home/user/Music/Video.mp4","title":"Video Title","artist":"UNKNOWN","album":"UNKNOWN","genre":"UNKNOWN","mtime":1706702500,"size":50960000,"media_type":"video"}

```

## Performance Optimizations

This crate uses a custom release profile in `Cargo.toml` to maximize speed and minimize binary size:

* **`lto = true`**: Link Time Optimization for cross-crate optimization.
* **`codegen-units = 1`**: Sacrifices build time for better runtime performance.
* **`panic = "abort"`**: Removes stack unwinding logic for smaller binaries.
* **`strip = true`**: Automatically removes debug symbols.

## Integration with mpv-music

This binary is designed to be called by the main `mpv-music` Bash script. The script automatically detects if this binary is present in the `$PATH` or config directory and prefers it over the legacy scanning method. But can be used as a standalone tool for indexing music files, or in other applications that require a JSONL output format.
