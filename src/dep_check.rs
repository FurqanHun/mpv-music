use crate::config::Config;
use anyhow::Result;
use std::process::{Command, exit};

pub fn check(cfg: &mut Config) -> Result<()> {
    log::info!("Checking external dependencies...");

    match Command::new("mpv").arg("--version").output() {
        Ok(output) => {
            let raw_output = String::from_utf8_lossy(&output.stdout);
            let mpv_line = raw_output.lines().next().unwrap_or("Unknown Version");
            let ffmpeg_line = raw_output
                .lines()
                .find(|l| l.contains("FFmpeg version"))
                .map(|s| s.trim())
                .unwrap_or("FFmpeg version: Unknown");

            log::info!("Dependency 'mpv': Found");
            log::info!(" └─ {}", mpv_line);
            log::info!(" └─ {}", ffmpeg_line);
        }
        Err(_) => {
            eprintln!("\n\x1b[31;1mCRITICAL ERROR: 'mpv' not found!\x1b[0m");
            eprintln!("mpv-music requires 'mpv' to be installed and in your PATH.");
            eprintln!("Please install it via your package manager (e.g. sudo dnf install mpv).");

            log::error!("Critical dependency missing: mpv. Exiting.");
            exit(1);
        }
    }

    match Command::new("yt-dlp").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            log::info!("Dependency 'yt-dlp': Found (Version: {})", version);
            cfg.ytdlp_available = true;
        }
        Err(_) => {
            log::warn!("Dependency 'yt-dlp' not found. Search and Streaming features disabled.");
            cfg.ytdlp_available = false;
        }
    }

    Ok(())
}
