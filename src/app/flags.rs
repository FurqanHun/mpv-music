use crate::cli::Cli;
use crate::config;
use crate::dep_check;
use crate::indexer;
use crate::tui;
use anyhow::Result;
use std::path::Path;

pub fn handle_utility_flags(args: &Cli, config_file: &Path, log_dir: &Path) -> Result<bool> {
    let log_file_path = log_dir.join("mpv-music.log");

    if args.remove_log {
        if log_file_path.exists() {
            std::fs::remove_file(&log_file_path)?;
            println!("Log file nuked.");
        } else {
            println!("No log file available.");
        }
        return Ok(true);
    }

    if let Some(viewer_opt) = &args.log {
        let viewer = viewer_opt.clone().unwrap_or_else(|| {
            std::env::var("PAGER").unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad.exe".to_string()
                } else {
                    "less".to_string()
                }
            })
        });
        if log_file_path.exists() {
            std::process::Command::new(viewer)
                .arg(&log_file_path)
                .status()?;
        } else {
            println!("No log file available.");
        }
        return Ok(true);
    }

    if args.remove_config {
        if config_file.exists() {
            std::fs::remove_file(config_file)?;
            println!("Config removed.");
        } else {
            println!("No config file found.");
        }
        return Ok(true);
    }

    if let Some(editor_opt) = &args.config {
        if !config_file.exists() {
            log::info!(
                "Config file not found. Generating default at {:?}...",
                config_file
            );
            let _ = config::load(None)?;
        }

        let editor = editor_opt.clone().unwrap_or_else(|| {
            std::env::var("EDITOR").unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad.exe".to_string()
                } else {
                    "nano".to_string()
                }
            })
        });

        log::info!("Opening config with editor: {}", editor);

        let status = std::process::Command::new(&editor)
            .arg(config_file)
            .status();

        if let Err(e) = status {
            eprintln!(
                "\x1b[31;1m[Error]\x1b[0m Failed to launch editor. Is '{}' installed? ({})",
                editor, e
            );
        }
        return Ok(true);
    }

    Ok(false)
}

pub fn apply_cli_overrides(cfg: &mut config::Config, args: &Cli) -> Result<()> {
    if let Some(ref p) = args.player {
        cfg.player = p.clone();
    }
    if let Some(ref y) = args.ytdlp {
        cfg.ytdlp = y.clone();
    }

    dep_check::check(cfg)?;

    if args.serial {
        cfg.serial_mode = true;
    }

    if cfg.serial_mode {
        if let Err(e) = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build_global()
        {
            log::warn!(
                "Failed to set serial mode (Rayon pool already built): {}",
                e
            );
        } else {
            log::debug!("Serial mode active: Thread pool restricted to 1 thread.");
        }
    }

    if let Some(v) = args.volume {
        cfg.volume = v;
    }
    if args.shuffle {
        cfg.shuffle = true;
    }
    if args.no_shuffle {
        cfg.shuffle = false;
    }
    if args.video_ok {
        cfg.video_ok = true;
    }
    if args.no_video {
        cfg.video_ok = false;
    }
    if args.watch {
        cfg.watch = true;
    }
    if args.no_watch {
        cfg.watch = false;
    }
    if let Some(ref mode) = args.loop_arg {
        cfg.loop_mode = mode.parse().unwrap_or(config::LoopMode::Inf);
    }
    if args.no_loop {
        cfg.loop_mode = config::LoopMode::No;
    }
    if args.repeat {
        cfg.loop_mode = config::LoopMode::Track;
    }

    if let Some(ref extensions) = args.ext {
        cfg.audio_exts = extensions
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    Ok(())
}

pub fn handle_dir_flags(cfg: &mut config::Config, args: &Cli) -> Result<bool> {
    let mut config_changed = false;
    if let Some(dirs) = &args.add_dir {
        for dir in dirs {
            if tui::add_directory(cfg, dir.clone())? {
                config_changed = true;
            }
        }
    }
    if let Some(dirs) = &args.remove_dir {
        for dir in dirs {
            if tui::remove_directory(cfg, dir.clone())? {
                config_changed = true;
            }
        }
    }
    if config_changed {
        config::save(cfg)?;
        println!("Configuration saved. Syncing index...");
        let tracks = indexer::scan(cfg, false)?;
        indexer::save(&tracks)?;
        return Ok(true);
    }
    if args.manage_dirs {
        if tui::run_manage_dirs_mode(cfg)? {
            config::save(cfg)?;
            println!("Configuration saved.");
            println!("Syncing index with new directories...");
            let tracks = indexer::scan(cfg, false)?;
            indexer::save(&tracks)?;
        }
        return Ok(true);
    }
    Ok(false)
}
