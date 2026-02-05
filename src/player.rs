use crate::config::Config;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn play(target: &str, config: &Config) -> Result<()> {
    log::info!("Preparing playback for target: {}", target);

    let mut cmd = Command::new("mpv");

    apply_common_args(&mut cmd, config);
    apply_url_optimizations(&mut cmd, target, config);

    cmd.arg(target);

    log::info!("Launching MPV process...");
    log::debug!("Exec: {:?}", cmd);

    let status = cmd.status().context("Failed to launch mpv")?;

    if !status.success() && target.starts_with("http") {
        log::error!("MPV process exited with error status. Checking yt-dlp health...");
        check_ytdlp_status();
    }

    Ok(())
}

pub fn play_files(paths: &[String], config: &Config) -> Result<()> {
    if paths.is_empty() {
        log::debug!("play_files called with empty path list, skipping");
        return Ok(());
    }

    log::info!("Preparing playback for {} files", paths.len());
    let mut cmd = Command::new("mpv");

    apply_common_args(&mut cmd, config);

    if let Some(first) = paths.first() {
        apply_url_optimizations(&mut cmd, first, config);
    }

    let dirs = ProjectDirs::from("com", "furqanhun", "mpv-music")
        .context("Could not determine data directory")?;
    let data_dir = dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;

    let queue_path = data_dir.join("queue.m3u8");

    {
        let mut file = std::fs::File::create(&queue_path)
            .context("Failed to create temporary playlist file")?;

        writeln!(file, "#EXTM3U")?;
        for path in paths {
            writeln!(file, "{}", path)?;
        }
    }

    log::info!("Generated playlist at {:?}", queue_path);

    // Pass the file to MPV
    cmd.arg(format!("--playlist={}", queue_path.to_string_lossy()));

    log::info!("Launching MPV for playlist playback...");
    log::debug!("Exec: {:?}", cmd);

    // blocks until mpv closes (finished/crashed)
    cmd.status().context("Failed to launch mpv for playlist")?;

    log::debug!("Cleaning up temporary playlist: {:?}", queue_path);
    if let Err(e) = std::fs::remove_file(&queue_path) {
        log::debug!("Failed to remove temporary playlist: {}", e);
    } else {
        log::debug!("Temporary playlist removed successfully.");
    }

    Ok(())
}

// helpers
fn apply_url_optimizations(cmd: &mut Command, target: &str, config: &Config) {
    let is_url = target.starts_with("http")
        || target.starts_with("yt-dlp://")
        || target.starts_with("ftp://");
    let is_youtube = target.contains("youtube.com") || target.contains("youtu.be");

    if is_url {
        log::debug!("Applying network stream optimizations");
        cmd.arg("--msg-level=ytdl_hook=info");

        if is_youtube {
            if !config.video_ok {
                log::debug!("YouTube detected & Video Disabled: forcing bestaudio format");
                cmd.arg("--ytdl-format=bestaudio/best");
            } else {
                log::debug!("YouTube detected & Video Enabled: allowing default formats");
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

            if config.ytdlp_ejs_remote_github {
                log::debug!("Enabling remote EJS components");
                ytdl_opts.push_str("remote-components=ejs:github,");
            }

            if has_command("deno") {
                log::debug!("JS runtime check: deno found (default)");
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

fn apply_common_args(cmd: &mut Command, config: &Config) {
    log::debug!("Applying common MPV arguments from config");

    if config.video_ok {
        log::debug!("Video enabled (video_ok=true)");
    } else {
        log::debug!("Video disabled (force-window=no, video=no)");
        cmd.arg("--force-window=no");
        cmd.arg("--video=no");
    }

    for arg in &config.mpv_default_args {
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
}

fn has_command(cmd: &str) -> bool {
    let exists = Command::new("which")
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
    log::info!("Executing yt-dlp health check (yt-dlp -U)...");
    let output = match Command::new("yt-dlp").arg("-U").output() {
        Ok(o) => o,
        Err(_) => {
            log::error!("yt-dlp executable not found in PATH");
            return;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}\n{}", stdout, String::from_utf8_lossy(&output.stderr));

    if combined.contains("Latest version:") {
        log::warn!("yt-dlp update available. Local version is outdated.");
    } else if combined.contains("up to date") {
        log::info!("yt-dlp is verified up to date.");
    }
}
