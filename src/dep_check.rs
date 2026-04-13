use crate::config::Config;
use anyhow::Result;
use std::process::{Command, Stdio, exit};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn check(cfg: &mut Config) -> Result<()> {
    log::info!("Checking external dependencies...");

    // Spawn both processes WITHOUT waiting (true parallelism without thread overhead)
    let mpv_cmd = if cfg!(windows) { "mpv.com" } else { "mpv" };
    let mut mpv_command = Command::new(mpv_cmd);
    mpv_command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    mpv_command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let mpv_child = mpv_command.spawn();

    let mut ytdlp_command = Command::new("yt-dlp");
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
            "mpv not found",
        )),
    };

    match mpv_output {
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
            log::warn!("Dependency 'yt-dlp' not found. Search and Streaming features disabled.");
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
                    log::info!("Dependency 'yt-dlp': Found Nightly (Version: {})", version);
                    cfg.ytdlp_is_nightly = true;
                } else {
                    log::info!("Dependency 'yt-dlp': Found Stable (Version: {})", version);
                    println!(
                        "\x1b[33m[Suggestion]\x1b[0m yt-dlp nightly is recommended for best performance."
                    );
                    println!(
                        "             Get it here: https://github.com/yt-dlp/yt-dlp-nightly-builds/releases"
                    );
                    cfg.ytdlp_is_nightly = false;
                }
            } else {
                log::warn!("Dependency 'yt-dlp' found but returned error status.");
                cfg.ytdlp_available = false;
                cfg.ytdlp_is_nightly = false;
            }
        }
        Err(_) => {
            log::warn!("Dependency 'yt-dlp' not found. Search and Streaming features disabled.");
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
