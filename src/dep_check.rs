use crate::config::Config;
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
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    mpv_command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let mpv_child = mpv_command.spawn();

    let ytdlp_cmd = cfg.ytdlp_bin().to_string();
    let mut ytdlp_command = Command::new(&ytdlp_cmd);
    ytdlp_command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    ytdlp_command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let ytdlp_child = ytdlp_command.spawn();

    let mpv_output = match mpv_child {
        Ok(child) => child.wait_with_output(),
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("player '{}' not found", player_cmd),
        )),
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
            eprintln!(
                "\n\x1b[31;1m[Critical Error]\x1b[0m '{}' not found!",
                player_cmd
            );
            eprintln!(
                "mpv-music requires '{}' to be installed and in your PATH.",
                player_cmd
            );
            if player_cmd != "mpv" && player_cmd != "mpv.com" {
                eprintln!("Check your 'player' setting in config.toml or the --player CLI option.");
            } else {
                eprintln!(
                    "Please install it via your package manager (e.g. sudo dnf install mpv)."
                );
            }

            log::error!("Critical dependency missing: {}. Exiting.", player_cmd);

            if cfg!(windows) {
                eprintln!("\nPress Enter to exit...");
                let _ = std::io::stdin().read_line(&mut String::new());
            }
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
                        println!(
                            "\x1b[33;1m[Suggestion]\x1b[0m yt-dlp nightly is recommended for best performance."
                        );
                        println!(
                            "             Get it here: https://github.com/yt-dlp/yt-dlp-nightly-builds/releases"
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
