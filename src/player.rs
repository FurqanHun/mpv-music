use crate::config::Config;
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

pub fn play(target: &str, config: &Config, extra_args: &[String]) -> Result<()> {
    log::info!("Preparing playback for target: {}", target);

    let mut cmd = Command::new("mpv");

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

    handle_radio_sync(&mut cmd, target);

    cmd.arg(target);

    log::debug!("Exec: {:?}", cmd);

    let status = cmd.status().context("Failed to launch mpv")?;

    if !status.success() && classify_target_weight(&optimization_target) > 0 {
        log::error!("MPV process exited with error status. Checking yt-dlp health...");
        check_ytdlp_status();
    }

    Ok(())
}

pub fn play_files(paths: &[String], config: &Config, extra_args: &[String]) -> Result<()> {
    if paths.is_empty() {
        log::debug!("play_files called with empty path list, skipping");
        return Ok(());
    }

    log::info!("Preparing playback for {} files", paths.len());
    let mut cmd = Command::new("mpv");

    apply_common_args(&mut cmd, config, extra_args);

    // O(N) Single-Pass Scan: Find the item with the highest requirement.
    // 0 = Local (Default)
    // 1 = HTTP/FTP (Basic network opts)
    // 2 = YouTube (Needs JS runtimes & headers)
    let mut best_target = paths.first();
    let mut max_weight = 0;

    for path in paths {
        let weight = classify_target_weight(path);
        if weight > max_weight {
            max_weight = weight;
            best_target = Some(path);
            if max_weight == 2 {
                break; // found yt, stop scanning
            }
        }
    }

    if let Some(target) = best_target {
        log::debug!("Configuring mpv based on representative track: {}", target);
        apply_url_optimizations(&mut cmd, target, config);
        handle_radio_sync(&mut cmd, target);
    }

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

    // Register signal handler
    ctrlc::set_handler(move || {
        if r_handler.swap(false, Ordering::SeqCst) && p_handler.exists() {
            let _ = std::fs::remove_file(&p_handler);
            log::info!("\nReceived Ctrl+C. Cleaned up queue file.");
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

    log::info!("Launching MPV for playlist playback...");
    log::debug!("Exec: {:?}", cmd);

    // blocks until mpv closes
    cmd.status().context("Failed to launch mpv for playlist")?;

    Ok(())
}

// helpers

// 0 = Local File
// 1 = Generic Network URL
// 2 = YouTube (Requires yt-dlp setup)
fn classify_target_weight(s: &str) -> u8 {
    if s.contains("youtube.com") || s.contains("youtu.be") {
        2
    } else if s.starts_with("http") || s.starts_with("ftp") {
        1
    } else {
        0
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
    let mut max_weight = 0;

    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let trim = line.trim();
            let weight = classify_target_weight(trim);

            if weight > max_weight {
                max_weight = weight;
                best_match = Some(trim.to_string());
                if max_weight == 2 {
                    break; // found yt, stop reading file
                }
            }
        }
    }
    best_match
}

fn apply_url_optimizations(cmd: &mut Command, target: &str, config: &Config) {
    let weight = classify_target_weight(target);
    let is_youtube = weight == 2;
    let is_url = weight >= 1;

    if is_url {
        log::debug!("Applying network stream optimizations");
        cmd.arg("--msg-level=ytdl_hook=info");

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
            ytdl_opts.push_str(&format!("user-agent={},", config.ytdlp_useragent));

            if config.ytdlp_ejs_remote_github && !config.ytdlp_is_nightly {
                log::debug!("Enabling remote EJS components");
                ytdl_opts.push_str("remote-components=ejs:github,");
            }

            if check_deno_availability() {
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

    for arg in &config.mpv_default_args {
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
                let is_debug = log::max_level() >= log::LevelFilter::Debug;
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
    match config.loop_mode.as_str() {
        "playlist" | "inf" => {
            cmd.arg("--loop-playlist=inf");
        }
        "track" | "file" => {
            cmd.arg("--loop-file=inf");
        }
        "no" | "off" | "false" => {
            cmd.arg("--loop-playlist=no");
            cmd.arg("--loop-file=no");
        }
        n if n.chars().all(char::is_numeric) => {
            cmd.arg(format!("--loop-playlist={}", n));
        }
        _ => {
            log::debug!("Unrecognized loop mode, skipping loop arguments");
        }
    }

    if !extra_args.is_empty() {
        log::debug!("Injecting manual CLI overrides: {:?}", extra_args);
        for arg in extra_args {
            cmd.arg(arg);
        }
    }
}

fn check_deno_availability() -> bool {
    let check_cmd = if cfg!(windows) { "where" } else { "which" };
    let Ok(output) = Command::new(check_cmd).arg("yt-dlp").output() else {
        return has_command("deno");
    };

    if !output.status.success() {
        return has_command("deno");
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let ytdlp_path = std::path::Path::new(&path_str);

    if ytdlp_path.parent().is_some_and(|p| p.join("deno").exists()) {
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

fn check_ytdlp_status() {
    log::info!("Attempting yt-dlp self-update (yt-dlp -U)...");
    let output = match Command::new("yt-dlp").arg("-U").output() {
        Ok(o) => o,
        Err(_) => {
            log::error!("yt-dlp executable not found in PATH");
            return;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}\n{}", stdout, String::from_utf8_lossy(&output.stderr));

    if combined.contains("is up to date") {
        log::info!("yt-dlp is verified up to date.");
    } else if combined.contains("Latest version:") || combined.contains("Available version:") {
        log::warn!("yt-dlp update available. Local version is outdated.");
    } else {
        log::debug!(
            "yt-dlp status check returned unexpected output:\n{}",
            combined
        );
    }
}

const JPOP_SQUAD: &[&str] = &[
    "https://listen.moe/stream",
    // "https://listen.moe/opus",
    // "https://listen.moe/fallback",
];

const KPOP_SQUAD: &[&str] = &[
    "https://listen.moe/kpop/stream",
    // "https://listen.moe/kpop/opus",
    // "https://listen.moe/kpop/fallback",
];

pub fn play_radio(choice: &str, config: &Config, extra_args: &[String]) -> Result<()> {
    let squad = if choice.to_lowercase() == "kpop" {
        KPOP_SQUAD
    } else {
        JPOP_SQUAD
    };

    let paths: Vec<String> = squad.iter().map(|s| s.to_string()).collect();

    log::info!("Entering {} Radio Mode", choice.to_uppercase());

    if paths.len() == 1 {
        log::debug!("Single radio stream detected, skipping playlist file generation.");
        return play(&paths[0], config, extra_args);
    }

    play_files(&paths, config, extra_args)
}

fn handle_radio_sync(cmd: &mut Command, target: &str) {
    if !target.contains("listen.moe") {
        return;
    }

    let ipc_socket = if cfg!(windows) {
        r"\\.\pipe\mpv-music-ipc"
    } else {
        "/tmp/mpv-music-ipc.sock"
    };

    cmd.arg(format!("--input-ipc-server={}", ipc_socket));
    let radio_type = if target.contains("kpop") {
        "KPOP"
    } else {
        "JPOP"
    };
    cmd.arg(format!(
        "--term-status-msg=▶ ${{media-title}} • ${{time-pos}} • [ {} RADIO ]",
        radio_type
    ));

    let target_clone = target.to_string();
    let socket_clone = ipc_socket.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let _ = crate::moe::start_radio_sync(&target_clone, socket_clone).await;
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_youtube() {
        assert_eq!(
            classify_target_weight("https://youtube.com/watch?v=test"),
            2
        );
        assert_eq!(classify_target_weight("https://youtu.be/test123"), 2);
        assert_eq!(classify_target_weight("http://youtube.com/playlist"), 2);
    }

    #[test]
    fn test_classify_generic_url() {
        assert_eq!(classify_target_weight("https://example.com/song.mp3"), 1);
        assert_eq!(classify_target_weight("http://radio.com/stream"), 1);
        assert_eq!(classify_target_weight("ftp://server.com/file"), 1);
    }

    #[test]
    fn test_classify_local_file() {
        assert_eq!(classify_target_weight("/home/user/music.mp3"), 0);
        assert_eq!(classify_target_weight("./local/file.flac"), 0);
        assert_eq!(classify_target_weight("C:\\Music\\song.mp3"), 0);
    }

    #[test]
    fn test_classify_empty() {
        assert_eq!(classify_target_weight(""), 0);
    }

    #[test]
    fn test_classify_priority_order() {
        // YouTube (2) > HTTP (1) > Local (0)
        let youtube = classify_target_weight("https://youtube.com/test");
        let http = classify_target_weight("https://example.com/test");
        let local = classify_target_weight("/path/to/file");

        assert!(youtube > http);
        assert!(http > local);
    }

    #[test]
    fn test_has_command_invalid() {
        // These commands should NOT exist
        assert!(!has_command("this_command_definitely_does_not_exist_12345"));
    }

    // Note: We can't reliably test has_command for real commands
    // because they might not be installed in CI environment
}
