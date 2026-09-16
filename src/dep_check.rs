use crate::config::Config;
use crate::ui;
use anyhow::Result;
use std::process::{Command, Stdio, exit};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn check(cfg: &mut Config) -> Result<()> {
    log::info!("Checking external dependencies...");

    // Spawn both proc without waiting
    let player_cmd = cfg.player_bin().to_string();
    let mut mpv_command = Command::new(&player_cmd);
    mpv_command
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    #[cfg(windows)]
    mpv_command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let mpv_child = mpv_command.spawn();

    let ytdlp_cmd = cfg.ytdlp_bin().to_string();
    let mut ytdlp_command = Command::new(&ytdlp_cmd);
    ytdlp_command
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    #[cfg(windows)]
    ytdlp_command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let ytdlp_child = ytdlp_command.spawn();

    // player is critical: wait and fail immediately if not present
    let mpv_output = match mpv_child {
        Ok(child) => child.wait_with_output(),
        Err(e) => Err(e),
    };

    match mpv_output {
        Ok(output) => {
            let raw_output = String::from_utf8_lossy(&output.stdout);
            let player_line = raw_output.lines().next().unwrap_or("Unknown Version");
            let ffmpeg_line = raw_output
                .lines()
                .find(|l| l.contains("FFmpeg version"))
                .map(|s| s.trim())
                .unwrap_or("FFmpeg version: Unknown");

            log::info!("Dependency '{}': Found", player_cmd);
            log::info!(" └─ {}", player_line);
            log::info!(" └─ {}", ffmpeg_line);
        }
        Err(_) => {
            let hint = if player_cmd != "mpv" && player_cmd != "mpv.com" {
                "Check your 'player' setting in config.toml or the --player CLI option."
            } else {
                "Please install it via your package manager (e.g. sudo dnf install mpv)."
            };
            ui::error(format!(
                "Critical: '{}' not found!\nmpv-music requires '{}' to be installed and in your PATH.\n{}",
                player_cmd, player_cmd, hint
            ));

            #[cfg(windows)]
            ui::prompt_exit();

            exit(1);
        }
    }

    let ytdlp_output = match ytdlp_child {
        Ok(child) => child.wait_with_output(),
        Err(_) => {
            log::warn!(
                "Dependency '{}' not found. Search and Streaming features disabled.",
                ytdlp_cmd
            );
            cfg.ytdlp_available = false;
            cfg.ytdlp_is_nightly = false;
            return Ok(());
        }
    };

    match ytdlp_output {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                cfg.ytdlp_available = true;

                let is_nightly = version.split('.').count() >= 4 || version.contains("nightly");

                if is_nightly {
                    log::info!(
                        "Dependency '{}': Found Nightly (Version: {})",
                        ytdlp_cmd,
                        version
                    );
                    cfg.ytdlp_is_nightly = true;
                } else {
                    log::info!(
                        "Dependency '{}': Found Stable (Version: {})",
                        ytdlp_cmd,
                        version
                    );
                    if ytdlp_cmd == "yt-dlp" {
                        ui::suggestion(
                            "yt-dlp nightly is recommended for best performance (https://github.com/yt-dlp/yt-dlp-nightly-builds/releases)",
                        );
                    }
                    cfg.ytdlp_is_nightly = false;
                }
            } else {
                log::warn!(
                    "Dependency '{}' found but returned error status.",
                    ytdlp_cmd
                );
                cfg.ytdlp_available = false;
                cfg.ytdlp_is_nightly = false;
            }
        }
        Err(_) => {
            log::warn!(
                "Dependency '{}' not found. Search and Streaming features disabled.",
                ytdlp_cmd
            );
            cfg.ytdlp_available = false;
            cfg.ytdlp_is_nightly = false;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_version_parsing_nightly() {
        let version = "2026.02.12.233641";
        let is_nightly = version.split('.').count() >= 4 || version.contains("nightly");

        assert!(is_nightly);
    }

    #[test]
    fn test_version_parsing_stable() {
        let version = "2026.02.12";
        let is_nightly = version.split('.').count() >= 4 || version.contains("nightly");

        assert!(!is_nightly);
    }
}
