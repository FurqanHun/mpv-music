use crate::config::{Config, LoopMode};
use crate::tui::Icons;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

struct TempCleaner {
    path: std::path::PathBuf,
    running: Arc<AtomicBool>,
}

impl Drop for TempCleaner {
    fn drop(&mut self) {
        if self.running.swap(false, Ordering::SeqCst) && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
            log::debug!("Cleaned up temporary file: {:?}", self.path);
        }
    }
}

struct IpcCleaner {
    path: Option<String>,
}

impl Drop for IpcCleaner {
    fn drop(&mut self) {
        if let Some(ref path) = self.path {
            let p = std::path::Path::new(path);
            if p.exists() {
                let _ = std::fs::remove_file(p);
                log::debug!("Cleaned up IPC socket: {}", path);
            }
        }
    }
}

pub fn play(target: &str, config: &Config, extra_args: &[String]) -> Result<()> {
    log::info!("Preparing playback for target: {}", target);

    let cmd_name = config.player_bin();
    let mut cmd = Command::new(cmd_name);

    apply_common_args(&mut cmd, config, extra_args);

    let optimization_target = if let Some(inner_url) = inspect_playlist_content(target, config) {
        log::debug!(
            "Playlist content scan found network link. optimizing for: {}",
            inner_url
        );
        inner_url
    } else {
        target.to_string()
    };

    apply_url_optimizations(&mut cmd, &optimization_target, config);

    let socket_to_clean = handle_radio_sync(&mut cmd, target, config);

    let _ipc_guard = IpcCleaner {
        path: socket_to_clean.clone(),
    };
    let ipc_handler = socket_to_clean.clone();
    ctrlc::set_handler(move || {
        log::info!("\nReceived Ctrl+C.");
        if let Some(ref path) = ipc_handler {
            let p = std::path::Path::new(path);
            if p.exists() {
                let _ = std::fs::remove_file(p);
                log::debug!("Cleaned up IPC socket: {}", path);
            }
        }
        std::process::exit(0);
    })
    .ok();

    cmd.arg(target);

    log::debug!("Exec: {:?}", cmd);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to launch player '{}'", cmd_name))?;

    if !status.success() && classify_target(&optimization_target).is_network() {
        let ytdlp_cmd = config.ytdlp_bin();
        log::error!(
            "Player '{}' process exited with error status. Checking {} health...",
            cmd_name,
            ytdlp_cmd
        );
        check_ytdlp_status(ytdlp_cmd);
    }

    Ok(())
}

pub fn play_files(paths: &[String], config: &Config, extra_args: &[String]) -> Result<()> {
    if paths.is_empty() {
        log::debug!("play_files called with empty path list, skipping");
        return Ok(());
    }

    log::info!("Preparing playback for {} files", paths.len());
    let cmd_name = config.player_bin();
    let mut cmd = Command::new(cmd_name);

    apply_common_args(&mut cmd, config, extra_args);

    // O(N) Single-Pass Scan: Find the item with the highest requirement.
    let mut best_target = paths.first();
    let mut max_kind = TargetKind::LocalFile;

    for path in paths {
        let kind = classify_target(path);
        if kind > max_kind {
            max_kind = kind;
            best_target = Some(path);
            if max_kind == TargetKind::YouTube {
                break; // found yt, stop scanning
            }
        }
    }

    let socket_to_clean = if let Some(target) = best_target {
        log::debug!("Configuring mpv based on representative track: {}", target);
        apply_url_optimizations(&mut cmd, target, config);
        handle_radio_sync(&mut cmd, target, config) // No semicolon here!
    } else {
        None
    };

    let _ipc_guard = IpcCleaner {
        path: socket_to_clean.clone(),
    };

    let dirs = ProjectDirs::from("com", "furqanhun", "mpv-music")
        .context("Could not determine data directory")?;
    let data_dir = dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;

    let pid = std::process::id();
    let queue_path = data_dir.join(format!("queue_{}.m3u8", pid));

    {
        let mut file = std::fs::File::create(&queue_path)
            .context("Failed to create temporary playlist file")?;

        writeln!(file, "#EXTM3U")?;
        for path in paths {
            writeln!(file, "{}", path)?;
        }
    }

    let running = Arc::new(AtomicBool::new(true));
    let r_handler = running.clone();
    let p_handler = queue_path.clone();
    let ipc_handler = socket_to_clean.clone();

    // Register signal handler
    ctrlc::set_handler(move || {
        if r_handler.swap(false, Ordering::SeqCst) && p_handler.exists() {
            let _ = std::fs::remove_file(&p_handler);
            log::info!("\nReceived Ctrl+C. Cleaned up queue file.");
        }

        if let Some(ref path) = ipc_handler {
            let p = std::path::Path::new(path);
            if p.exists() {
                let _ = std::fs::remove_file(p);
                log::info!("Cleaned up IPC socket on Ctrl+C.");
            }
        }
        std::process::exit(0);
    })
    .ok();

    let _cleaner = TempCleaner {
        path: queue_path.clone(),
        running: running.clone(),
    };

    log::info!("Generated unique playlist at {:?}", queue_path);

    // pass the file to MPV
    cmd.arg(format!("--playlist={}", queue_path.to_string_lossy()));

    log::info!("Launching player '{}' for playlist playback...", cmd_name);
    log::debug!("Exec: {:?}", cmd);

    // blocks until mpv closes
    cmd.status()
        .with_context(|| format!("Failed to launch player '{}' for playlist", cmd_name))?;

    Ok(())
}

// helpers

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetKind {
    LocalFile = 0,
    GenericUrl = 1,
    YouTube = 2,
}

impl TargetKind {
    pub fn is_network(&self) -> bool {
        *self >= TargetKind::GenericUrl
    }

    pub fn is_youtube(&self) -> bool {
        *self == TargetKind::YouTube
    }
}

pub fn classify_target(s: &str) -> TargetKind {
    if s.contains("youtube.com") || s.contains("youtu.be") {
        TargetKind::YouTube
    } else if s.starts_with("http") || s.starts_with("ftp") {
        TargetKind::GenericUrl
    } else {
        TargetKind::LocalFile
    }
}

// Scans a playlist file to find the "heaviest" URL inside
fn inspect_playlist_content(path_str: &str, config: &Config) -> Option<String> {
    let path = std::path::Path::new(path_str);

    let ext = path.extension()?.to_str()?.to_lowercase();
    if !config.playlist_exts.contains(&ext) {
        return None;
    }

    let mut best_match: Option<String> = None;
    let mut max_kind = TargetKind::LocalFile;

    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let trim = line.trim();
            let kind = classify_target(trim);

            if kind > max_kind {
                max_kind = kind;
                best_match = Some(trim.to_string());
                if max_kind == TargetKind::YouTube {
                    break; // found yt, stop reading file
                }
            }
        }
    }
    best_match
}

fn apply_url_optimizations(cmd: &mut Command, target: &str, config: &Config) {
    let kind = classify_target(target);
    let is_youtube = kind.is_youtube();
    let is_url = kind.is_network();

    if is_url {
        log::debug!("Applying network stream optimizations");

        if target.contains("listen.moe") {
            log::debug!("LISTEN.moe detected: Forcing Live Mode (Cache=No, 1M Buffer)");
            cmd.arg("--cache=no");
            cmd.arg("--demuxer-max-bytes=1M");
            cmd.arg("--demuxer-max-back-bytes=0");
        }

        cmd.arg("--msg-level=ytdl_hook=info");
        let ytdlp_bin = config.ytdlp_bin();
        if ytdlp_bin != "yt-dlp" {
            log::debug!("Configuring custom yt-dlp binary for player: {}", ytdlp_bin);
            cmd.arg(format!(
                "--script-opts-append=ytdl_hook-ytdl_path={}",
                ytdlp_bin
            ));
        }

        if is_youtube {
            if !config.video_ok && !config.watch {
                log::debug!("YouTube detected & Audio Mode: forcing bestaudio format");
                cmd.arg("--ytdl-format=bestaudio/best");
            } else {
                log::debug!("YouTube detected & Video/Watch Mode: allowing default formats");
            }
        }

        let mut ytdl_opts = String::new();

        if target.contains("list=") {
            log::debug!("Playlist detected in URL, forcing yes-playlist");
            ytdl_opts.push_str("yes-playlist=,");
        }

        if is_youtube {
            log::debug!("Applying User-Agent from config");
            let ua = if config.ytdlp_useragent == "default" || config.ytdlp_useragent.is_empty() {
                crate::config::DEFAULT_YTDLP_USER_AGENT
            } else {
                &config.ytdlp_useragent
            };
            ytdl_opts.push_str(&format!("user-agent={},", ua));

            if config.ytdlp_ejs_remote_github && !config.ytdlp_is_nightly && ytdlp_bin == "yt-dlp" {
                log::debug!("Enabling remote EJS components");
                ytdl_opts.push_str("remote-components=ejs:github,");
            }

            if check_deno_availability(ytdlp_bin) {
                log::debug!("JS runtime check: Deno found (skipping fallbacks)");
            } else if has_command("node") {
                log::debug!("JS runtime check: node found");
                ytdl_opts.push_str("js-runtimes=node,");
            } else if has_command("qjs") || has_command("quickjs") {
                log::debug!("JS runtime check: quickjs found");
                ytdl_opts.push_str("js-runtimes=quickjs,");
            } else if has_command("bun") {
                log::debug!("JS runtime check: bun found");
                ytdl_opts.push_str("js-runtimes=bun,");
            } else {
                log::warn!("No JS runtime found (Deno/Node). YouTube playback may fail with 403.");
            }
        }

        if !ytdl_opts.is_empty() {
            let clean_opts = ytdl_opts.trim_end_matches(',');
            log::debug!("Applying ytdl-raw-options: {}", clean_opts);
            cmd.arg(format!("--ytdl-raw-options={}", clean_opts));
        }
    }
}

pub const DEFAULT_BANNER_TEXT: &str = " ╔══  MPV-MUSIC  ══╗";

pub fn default_banner_text(nerd_fonts: crate::config::NerdFontMode) -> String {
    let icons = Icons::new(nerd_fonts);
    match nerd_fonts {
        crate::config::NerdFontMode::None => DEFAULT_BANNER_TEXT.to_string(),
        _ => format!(" ── {} MPV-MUSIC ──", icons.track()),
    }
}

pub fn default_status_msg(nerd_fonts: crate::config::NerdFontMode) -> String {
    let icons = Icons::new(nerd_fonts);
    format!(
        " {} ${{?metadata/artist:${{metadata/artist}} - }}${{?metadata/title:${{metadata/title}}}}${{!metadata/title:${{media-title}}}} • ${{time-pos}} / ${{duration}} • (${{percent-pos}}%)",
        icons.play()
    )
}

fn apply_common_args(cmd: &mut Command, config: &Config, extra_args: &[String]) {
    log::debug!("Applying common MPV arguments from config");

    if config.watch {
        log::debug!("Visual mode enabled (--watch)");
        cmd.arg("--force-window=immediate");
        cmd.arg("--video=auto");
    } else {
        log::debug!("Audio-only mode (forcing video=no)");
        cmd.arg("--force-window=no");
        cmd.arg("--video=no");
        cmd.arg("--audio-display=no");
    }

    // Built-in base defaults (UI & terminal formatting)
    cmd.arg("--msg-level=cplayer=warn");
    cmd.arg("--display-tags=");
    cmd.arg("--no-term-osd-bar");

    let banner = default_banner_text(config.nerd_fonts);
    let is_debug = log::max_level() >= log::LevelFilter::Debug;
    if is_debug {
        log::debug!("Skipping screen clear to preserve logs");
        cmd.arg(format!("--term-playing-msg=\n{}\n", banner));
    } else {
        log::debug!("Injecting ANSI clear codes into banner");
        cmd.arg(format!(
            "--term-playing-msg=\x1b[H\x1b[2J\x1b[3J\n{}\n",
            banner
        ));
    }
    cmd.arg(format!(
        "--term-status-msg={}",
        default_status_msg(config.nerd_fonts)
    ));

    // User-configured extra args from config.toml (overrides base defaults if repeated)
    for arg in &config.mpv_args {
        if config.watch
            && (arg == "--no-video" || arg == "--video=no" || arg == "--audio-display=no")
        {
            log::debug!("Skipping '{}' because visual mode is active", arg);
            continue;
        }

        if arg.contains("--term-playing-msg=") {
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            if parts.len() == 2 {
                let banner_text = parts[1];
                if is_debug {
                    log::debug!("Skipping screen clear to preserve logs");
                    cmd.arg(format!("--term-playing-msg=\n{}\n", banner_text.trim()));
                } else {
                    log::debug!("Injecting ANSI clear codes into banner");
                    cmd.arg(format!(
                        "--term-playing-msg=\x1b[H\x1b[2J\x1b[3J\n{}\n",
                        banner_text.trim()
                    ));
                }
                continue; // skip the default cmd.arg(arg) below
            }
        }
        cmd.arg(arg);
    }

    log::debug!("Setting volume: {}", config.volume);
    cmd.arg(format!("--volume={}", config.volume));

    if config.shuffle {
        log::debug!("Shuffle enabled");
        cmd.arg("--shuffle");
    }

    log::debug!("Setting loop mode: {}", config.loop_mode);
    match config.loop_mode {
        LoopMode::Inf => {
            cmd.arg("--loop-playlist=inf");
        }
        LoopMode::Track => {
            cmd.arg("--loop-file=inf");
        }
        LoopMode::No => {
            cmd.arg("--loop-playlist=no");
            cmd.arg("--loop-file=no");
        }
        LoopMode::Count(n) => {
            cmd.arg(format!("--loop-playlist={}", n));
        }
    }

    if !extra_args.is_empty() {
        log::debug!("Injecting manual CLI overrides: {:?}", extra_args);
        for arg in extra_args {
            cmd.arg(arg);
        }
    }
}

fn check_deno_availability(ytdlp_bin: &str) -> bool {
    let p = std::path::Path::new(ytdlp_bin);
    if p.parent().is_some_and(|parent| {
        parent
            .join(if cfg!(windows) { "deno.exe" } else { "deno" })
            .exists()
            || parent.join("deno").exists()
    }) {
        return true;
    }

    let check_cmd = if cfg!(windows) { "where" } else { "which" };
    let Ok(output) = Command::new(check_cmd).arg(ytdlp_bin).output() else {
        return has_command("deno");
    };

    if !output.status.success() {
        return has_command("deno");
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let first_line = path_str.lines().next().unwrap_or("").trim();
    let ytdlp_path = std::path::Path::new(first_line);

    if ytdlp_path.parent().is_some_and(|p| {
        p.join(if cfg!(windows) { "deno.exe" } else { "deno" })
            .exists()
            || p.join("deno").exists()
    }) {
        return true;
    }

    has_command("deno")
}

fn has_command(cmd: &str) -> bool {
    let check_cmd = if cfg!(windows) { "where" } else { "which" };
    let exists = Command::new(check_cmd)
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    log::debug!("Command check: {} exists = {}", cmd, exists);
    exists
}

fn check_ytdlp_status(ytdlp_bin: &str) {
    log::info!("Attempting {} self-update ({} -U)...", ytdlp_bin, ytdlp_bin);
    let output = match Command::new(ytdlp_bin).arg("-U").output() {
        Ok(o) => o,
        Err(_) => {
            log::error!("{} executable not found in PATH", ytdlp_bin);
            return;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}\n{}", stdout, String::from_utf8_lossy(&output.stderr));

    if combined.contains("is up to date") {
        log::info!("{} is verified up to date.", ytdlp_bin);
    } else if combined.contains("Latest version:") || combined.contains("Available version:") {
        log::warn!("{} update available. Local version is outdated.", ytdlp_bin);
    } else {
        log::debug!(
            "{} status check returned unexpected output:\n{}",
            ytdlp_bin,
            combined
        );
    }
}

pub fn play_radio(name: &str, url: &str, config: &Config, extra_args: &[String]) -> Result<()> {
    log::info!("Entering Radio Mode: {}", name);
    play(url, config, extra_args)
}

fn handle_radio_sync(cmd: &mut Command, target: &str, config: &Config) -> Option<String> {
    let is_radio = crate::radio::RADIO_STATIONS
        .iter()
        .any(|(_, url, _)| *url == target);

    if !is_radio {
        return None;
    }

    let pid = std::process::id();
    let ipc_socket = if cfg!(windows) {
        format!(r"\\.\pipe\mpv-music-ipc-{}", pid)
    } else {
        format!("/tmp/mpv-music-ipc-{}.sock", pid)
    };

    cmd.arg(format!("--input-ipc-server={}", ipc_socket));

    let (station_name, is_listen_moe) = crate::radio::RADIO_STATIONS
        .iter()
        .find(|(_, url, _)| *url == target)
        .map(|(name, _, is_moe)| {
            let clean_name = name.split(") ").nth(1).unwrap_or(name).to_uppercase();
            (clean_name, *is_moe)
        })
        .unwrap_or_else(|| ("RADIO".to_string(), false));

    let icons = Icons::new(config.nerd_fonts);
    cmd.arg(format!(
        "--term-status-msg= {} ${{media-title}} • ${{time-pos}} • [ {} ]",
        icons.play(),
        station_name
    ));

    if is_listen_moe {
        let target_clone = target.to_string();
        let socket_clone = ipc_socket.to_string();
        std::thread::spawn(move || {
            let p = std::path::Path::new(&socket_clone);
            let mut attempts = 0;

            // Wait up to 5 seconds (50 * 100ms) for mpv to create the socket
            while !p.exists() && attempts < 50 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                attempts += 1;
            }
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let _ =
                    crate::radio::listen_moe::start_radio_sync(&target_clone, socket_clone).await;
            });
        });
    }

    Some(ipc_socket)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_youtube() {
        assert_eq!(
            classify_target("https://youtube.com/watch?v=test"),
            TargetKind::YouTube
        );
        assert_eq!(
            classify_target("https://youtu.be/test123"),
            TargetKind::YouTube
        );
        assert_eq!(
            classify_target("http://youtube.com/playlist"),
            TargetKind::YouTube
        );
    }

    #[test]
    fn test_classify_generic_url() {
        assert_eq!(
            classify_target("https://example.com/song.mp3"),
            TargetKind::GenericUrl
        );
        assert_eq!(
            classify_target("http://radio.com/stream"),
            TargetKind::GenericUrl
        );
        assert_eq!(
            classify_target("ftp://server.com/file"),
            TargetKind::GenericUrl
        );
    }

    #[test]
    fn test_classify_local_file() {
        assert_eq!(
            classify_target("/home/user/music.mp3"),
            TargetKind::LocalFile
        );
        assert_eq!(classify_target("./local/file.flac"), TargetKind::LocalFile);
        assert_eq!(
            classify_target("C:\\Music\\song.mp3"),
            TargetKind::LocalFile
        );
    }

    #[test]
    fn test_classify_empty() {
        assert_eq!(classify_target(""), TargetKind::LocalFile);
    }

    #[test]
    fn test_classify_priority_order() {
        // YouTube > GenericUrl > LocalFile
        let youtube = classify_target("https://youtube.com/test");
        let http = classify_target("https://example.com/test");
        let local = classify_target("/path/to/file");

        assert!(youtube > http);
        assert!(http > local);
    }

    #[test]
    fn test_has_command_invalid() {
        // These commands should NOT exist
        assert!(!has_command("this_command_definitely_does_not_exist_12345"));
    }

    #[test]
    fn test_apply_url_optimizations_custom_ytdlp() {
        let mut config = Config::default();
        config.ytdlp = "custom-dlp".to_string();
        let mut cmd = Command::new("mpv");
        apply_url_optimizations(&mut cmd, "https://youtube.com/watch?v=123", &config);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(
            args.iter()
                .any(|a| a == "--script-opts-append=ytdl_hook-ytdl_path=custom-dlp")
        );
    }

    #[test]
    fn test_apply_url_optimizations_default_ytdlp() {
        let config = Config::default();
        let mut cmd = Command::new("mpv");
        apply_url_optimizations(&mut cmd, "https://youtube.com/watch?v=123", &config);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(!args.iter().any(|a| a.contains("ytdl_hook-ytdl_path")));
    }

    #[test]
    fn test_apply_url_optimizations_custom_ytdlp_skips_remote_ejs() {
        let mut config = Config::default();
        config.ytdlp = "custom-fork".to_string();
        config.ytdlp_ejs_remote_github = true;
        let mut cmd = Command::new("mpv");
        apply_url_optimizations(&mut cmd, "https://youtube.com/watch?v=123", &config);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(
            !args
                .iter()
                .any(|a| a.contains("remote-components=ejs:github"))
        );
    }

    #[test]
    fn test_apply_common_args_defaults() {
        let config = Config::default();
        let mut cmd = Command::new("mpv");
        apply_common_args(&mut cmd, &config, &[]);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.iter().any(|a| a == "--msg-level=cplayer=warn"));
        assert!(args.iter().any(|a| a == "--no-term-osd-bar"));
        assert!(args.iter().any(|a| a.contains("MPV-MUSIC")));
        assert!(args.iter().any(|a| a.contains("term-status-msg")));
    }

    #[test]
    fn test_apply_common_args_custom_mpv_args() {
        let mut config = Config::default();
        config.mpv_args = vec!["--gapless-audio=yes".to_string()];
        let mut cmd = Command::new("mpv");
        apply_common_args(&mut cmd, &config, &[]);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.iter().any(|a| a == "--gapless-audio=yes"));
    }

    #[test]
    fn test_apply_common_args_nerd_fonts() {
        let mut config_none = Config::default();
        config_none.nerd_fonts = crate::config::NerdFontMode::None;
        let mut cmd_none = Command::new("mpv");
        apply_common_args(&mut cmd_none, &config_none, &[]);
        let args_none: Vec<String> = cmd_none
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        let status_none = args_none
            .iter()
            .find(|a| a.starts_with("--term-status-msg="))
            .unwrap();
        assert!(status_none.contains("▶"));
        let banner_none = args_none
            .iter()
            .find(|a| a.starts_with("--term-playing-msg="))
            .unwrap();
        assert!(banner_none.contains("╔══  MPV-MUSIC  ══╗"));

        let mut config_mono = Config::default();
        config_mono.nerd_fonts = crate::config::NerdFontMode::Mono;
        let mut cmd_mono = Command::new("mpv");
        apply_common_args(&mut cmd_mono, &config_mono, &[]);
        let args_mono: Vec<String> = cmd_mono
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        let status_mono = args_mono
            .iter()
            .find(|a| a.starts_with("--term-status-msg="))
            .unwrap();
        assert!(status_mono.contains("\u{f04b}"));
        let banner_mono = args_mono
            .iter()
            .find(|a| a.starts_with("--term-playing-msg="))
            .unwrap();
        assert!(banner_mono.contains("── \u{f001} MPV-MUSIC ──"));
    }
}
