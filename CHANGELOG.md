# Changelog

All notable changes to furqanhun/mpv-music will be documented in this file.

## [v0.24.0-dev.14](https://github.com/FurqanHun/mpv-music/releases/tag/v0.24.0-dev.14) - 2026-02-10 (Pre-release)

### Features
- **Visual Playback Mode:** Introduced the `--watch` (`-w`) flag.
  - explicitly enables the MPV video window for playback.
  - forces MPV to run in "visual mode" with raw terminal output (so you can see buffering/connection logs) and an immediate window pop-up.
- **Video Logic Decoupling:** Separated "Indexing" from "Playback".
  - `yt-dlp` integration now respects `--watch` to fetch video streams instead of forcing `bestaudio`.
  - Added `--no-video` flag to forcefully override `video_ok=true` from your config (useful for temporarily hiding video files from search).
  - *Context:* This aligns behavior closer to the original Bash script, while offering new flexibility to toggle `--watch` or disable `--video-ok` per session.

### Refactoring
- **Audio-Only Polish:** The default audio-only mode now strictly enforces `--audio-display=no`. This prevents MPV from accidentally opening a window for cover art when you only wanted music.
- **Crates.io Prep:** Updated `Cargo.toml` with complete crate metadata (categories, keywords, repository links).

---

## [v0.24.0-dev.13](https://github.com/FurqanHun/mpv-music/releases/tag/v0.24.0-dev.13) - 2026-02-09 (Pre-release)

### Fixes
- **Updater Logic:** Fixed an edge case where development builds (e.g., `0.23.5-dev`) failed to identify the matching stable release (`0.23.5`) as a valid update.

### Refactoring
- **Version Comparison:** Replaced manual semantic version comparison logic with native Rust tuple comparison for improved reliability and readability.
- **Code Quality:** Refactored `player.rs` and `update.rs` to use guard clauses and flattened control flow, resolving multiple Clippy warnings.

---

## [v0.24.0-dev.12](https://github.com/FurqanHun/mpv-music/releases/tag/v0.24.0-dev.12) - 2026-02-09 (Pre-release)

### Added
- **Portable Deno Support:** The player now automatically searches for the `deno` binary in the same directory as the `yt-dlp` executable, improving compatibility for portable installations.

### Changed
- **Update Checker:** Fully implemented the `--update` CLI flag logic. (for now)
    - Feature is now optional and gated behind the `update` cargo feature (enabled in release builds).
    - Implements semantic version comparison (SemVer) to accurately detect updates.
    - Displays upgrade instructions (curl command and manual download link) if a new version is found.
    - Detects and handles development builds correctly to prevent false positives.
- **Logging:** Clarified the `yt-dlp` check message. It now explicitly states "Attempting self-update" instead of "health check" to better reflect that dependencies are being maintained.
- **CI/Build:** Enabled the `update` feature flag in release workflows, ensuring all distributed binaries ship with the self-update capability.

---

## [v0.24.0-dev.11](https://github.com/FurqanHun/mpv-music/releases/tag/v0.24.0-dev.11) - 2026-02-09 (Pre-release)

### Bug Fixes

- **Ctrl+C Cleanup Now Works in Release Builds:** Fixed race condition where `std::process::exit(0)` terminated the process before `Drop` implementations could run. Signal handler now uses `swap()` instead of `load()` to guarantee single-execution cleanup, ensuring queue files are deleted on interrupt in both debug and release modes.

### Technical Details

- The bug only manifested in optimized builds due to faster execution paths
- Handler now directly deletes temp files before calling `exit()` rather than relying on `Drop`
- `AtomicBool::swap()` prevents double-deletion between handler and `Drop` cleanup

---

## [v0.24.0-dev.10](https://github.com/FurqanHun/mpv-music/releases/tag/v0.24.0-dev.10) - 2026-02-09 (Pre-release)

### Performance

- **Zero-Cost Filtering:** Refactored tag filtering to use `Borrow<T>` generics, eliminating unnecessary cloning during searches. Filters now work with references until absolutely necessary, drastically reducing allocations for large libraries.

### Reliability

- **Multi-Instance Safety:** Temporary playlist files now use unique PID-based naming (`queue-{PID}.m3u8`), preventing conflicts when running multiple instances simultaneously.
- **Graceful Shutdown:** Queue files now clean up properly on Ctrl+C/SIGINT. No more stale `.m3u8` files littering your temp directory! 🧹
- **Smart File Recovery:** Enhanced the indexer with attribute-based tracking (size + mtime + filename). Moved or renamed files are now detected without re-probing metadata, making library reorganization instant.

### UX Improvements

- **YouTube Shorts Filter:** Search results now automatically exclude YouTube Shorts to improve music discovery quality.
- **Interactive Multi-Value Filters:** When passing comma-separated values to `-g`, `-a`, or `-b` flags, you now get an interactive picker to refine your selection before playing.

### Technical Details

- All changes maintain backward compatibility with existing indexes.
- The `Borrow<T>` refactor is particularly notable for big ah libraries.

---

## [0.24.0-dev.9](https://github.com/FurqanHun/mpv-music/releases/tag/0.24.0-dev.9) - 2026-02-08 (Pre-release)

## Features & Enhancements

- **Smart `yt-dlp` Nightly Detection:**
    * The app now detects if you are using a Nightly build of `yt-dlp`.
    * Intelligently disables legacy "EJS" workarounds when Nightly is detected to prevent conflicts.
    * Adds a helpful suggestion to upgrade if a Stable version is found.

## Fixes

- **Robust File Cleanup:**
    * Implemented a **RAII Guard** (`TempCleaner`) to ensure the temporary playlist file (`queue.m3u8`) is always deleted, even if MPV crashes or exits with an error.
- **CLI:** Fixed value handling for the `--yt` flag.

## Refactoring

- **Modular Architecture:**
    * Split the monolithic `main.rs` into a dedicated `cli.rs` and `tui` module.
    * Further separated the TUI into `tui/items.rs` (data structures) and `tui/mod.rs` (logic) for better maintainability.

---

## [0.24.0-dev.8](https://github.com/FurqanHun/mpv-music/releases/tag/0.24.0-dev.8) - 2026-02-08 (Pre-release)

## Fixes & Improvements

- **CLI Disambiguation Logic:**
    * Fixed a bug where the ambiguity picker ("Which artist did you mean?") blocked the `-p` / `--play-all` flag. It now correctly bypasses the menu if the flag is present.
    * Added **Multi-Selection** support to the disambiguation menu. You can now select multiple artists or genres using `TAB` before confirming.
    * Fixed matching logic for tags containing separators (e.g., `Artist A; Artist B`).

## Maintenance

- **Code Cleanup:** applied comprehensive `cargo clippy` fixes and optimizations.
- **Refactor:** introduced `run_skim_multi_selection` helper in `main.rs`.

---

## [0.24.0-dev.7](https://github.com/FurqanHun/mpv-music/releases/tag/0.24.0-dev.7) - 2026-02-06 (Pre-release)

### Changes
- **Forgiving TUI Shortcuts**: Flags `-l`, `-t`, `-g`, `-a`, and `-b` now open the TUI picker automatically if no value is passed.
- **Instant Play**: If a filter results in exactly one match, the prompt is bypassed and playback starts immediately.
- **Ad-hoc Sessions**: Implemented session-based directory browsing for arbitrary paths.
- **Playback Flags**: Added `--loop`, `--no-loop`, `--repeat`, and `--ext` overrides.
- **Installer & Updater**: Updated logic for Rust binaries, architecture detection, and archival notices for legacy users.

And saner defaults :) you no longer get my settings as defaults.

---

## [0.24.0-dev.6](https://github.com/FurqanHun/mpv-music/releases/tag/0.24.0-dev.6) - 2026-02-05 (Pre-release)

This is a **Development Preview** of the upcoming Rust-native version of mpv-music. We have officially swapped the legacy Bash hybrid engine for a high-performance Rust core. This dev build is intended for testing the new architecture, TUI responsiveness, and the new indexing logic.

###  Major Improvements (Dev-Preview)
- **Single Unified Binary:** No more separate indexer and script. The player and the scanner are now one efficient, statically-linked executable.
- **Integrated Rust TUI:** Replaced external `fzf` with a native `skim` interface. Expec integrated metadata previews, and a more responsive UI.
- **MUSL Portability:** This build is statically linked with `musl`, meaning it should run on almost any Linux distro without GLIBC version conflicts.
- **XDG Compliance:** Now properly utilizes standard system paths for a cleaner `$HOME`:
    - **Config:** `~/.config/mpv-music/`
    - **Index/Data:** `~/.local/share/mpv-music/`
    - **Logs:** `~/.local/share/mpv-music/`

###  New Features to Test
- **In-App Settings:** Manage your music directories and configuration directly from the TUI menu.
- **Revised Logging:** Added a cleaner file-logging system that overwrites by default to save space.
- **CLI Filtering:** Test the new `-a` (artist), `-g` (genre), and `-b` (album) flags for direct filtering. (new as in implementation is a bit different)
- **Search & Stream URL:** Now you can search on youtube too :)

###  Technical Notes
* **Dev Build Status:** Some legacy Bash flags may still be missing or in progress. Use `mpv-music --help` to see currently implemented features.
* **Config Change:** Your old `.conf` files are incompatible with this version. A fresh `config.toml` will be generated on your first run. Also the config defaultts aren'tt sane yet... Imma change i later on (for now you get my config as default ig)
* **MSRV:** Requires Rust **1.93.0+** if building from source.

### Testing the Build
Download the binary's tar ball/archive for your architecture, extract it, make it executable (`chmod +x`) (you don't really have to but still as a precaution), and run it. Please report any crashes or TUI glitches on the issue tracker!


---

## [v0.23.5](https://github.com/FurqanHun/mpv-music/releases/tag/v0.23.5) - 2026-02-03

This patch resolves iterator invalidation, updater logic, and missing flags

- **CLI Parsing:** Fixed a critical "Iterator Invalidation" (positional desync) bug in the pre-flight check by switching to a robust `while` loop. Flags like `-p` before `--config` no longer crash the script.
- **Updater:** Fixed `invoke_updater` to correctly handle `dev`/`stable` arguments without forcing a default, allowing the updater script to auto-detect channels properly.
- **UX:** Added support for plural flags `--add-dirs` and `--remove-dirs`.

---

## [v0.23.5-dev](https://github.com/FurqanHun/mpv-music/releases/tag/v0.23.5-dev) - 2026-02-03 (Pre-release)

- Fixed updater and it's call in `invoke_updater()`, plus `--update` for multi channel.

---

## [v0.23.4-dev](https://github.com/FurqanHun/mpv-music/releases/tag/v0.23.4-dev) - 2026-02-03 (Pre-release)

- The reason why i switched from for loop to while loop was this, as argument handling is done better with while loops, but ofc i messed it up somehow then reverted back to for loop in v0.23.3. This seems to have fixed it but imma test it first later on.

---

## [v0.23.3](https://github.com/FurqanHun/mpv-music/releases/tag/v0.23.3) - 2026-02-03

- Fixed the bug caused by switching to while loop in preflight, where for example in `mpv-music -t angel` angel was being dropped, i just switched back to for loop cause i don't wanna think about it rn

---

## [v0.23.2](https://github.com/FurqanHun/mpv-music/releases/tag/v0.23.2) - 2026-02-02

This patch refactors the initialization logic to fix logging leaks and argument parsing bugs.

### Fixes & Improvements
- **Zombie Logs Killed:** The configuration file is now sourced **before** argument parsing. This ensures settings like `LOG_MAX_SIZE_KB=0` apply immediately, preventing the script from creating unwanted log files during operations like `--remove-log` or `--config`.
- **Refactored Parsing:** Switched the pre-flight check from a `for` loop to a `while` loop to correctly handle multi-part flags.
- **Black Hole Fix:** Implemented a pass-through mechanism to strictly preserve unknown arguments (like `-a`, `-p`, URLs) during the pre-flight check, ensuring they reach the main execution loop intact. This shit was a side affect of using switching to while loop.

---

## [v0.23.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.23.1) - 2026-02-02

This patch restores functionality that i forgot i removed while refactoring code to add `resolve_editor()` smh

### Fixes
- **CLI:** Restored the `--remove-log` (alias `--rm-log`) flag to allow users to delete the log file directly from the command line, maintaining symmetry with `--remove-config`.

---

## [v0.23.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.23.0) - 2026-02-02

This milestone release marks the transition to Stable, introducing a dual-channel update system, granular playback controls, and integrated management tools for logs and configuration.

## Features
- **Stable Channel:** The installer and updater now default to the Stable channel for reliability, with a dedicated `--dev` flag for accessing bleeding-edge pre-releases
- **Smart Playback Control:** Exposed `SHUFFLE` and `LOOP_MODE` variables in `mpv-music.conf`, allowing users to define default behavior without modifying complex argument arrays
- **Management Tools:** Added `--log` and `--config` flags with intelligent editor resolution (detecting `nvim`, `nano`, `code`, etc.) for instant access to logs and settings
- **CLI Overrides:** Implemented a priority-based argument injection system ("Sandwich Strategy"), ensuring CLI flags like `--no-shuffle` or `--loop-playlist=5` always override config defaults

## Improvements
- **Channel Switching:** The updater now detects Stable/Dev channel switches and triggers a safety protocol to wipe incompatible configurations before installing
- **Editor Resolution:** Unified logic for opening files, prioritizing read-only pagers (`less`) for logs while respecting the user's `$EDITOR` for configuration

---

## [v0.22.3](https://github.com/FurqanHun/mpv-music/releases/tag/v0.22.3) - 2026-02-02 (Pre-release)

This patch release focuses on developer tooling and stability, completely overhauling the debug infrastructure and fixing edge-case crashes during setup.

### Ehancements
The `--debug` flag has been transformed from a silent toggle into a full system tracer:

- **Full Visibility:** Now enables Bash tracing (`set -x`) and MPV internal hooks (`ytdl_hook=trace`) for granular execution details.
- **System Diagnostics:** Automatically dumps OS, MPV, FFmpeg, and yt-dlp versions on startup to aid troubleshooting.
- **Log Preservation:** Modified MPV arguments to prevent screen clearing in debug mode, ensuring logs remain visible in the terminal.

### Fixes
- **Config Initialization:** Fixed a crash where running `mpv-music --config` would fail if the configuration file did not already exist. The configuration generator is now prioritized in the boot sequence.
- **Documentation:** Updated `README.md` and CLI help (`--help`) to accurately reflect new dependency configurations and debug behaviors.
-  **Logic Fixes:** Now each verbose/messages runs the `rotate_log()` before writing to log file.

---

## [v0.22.2](https://github.com/FurqanHun/mpv-music/releases/tag/v0.22.2) - 2026-02-02 (Pre-release)

This patch ensures compatibility with package-managed `yt-dlp` installations by handling missing components and resolves critical API rate limits for the project website.

## Features
- **EJS Remote Fix:** Added `YTDLP_EJS_REMOTE_GITHUB` config option (default: `false`) to fetch missing EJS runtimes on demand, fixing playback for `apt`/`dnf` installed versions that lack internal components
- **Proactive Diagnostics:** The crash handler now specifically detects errors caused by missing runtimes and advises the user to enable the remote fix

## Fixes
- **Docs Stability:** Replaced client-side GitHub API calls with static JSON generation (`latest.json`), permanently eliminating "API Rate Limit Exceeded" errors for the documentation site

---

## [v0.22.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.22.1) - 2026-02-01 (Pre-release)

This patch resurrects direct playback functionality by fixing a critical execution oversight, implements a smart crash handler for outdated dependencies, and introduces utilities for rapid configuration resets.

## Features
- **Smart Crash Handler:** If URL playback fails, the script now automatically checks for outdated `yt-dlp` versions—the primary cause of streaming errors—and alerts the user immediately
- **Config Reset:** Introduced `--remove-config` (and `--rm-conf`) to safely delete the existing configuration file, allowing for a clean reset to defaults on the next launch

## Fixes
- **Direct Playback:** Resolved a critical regression where the script would correctly prepare all MPV arguments for direct URLs/files but exit before actually executing the command

---

## [v0.22.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.22.0) - 2026-01-31 (Pre-release)

This release unlocks the full potential of the Rust indexer with multi-threaded parallel scanning, adds hardware-aware controls for HDD users, and exposes comprehensive configuration options for volume and video playback.

## Features
- **Parallel Indexing:** The Rust indexer now utilizes all available CPU cores via `rayon`, delivering massive speed gains for SSD users
- **Serial Mode:** Added `--serial` flag (and `SERIAL_MODE` config) to force single-threaded indexing, preventing thrashing on mechanical drives
- **Volume Control:** Introduced `--volume` / `--vol` flag and `VOLUME` config setting to manage playback levels directly
- **Persistent Settings:** Users can now permanently enable video scanning (`VIDEO_OK`) and serial mode via the config file

## Improvements
- **Lock Contention Fix:** Optimized the Rust indexer's output strategy to buffer JSON serialization, eliminating thread blocking and maximizing throughput
- **Argument Parsing:** Updated the wrapper script to correctly prioritize Config settings while allowing CLI overrides for all new flags

---

## [v0.21.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.21.0) - 2026-01-30 (Pre-release)

This release introduces a high-performance Rust-based indexer for near-instant library scanning and refactors the project structure for better maintainability.

## Features
- **Rust Indexer:** Implemented a high-speed, multi-threaded indexing binary (`mpv-music-indexer`) that replaces the legacy Bash loop
- **Hybrid Support:** Added fallback logic to seamlessly switch between the Rust indexer and the legacy Bash implementation if the binary is missing

## Improvements
- **Installer:** Updated installation logic to detect `x86_64` architecture and offer the pre-compiled Rust binary from GitHub Releases
- **Optimization:** Enabled aggressive build optimizations (LTO, stripping) and streaming metadata retrieval for the Rust binary
- **Refactor:** Restructured the monorepo by moving Rust code to `crates/` and modularizing the metadata script into distinct components

---

## [v0.20.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.20.1) - 2026-01-29 (Pre-release)

This patch release optimizes the indexing engine by removing dead code and refines the installer for better usability.

## Improvements
- **Installer:** Implemented batch directory addition, allowing multiple folders to be queued during setup before applying changes
- **Optimization:** Removed unused `dirs_state` logic and `dirs_state.json` generation, reducing unnecessary file I/O and CPU cycles during scans

## Fixes
- Fixed path sanitization in the installer to properly handle drag-and-drop quotes without breaking paths containing apostrophes

---

## [v0.20.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.20.0) - 2026-01-29 (Pre-release)

This release introduces interactive library management and completely rewrites the configuration engine to support complex paths.

## Features
- Implemented `--manage-dirs`, `--add-dir`, and `--remove-dir` CLI flags for direct library management
- Added interactive "Manage Directories" menu to add/remove folders directly from the UI
- Installer now intelligently prompts to add music directories during setup

## Core Improvements
- Refactored `MUSIC_DIRS` persistence to use native Bash arrays, fully supporting paths with spaces
- Implemented auto-migration in the updater to upgrade legacy string-based configs to the new array format
- Implemented "nuclear" save strategy to ensure config updates are appended safely without corruption

## Fixes
- Refactored config reload logic to ensure `build_music_index` always sees the latest state (fixed memory/disk desync)
- Fixed updater to properly source configuration and respect `LOG_MAX_SIZE_KB` settings
- Updated README with new usage instructions and installation steps

---

## [v0.19.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.19.1) - 2026-01-28 (Pre-release)

- Fixed an issue where the temporary file cleanup routine ignored the log size configuration; it now strictly adheres to the limit (or disabled state) defined in the config.

---

## [v0.19.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.19.0) - 2026-01-28 (Pre-release)

## Enhancements
- Improved debug and verbose logging system with timestamped file logging
- Implemented improved self-healing for index corruption

## Performance
- Optimized indexing using pure Bash and `find -printf` to remove process overhead
- Optimized index refresh using dual-stream reader to reduce memory usage during updates

---

## [v0.18.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.18.1) - 2026-01-28 (Pre-release)

- Fixed video indexing lag `--video-ok` by limiting ffprobe duration/size in `get_audio_metadata`. As a result, general indexing `--reindex`,`--refresh-index` is now faster too.
- Wrapped interactive menu in a while loop to prevent accidental exits on invalid input.

Honorary mention:
- Added clean exit trap for SIGINT (Ctrl+C). (didn't really mentioned in commits so yeah)

---

## [v0.18.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.18.0) - 2026-01-27 (Pre-release)

- Automatically validates index integrity on startup. It detects corruption (e.g., from power loss), tries to repair by removing the last line, or triggers a smart rebuild to prevent crashes. In a blink of an eye.

---

## [v0.17.3](https://github.com/FurqanHun/mpv-music/releases/tag/v0.17.3) - 2026-01-26 (Pre-release)

fix: improve stability for large libraries and weird filenames
- Use piping for large playlists (avoids arg limit crash)
- Preserve newlines/spaces in filenames (removed tr -d)
- Allow GNU Compatible find (feature check instead of version check)

---

## [v0.17.2](https://github.com/FurqanHun/mpv-music/releases/tag/v0.17.2) - 2026-01-26 (Pre-release)

## Bug Fixes
- Fixed script exiting early (it still cleared temp files, but failed to print warn messages) when pressing ESC in interactive menus (fzf exit 130)
- Fixed argument parsing to handle spaces in filters correctly (replaced xargs with sed)
- Fixed regex filter escaping to prevent jq errors on special metadata

## Enhancements
- Overhauled UI with contextual emojis for tracks, videos, folders, and tags
- Implemented safer line-number based selection strategy for previews (fixes special char handling)
- Cleaned up fzf list views to hide raw data columns while preserving rich previews
- Removed unnecessary locale settings from clarification logic

---

## [v0.17.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.17.0) - 2026-01-26 (Pre-release)

## Bug Fixes
- Fixed empty fzf selection handling in dir mode, playlist mode, and CLI filter clarification
- Fixed CLI filters using MUSIC_INDEX_FILE instead of INDEX_TO_USE
- Fixed interactive filters running before custom directory index was built
- Fixed play_all_music using wrong index for custom directories
- Fixed empty clarification fzf selection not being handled
- Fixed indentation for playlist media_type in update_music_index

## Enhancements
- Added deferred interactive filter system with GENRE/ARTIST/ALBUM/TITLE_INTERACTIVE flags
- Removed redundant empty array checks that became dead code

---

## [v0.16.2](https://github.com/FurqanHun/mpv-music/releases/tag/v0.16.2) - 2026-01-25 (Pre-release)

- **Fix:** Implemented a pre-flight check to prevent startup crashes caused by syntax errors in `mpv-music.conf`.
- **Refactor:** Centralized `--config`, `--update`, and `--version` flag handling in the configuration module.

---

## [v0.16.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.16.1) - 2026-01-25 (Pre-release)

- someone wanted it so here it is, fixed the radio/mixes in yt, by forcing yes-playlist if list= detected in url

---

## [v0.16.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.16.0) - 2026-01-25 (Pre-release)

## What's New

- **Interactive URL Mode (Option 6):** Introduced a dedicated input mode for URLs. This bypasses terminal parsing issues where links containing special characters (like `&` in YouTube mixes) would break shell commands.
- **Multi-URL Support:** Added the ability to pass multiple URLs or file paths as arguments (or via the new URL mode) to queue them sequentially.

## Fixes & Improvements

- **Critical YouTube Playback Fix (JS Runtime):** Implemented automatic detection for JavaScript runtimes (Deno, Node.js, QuickJS, Bun).
  - *Context:* YouTube has updated their delivery methods (forcing SABR format), which now requires `yt-dlp` to execute JavaScript to bypass anti-bot protections. Without a valid runtime, playback was failing with `HTTP 403 Forbidden` errors. This update injects the necessary flags to ensure reliable streaming. (I personally haven't used yt-dlp in ages but knew about SABR format being forced, but didn't think desktop had started to do it too)

- **Improved Metadata Display:** Changed the playback status fallback from `filename` to `media-title`. Streams will now display the actual video title (e.g., "Artist - Song") instead of the raw URL string. This doesn't affect offline media that doesn't have proper media title tags, as it will automatically fall back to the filename.

---

## [v0.15.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.15.1) - 2026-01-24 (Pre-release)

exactly what the title says, when the script was ran with `--reindex` or `--refresh-index` and the script detected the index missing or it would first create the auto index, and then run the command to reindex or refresh it. so i added a check to avoid that

---

## [v0.15.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.15.0) - 2026-01-24 (Pre-release)

switched the index from json to jsonl for better performance and scalability

---

## [v0.14.6](https://github.com/FurqanHun/mpv-music/releases/tag/v0.14.6) - 2026-01-21 (Pre-release)

- Improved the progress bar logic in indexing.
- Demoted some info messages
- Updater no longer downgrades, aka added semantic version check

---

## [v0.14.5](https://github.com/FurqanHun/mpv-music/releases/tag/v0.14.5) - 2026-01-21 (Pre-release)

- Updated the generated configuration example to correctly reflect the new array syntax.
- Standardized logging with clean tags and reduced terminal noise.
- Switched fzf previews to TSV format to prevent breakage on filenames containing pipe characters.
- Unified interactive prompts and improved interrupt handling across CLI and Menu modes.

---

## [v0.14.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.14.1) - 2026-01-18 (Pre-release)

while building v0.14.0 through the build.sh it uhm ran the commands in the mpv default args, which then ofc broke the script, and this fixes the issue by dividing it into more vars and then merging in create config, while also making them string literal.

---

## [v0.14.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.14.0) - 2026-01-18 (Pre-release)

enforce minimal TUI, silence spam, and fix config parsing

---

## [v0.13.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.13.0) - 2026-01-18 (Pre-release)

split the file into modules for easier development/maintenance, however it doesn't effect the user

---

## [v0.12.3](https://github.com/FurqanHun/mpv-music/releases/tag/v0.12.3) - 2026-01-17 (Pre-release)

changed parsing from comma to tab separator, and updated the updater logic to use tagged release instead of master branch

---

## [v0.12.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.12.1) - 2026-01-03 (Pre-release)

Adds `--update`. This fetches a separate` mpv-music-updater` script to handle the upgrade.

This approach ensures future architectural changes (like modularization) can be handled smoothly without breaking existing installations, and avoids race conditions caused by modifying an executing file.

---

## [v0.11.3](https://github.com/FurqanHun/mpv-music/releases/tag/v0.11.3) - 2025-08-14 (Pre-release)

- Now handles `--help`, `--version`, and `--config` before main execution or index logic.
- Fixes the bug/regression introduced in v0.11.2 where some idiot tried halfassing the fix and caused regression with CLI filters.
- Restores correct CLI filter behavior for `--genre`, `--artist`, `--album`, `--title`, etc.
- Argument parsing and index setup order are now robust for all entry points.


---

## [v0.11.2](https://github.com/FurqanHun/mpv-music/releases/tag/v0.11.2) - 2025-08-13 (Pre-release)

- Fixed a bug where running `mpv-music --config` would incorrectly build an empty music index, even though only config editing was intended
- Now, `--config` and `--config=EDITOR` will only open the config file and exit—no more index creation, no find errors, just a clean setup experience
- Set `--audio-display=no` as a default mpv flag so embedded cover art is never shown or rendered, even with `--no-video`

No changes to other features. 

---

## [v0.11.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.11.0) - 2025-08-13 (Pre-release)

##  Major Upgrade!

- **Tag Filtering:** Filter music by genre, artist, album, or title via interactive menu or CLI flags.
- **Play All Mode:** Instantly play all tracks, or all tracks matching your filters.
- **Improved Playlist Mode:** Now uses indexed playlists for faster searches and selection.
- **New CLI Flags & Expanded Menu:** Access new features via CLI (`--genre`, `--artist`, `--album`, `--title`, `--play-all`, `--playlist`) and interactive menu.
- **Enhanced Logging, Error Handling, and UX:** 
  - Verbose/debug modes, log rotation, configurable log file size.
  - Better error messages and more polish throughout.
- **Help Text & Code Cleanup:** Help message updated for all features, code and comments cleaned up.

**Note:**  
- **Recommended:** Rebuild your music index after updating (`mpv-music --reindex`).
- The configuration file now requires this line for logging.  
  If set to 0, it won't store any logs (but will still display them if `--debug` is passed):
  ```sh
  LOG_MAX_SIZE_KB=1024
  ```
  If upgrading, add this line to your config file, or delete your config and let the script regenerate it.


---

## [v0.10.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.10.0) - 2025-08-11 (Pre-release)

## Highlights
- **Playlist Mode:** Select and play playlists (`.m3u`, `.m3u8`, `.pls`) directly from the interactive menu.
- **Much faster indexing:** Dramatic speed improvements for large music libraries.
- **Metadata extraction:** Now uses a single, efficient `jq` call for all tags.
- **Cleaner codebase:** Unified extension filter logic and improved maintainability.
- **Updated help output and UI:** Consistent interface across all modes.

---

**Note:**  
- It is recommended to rebuild your music index after updating.
- The configuration file now requires this line for playlist extensions:
  ```sh
  PLAYLIST_EXTS="m3u m3u8 pls"
  ```
  If upgrading, add this line to your config file, or remove your config and let the script regenerate it.

---

## [v0.9.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.9.0) - 2025-06-29 (Pre-release)

feat: Enhance script robustness with temp file management and logging for v0.9.0

- Fix Album/Folder mode to utilize index
- Fix both modes to utilize temporary indexing for custom dirs
- Add centralized temporary file management with proper cleanup
- Implement verbose (-V) and debug logging capabilities
- Add log file rotation when size exceeds 1MB
- Improve signal handling with comprehensive traps
- Update help documentation to describe new options

---

## [v0.8.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.8.0) - 2025-06-29 (Pre-release)

feat(config): add --config flag to edit configuration
- Add `--config` flag to open the config in **nano/vi**
- Support custom editor with `--config=EDITOR` syntax
- Update help text to document new functionality
- Fix function order by moving `create_config()` definition earlier


---

## [v0.7.2](https://github.com/FurqanHun/mpv-music/releases/tag/v0.7.2) - 2025-06-29 (Pre-release)

fix(mode2): restore custom directory support in track mode
- Fix regression in track mode where custom directories were ignored
- Implement temporary indexing for custom directories in track mode
- Maintain same preview interface for consistency

---

## [v0.7.1](https://github.com/FurqanHun/mpv-music/releases/tag/v0.7.1) - 2025-06-29 (Pre-release)

feat(ui): add media type indicators to track listing

- Display icons for audio (🎵) and video (🎬) files in track selection mode
- Add media_type field to index file during build and update operations
- Update track selection mode to show file type in preview panel
- Require reindex for users upgrading from previous versions

---

## [v0.7.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.7.0) - 2025-06-28 (Pre-release)

This milestone release consolidates all work since v0.4.0.

- Implements a new JSON-based indexing engine for massive performance gains, especially on large libraries.
- Overhauls the fzf interface to use the index, providing rich metadata previews for tracks and clean names for folders.
- Stabilizes the indexing system, making `--reindex` and `--refresh-index` reliable.
- Adds `jq` and `ffprobe` as required dependencies.
- Adds `mediainfo` as optional dependency.

---

## [v0.4.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.4.0) - 2025-06-28 (Pre-release)

- Introduced direct playback for local files and URLs as command-line arguments.
- Ensured MPV arguments from the command line are correctly applied to direct playback.
- Refactored argument parsing into a unified block for improved robustness and clarity.
- Added a warning if `yt-dlp` is not found for full URL support.

---

## [v0.3.0](https://github.com/FurqanHun/mpv-music/releases/tag/v0.3.0) - 2025-06-27 (Pre-release)

- Externalized music directories, MPV arguments, and extensions to config file.
- Added comments to improve script readability and documentation within config file.
- Introduced automatic default config file creation on first run.
- Improved 'find' command usage by introducing EFFECTIVE_MUSIC_DIRS for precedence.



---

