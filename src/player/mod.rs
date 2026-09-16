pub mod builder;
pub mod radio;
pub mod target;
pub mod ytdlp;

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use directories::ProjectDirs;

use crate::config::Config;

pub use builder::MpvCommandBuilder;
pub use target::{classify_target, find_representative_target, resolve_target_optimization};
pub use ytdlp::check_ytdlp_status;

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

    let opt_target = resolve_target_optimization(target, config);
    let builder = MpvCommandBuilder::new(config)
        .target(target)
        .optimization_target(opt_target.clone())
        .extra_args(extra_args)
        .with_radio_sync();

    let socket_to_clean = builder.ipc_socket().map(|s| s.to_string());
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

    let cmd_name = config.player_bin();
    let mut cmd = builder.build();

    log::debug!("Exec: {:?}", cmd);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to launch player '{}'", cmd_name))?;

    if !status.success() && classify_target(&opt_target).is_network() {
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

    let queue_path_str = queue_path.to_string_lossy().to_string();
    let best_target = find_representative_target(paths);

    let mut builder = MpvCommandBuilder::new(config)
        .playlist(queue_path_str)
        .extra_args(extra_args);

    if let Some(target) = best_target {
        log::debug!("Configuring mpv based on representative track: {}", target);
        builder = builder.optimization_target(target.to_string());
        if radio::is_radio_station(target) {
            builder = builder.target(target).with_radio_sync();
        }
    }

    let socket_to_clean = builder.ipc_socket().map(|s| s.to_string());
    let _ipc_guard = IpcCleaner {
        path: socket_to_clean.clone(),
    };

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

    let cmd_name = config.player_bin();
    let mut cmd = builder.build();

    log::info!("Launching player '{}' for playlist playback...", cmd_name);
    log::debug!("Exec: {:?}", cmd);

    // blocks until mpv closes
    cmd.status()
        .with_context(|| format!("Failed to launch player '{}' for playlist", cmd_name))?;

    Ok(())
}

pub fn play_radio(name: &str, url: &str, config: &Config, extra_args: &[String]) -> Result<()> {
    crate::ui::info(format!("Connecting to station '{}'...", name));
    play(url, config, extra_args)
}
