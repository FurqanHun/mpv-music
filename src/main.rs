mod cli;
mod config;
mod dep_check;
mod indexer;
mod player;
mod search;
mod tui;
mod update;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use directories::ProjectDirs;
use flexi_logger::{FileSpec, Logger, WriteMode, style};
use std::collections::HashSet;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = Cli::parse();

    // deterministic paths
    let dirs = ProjectDirs::from("com", "furqanhun", "mpv-music")
        .context("Could not determine system paths")?;
    let log_dir = dirs.data_dir();
    let config_dir = dirs.config_dir();
    let config_file = config_dir.join("config.toml");

    let config_path_override = if let Some(Some(path)) = &args.config {
        Some(PathBuf::from(path))
    } else {
        None
    };

    // utility flags
    let log_file_path = log_dir.join("mpv-music.log");
    if args.remove_log {
        if log_file_path.exists() {
            std::fs::remove_file(&log_file_path)?;
            println!("Log file nuked.");
        } else {
            println!("No log file available.");
        }
        return Ok(());
    }
    if let Some(viewer_opt) = args.log {
        let viewer = viewer_opt
            .unwrap_or_else(|| std::env::var("PAGER").unwrap_or_else(|_| "less".to_string()));
        if log_file_path.exists() {
            std::process::Command::new(viewer)
                .arg(&log_file_path)
                .status()?;
        } else {
            println!("No log file available.");
        }
        return Ok(());
    }
    if args.remove_config {
        if config_file.exists() {
            std::fs::remove_file(&config_file)?;
            println!("Config removed.");
        } else {
            println!("No config file found.");
        }
        return Ok(());
    }

    // handle editor
    if let Some(editor_opt) = args.config {
        if !config_file.exists() {
            log::info!(
                "Config file not found. Generating default at {:?}...",
                config_file
            );
            let _ = config::load(None)?;
        }

        let editor = editor_opt
            .unwrap_or_else(|| std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string()));

        log::info!("Opening config with editor: {}", editor);

        let status = std::process::Command::new(&editor)
            .arg(&config_file)
            .status();

        if let Err(e) = status {
            eprintln!(
                "Error: Failed to launch editor. Is '{}' installed? ({})",
                editor, e
            );
        }
        return Ok(());
    }

    let mut cfg = config::load(config_path_override.clone())?;

    // init logger
    let log_filter = if cfg.enable_file_logging {
        if args.debug {
            "mpv_music=debug, warn"
        } else {
            "mpv_music=info, warn"
        }
    } else if args.debug {
        "mpv_music=debug, warn"
    } else if args.verbose > 0 {
        "mpv_music=info, warn"
    } else {
        "mpv_music=error, warn"
    };

    std::fs::create_dir_all(log_dir)?;
    let mut logger = Logger::try_with_str(log_filter)?.format_for_stderr(|w, _now, record| {
        let level = record.level();
        write!(
            w,
            "[{}] {}",
            style(level).paint(level.as_str()),
            record.args()
        )
    });
    if cfg.enable_file_logging {
        let log_path = log_dir.join("mpv-music.log");
        if log_path.exists() {
            let _ = std::fs::remove_file(&log_path);
        }

        logger = logger
            .log_to_file(
                FileSpec::default()
                    .directory(log_dir)
                    .basename("mpv-music")
                    .suffix("log")
                    .use_timestamp(false),
            )
            .format_for_files(flexi_logger::opt_format)
            .write_mode(WriteMode::Direct);
    }

    let is_interactive = args.target.is_none() || args.refresh_index;
    if args.debug {
        logger = logger.duplicate_to_stderr(flexi_logger::Duplicate::All);
    } else if args.verbose > 0 {
        logger = logger.duplicate_to_stderr(flexi_logger::Duplicate::Info);
    } else if is_interactive {
        logger = logger.duplicate_to_stderr(flexi_logger::Duplicate::Error);
    }

    let _logger_handle = logger.start()?;

    log::info!("Starting MPV-Music...");
    log::debug!("CLI Args: {:?}", args);
    log::debug!("Config loaded from: {:?}", config_file);

    dep_check::check(&mut cfg)?;

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
    #[cfg(feature = "update")]
    if args.update {
        update::update_self()?;
        return Ok(());
    }
    if let Some(ref mode) = args.loop_arg {
        cfg.loop_mode = mode.clone(); // Handles "inf", "5", etc.
    }
    if args.no_loop {
        cfg.loop_mode = "no".to_string();
    }
    if args.repeat {
        cfg.loop_mode = "track".to_string();
    }

    if let Some(ref extensions) = args.ext {
        cfg.audio_exts = extensions
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // dir management
    let mut config_changed = false;
    if let Some(dirs) = &args.add_dir {
        for dir in dirs {
            if tui::add_directory(&mut cfg, dir.clone())? {
                config_changed = true;
            }
        }
    }
    if let Some(dirs) = &args.remove_dir {
        for dir in dirs {
            if tui::remove_directory(&mut cfg, dir.clone())? {
                config_changed = true;
            }
        }
    }
    if config_changed {
        config::save(&cfg)?;
        println!("Configuration saved. Syncing index...");
        let tracks = indexer::scan(&cfg, false)?;
        indexer::save(&tracks)?;
        return Ok(());
    }
    if args.manage_dirs {
        if tui::run_manage_dirs_mode(&mut cfg)? {
            config::save(&cfg)?;
            println!("Configuration saved.");
            println!("Syncing index with new directories...");
            let tracks = indexer::scan(&cfg, false)?;
            indexer::save(&tracks)?;
        }
        return Ok(());
    }

    // tracks population (session vs persistence)
    let mut tracks: Vec<indexer::Track>;

    if let Some(target) = args.target.clone() {
        let path = PathBuf::from(&target);

        if path.is_dir() {
            log::info!("Session started for directory: {:?}", path);
            let target_canonical = std::fs::canonicalize(&path).unwrap_or(path.clone());
            let target_str = target_canonical.to_string_lossy();

            let mut temp_cfg = cfg.clone();
            temp_cfg.music_dirs = vec![target_canonical.clone()];

            tracks = indexer::scan(&temp_cfg, true)?;

            if tracks.is_empty() {
                eprintln!("No music files found in: {}", target_str);
                return Ok(());
            }
        } else {
            player::play(&target, &cfg)?;
            return Ok(());
        }
    } else {
        let (mut loaded_tracks, was_repaired) = indexer::load_index()?;

        if args.reindex {
            log::info!("Rebuilding index (Full)...");
            loaded_tracks = indexer::scan(&cfg, true)?;
            indexer::save(&loaded_tracks)?;
        } else if args.refresh_index || was_repaired {
            if was_repaired {
                log::info!("Index corruption healed. Syncing...");
            } else {
                log::info!("Refreshing index...");
            }
            loaded_tracks = indexer::scan(&cfg, false)?;
            indexer::save(&loaded_tracks)?;
        } else if loaded_tracks.is_empty() {
            log::info!("Index empty. First scan...");
            loaded_tracks = indexer::scan(&cfg, true)?;
            indexer::save(&loaded_tracks)?;
        }

        tracks = loaded_tracks;
    }

    if tracks.is_empty() {
        eprintln!("No music found.");
        return Ok(());
    }

    // enry point shortcuts
    if let Some(None) = args.genre {
        log::info!("Empty genre flag. Opening Genre Picker.");
        tui::run_tag_mode(&tracks, &cfg, Some("genre"))?;
        return Ok(());
    }
    if let Some(None) = args.artist {
        log::info!("Empty artist flag. Opening Artist Picker.");
        tui::run_tag_mode(&tracks, &cfg, Some("artist"))?;
        return Ok(());
    }
    if let Some(None) = args.album {
        log::info!("Empty album flag. Opening Album Picker.");
        tui::run_tag_mode(&tracks, &cfg, Some("album"))?;
        return Ok(());
    }
    if let Some(None) = args.title {
        log::info!("Empty title flag. Opening Track Mode.");
        tui::run_track_mode(&tracks, &cfg)?;
        return Ok(());
    }
    if let Some(search_input) = args.search {
        if let Some(query) = search_input {
            tui::run_search_mode(&cfg, Some(query))?;
        } else {
            log::info!("Empty search flag. Opening YouTube Search.");
            tui::run_search_mode(&cfg, None)?;
        }
        return Ok(());
    }
    if let Some(None) = args.playlist {
        log::info!("Empty playlist flag. Opening Playlist Picker.");
        tui::run_playlist_mode(&tracks, &cfg)?;
        return Ok(());
    }

    // main search and filter logic
    if args.genre.is_some() || args.artist.is_some() || args.album.is_some() || args.title.is_some()
    {
        let is_multi_value_search = args
            .artist
            .as_ref()
            .and_then(|o| o.as_ref())
            .is_some_and(|s| s.contains(','))
            || args
                .genre
                .as_ref()
                .and_then(|o| o.as_ref())
                .is_some_and(|s| s.contains(','))
            || args
                .album
                .as_ref()
                .and_then(|o| o.as_ref())
                .is_some_and(|s| s.contains(','))
            || args
                .title
                .as_ref()
                .and_then(|o| o.as_ref())
                .is_some_and(|s| s.contains(','));

        // stage 1: exact match
        let mut filtered = if !is_multi_value_search {
            tui::apply_cli_filters(&tracks, &args, true)
        } else {
            Vec::new()
        };

        // stage 2: partial match / ambiguity handling
        if filtered.is_empty() {
            log::debug!("Exact match skipped or failed, trying partial...");
            let partials = tui::apply_cli_filters(&tracks, &args, false);

            if partials.is_empty() {
                eprintln!("No match.");
                return Ok(());
            }

            // identify active tag
            let mut unique_options: HashSet<String> = HashSet::new();
            let mut active_key = "";
            if args.artist.is_some() {
                active_key = "artist";
            } else if args.genre.is_some() {
                active_key = "genre";
            } else if args.album.is_some() {
                active_key = "album";
            } else if args.title.is_some() {
                active_key = "title";
            }

            for t in &partials {
                match active_key {
                    "artist" => {
                        unique_options.insert(t.artist.clone());
                    }
                    "genre" => {
                        unique_options.insert(t.genre.clone());
                    }
                    "album" => {
                        unique_options.insert(t.album.clone());
                    }
                    "title" => {
                        unique_options.insert(t.title.clone());
                    }
                    _ => {}
                }
            }

            if !args.play_all && unique_options.len() > 1 && !active_key.is_empty() {
                let mut options_vec: Vec<String> = unique_options.into_iter().collect();
                options_vec.sort();

                if let Some(selected_vals) = tui::run_skim_multi_selection(
                    options_vec,
                    &format!("Which {}s? (TAB to select multiple) > ", active_key),
                ) {
                    let selected_set: HashSet<String> = selected_vals.into_iter().collect();

                    filtered = partials
                        .into_iter()
                        .filter(|t| {
                            let val = match active_key {
                                "artist" => &t.artist,
                                "genre" => &t.genre,
                                "album" => &t.album,
                                "title" => &t.title,
                                _ => "",
                            };
                            selected_set.contains(val)
                        })
                        .collect();
                } else {
                    return Ok(());
                }
            } else {
                filtered = partials;
            }
        }

        if filtered.is_empty() {
            eprintln!("No match.");
            return Ok(());
        }

        if filtered.len() == 1 {
            log::info!("Single match found. Playing directly.");
            player::play(&filtered[0].path, &cfg)?;
            return Ok(());
        }

        println!("Found {} matching tracks.", filtered.len());
        if args.play_all || args.title.is_some() {
            let paths: Vec<String> = filtered.iter().map(|t| t.path.clone()).collect();
            player::play_files(&paths, &cfg)?;
        } else {
            tui::run_post_filter_action(&filtered, &cfg)?;
        }
        return Ok(());
    }

    // default modes
    if args.play_all {
        let paths: Vec<String> = tracks.iter().map(|t| t.path.clone()).collect();
        player::play_files(&paths, &cfg)?;
    } else if let Some(maybe_val) = args.playlist {
        if let Some(playlist_name) = maybe_val {
            let name_lower = playlist_name.to_lowercase();
            let matches: Vec<&indexer::Track> = tracks
                .iter()
                .filter(|t| {
                    t.media_type == "playlist" && t.title.to_lowercase().contains(&name_lower)
                })
                .collect();

            if matches.len() == 1 {
                log::info!(
                    "Single playlist match found: {}. Playing directly.",
                    matches[0].title
                );
                player::play(&matches[0].path, &cfg)?;
                return Ok(());
            }
        }
        tui::run_playlist_mode(&tracks, &cfg)?;
    } else {
        tui::run_main_menu(&mut tracks, &mut cfg)?;
    }

    Ok(())
}
