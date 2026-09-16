use crate::cli::Cli;
use crate::config;
use crate::dep_check;
use crate::indexer;
use crate::tui;
use crate::ui;
use anyhow::Result;
use std::path::Path;

pub fn handle_utility_flags(args: &Cli, config_file: &Path, log_dir: &Path) -> Result<bool> {
    if let Some(ref remove_arg) = args.remove_log {
        let logs = crate::app::logging::list_log_files(log_dir);
        if logs.is_empty() {
            ui::warning("No log files available to delete.");
            return Ok(true);
        }

        match remove_arg {
            None => {
                let mut count = 0;
                for file in &logs {
                    if std::fs::remove_file(file).is_ok() {
                        count += 1;
                    }
                }
                ui::success(format!("Deleted all {} log file(s).", count));
            }
            Some(n) => {
                let to_delete = (*n).min(logs.len());
                let start_idx = logs.len() - to_delete;
                let mut count = 0;
                for file in &logs[start_idx..] {
                    if std::fs::remove_file(file).is_ok() {
                        count += 1;
                    }
                }
                ui::success(format!("Deleted {} oldest log file(s).", count));
            }
        }
        return Ok(true);
    }

    if let Some(viewer_opt) = &args.log {
        let logs = crate::app::logging::list_log_files(log_dir);
        if logs.is_empty() {
            ui::warning("No log files available.");
            return Ok(true);
        }

        let viewer = viewer_opt.clone().unwrap_or_else(|| {
            std::env::var("PAGER").unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad.exe".to_string()
                } else {
                    "less".to_string()
                }
            })
        });

        let target_file = if logs.len() == 1 {
            logs[0].clone()
        } else {
            let items: Vec<String> = logs
                .iter()
                .enumerate()
                .map(|(idx, path)| crate::app::logging::format_log_display(path, idx == 0))
                .collect();

            let prompt = "Select Log Session > ";
            let options: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
            if let Some(selected) = tui::runner::run_skim_simple(options, prompt) {
                if let Some(idx) = items.iter().position(|item| item == &selected) {
                    logs[idx].clone()
                } else {
                    return Ok(true);
                }
            } else {
                return Ok(true);
            }
        };

        let status = std::process::Command::new(&viewer)
            .arg(&target_file)
            .status();

        if let Err(e) = status {
            ui::error(format!("Failed to launch log viewer '{}': {}", viewer, e));
        }
        return Ok(true);
    }

    if args.remove_config {
        if config_file.exists() {
            std::fs::remove_file(config_file)?;
            ui::success("Configuration removed.");
        } else {
            ui::warning("No config file found.");
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
            ui::error(format!(
                "Failed to launch editor. Is '{}' installed? ({})",
                editor, e
            ));
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
        ui::success("Configuration saved.");
        ui::info("Syncing index...");
        let tracks = indexer::scan(cfg, false)?;
        indexer::save(&tracks)?;
        ui::success(format!("Index updated ({} tracks).", tracks.len()));
        return Ok(true);
    }
    if args.manage_dirs {
        if tui::run_manage_dirs_mode(cfg)? {
            config::save(cfg)?;
            ui::success("Configuration saved.");
            ui::info("Syncing index with new directories...");
            let tracks = indexer::scan(cfg, false)?;
            indexer::save(&tracks)?;
            ui::success(format!("Index updated ({} tracks).", tracks.len()));
        }
        return Ok(true);
    }
    Ok(false)
}
