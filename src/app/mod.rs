pub mod filter;
pub mod flags;
pub mod library;
pub mod logging;

pub use flags::{apply_cli_overrides, handle_dir_flags, handle_utility_flags};

use crate::cli::Cli;
use crate::config;
use crate::indexer;
use crate::player;
use crate::tui;
#[cfg(feature = "update")]
use crate::update;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

pub fn run(args: Cli) -> Result<()> {
    let extra_mpv_args = args.mpv_args.as_deref().unwrap_or(&[]);

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

    // utility flags: handled before logger init to prevent file locks
    if handle_utility_flags(&args, &config_file, log_dir)? {
        return Ok(());
    }

    let mut cfg = config::load(config_path_override.clone())?;

    // init logger
    let _logger_handle = logging::init(&args, &cfg, log_dir)?;

    log::info!("Starting MPV-Music...");
    log::debug!("CLI Args: {:?}", args);
    log::debug!("Config loaded from: {:?}", config_file);

    apply_cli_overrides(&mut cfg, &args)?;

    #[cfg(feature = "update")]
    if args.update {
        update::update_self(args.yes)?;
        return Ok(());
    }

    if let Some(None) = args.radio {
        log::info!("Empty radio flag. Opening Radio Picker.");
        tui::run_radio_mode(&cfg, extra_mpv_args, None)?;
        return Ok(());
    }

    if let Some(Some(choice)) = args.radio {
        tui::run_radio_mode(&cfg, extra_mpv_args, Some(&choice))?;
        return Ok(());
    }

    if handle_dir_flags(&mut cfg, &args)? {
        return Ok(());
    }

    let tracks_opt = library::load_or_scan_tracks(&cfg, &args, extra_mpv_args)?;
    let mut tracks = match tracks_opt {
        Some(t) => t,
        None => return Ok(()),
    };

    // entry point shortcuts
    if let Some(None) = args.genre {
        log::info!("Empty genre flag. Opening Genre Picker.");
        tui::run_tag_mode(&tracks, &cfg, Some("genre"), extra_mpv_args)?;
        return Ok(());
    }
    if let Some(None) = args.artist {
        log::info!("Empty artist flag. Opening Artist Picker.");
        tui::run_tag_mode(&tracks, &cfg, Some("artist"), extra_mpv_args)?;
        return Ok(());
    }
    if let Some(None) = args.album {
        log::info!("Empty album flag. Opening Album Picker.");
        tui::run_tag_mode(&tracks, &cfg, Some("album"), extra_mpv_args)?;
        return Ok(());
    }
    if let Some(None) = args.title {
        log::info!("Empty title flag. Opening Track Mode.");
        tui::run_track_mode(&tracks, &cfg, extra_mpv_args)?;
        return Ok(());
    }
    if let Some(search_input) = args.search {
        if let Some(query) = search_input {
            tui::run_search_mode(&cfg, Some(query), extra_mpv_args)?;
        } else {
            log::info!("Empty search flag. Opening YouTube Search.");
            tui::run_search_mode(&cfg, None, extra_mpv_args)?;
        }
        return Ok(());
    }
    if let Some(None) = args.playlist {
        log::info!("Empty playlist flag. Opening Playlist Picker.");
        tui::run_playlist_mode(&tracks, &cfg, extra_mpv_args)?;
        return Ok(());
    }

    // main search and filter logic
    if filter::has_filter_flags(&args) {
        filter::handle_cli_filters(&tracks, &cfg, &args, extra_mpv_args)?;
        return Ok(());
    }

    // default modes
    if args.play_all {
        let paths: Vec<String> = tracks.iter().map(|t| t.path.clone()).collect();
        player::play_files(&paths, &cfg, extra_mpv_args)?;
    } else if let Some(maybe_val) = args.playlist {
        if let Some(playlist_name) = maybe_val {
            let name_lower = playlist_name.to_lowercase();
            let matches: Vec<&indexer::Track> = tracks
                .iter()
                .filter(|t| t.is_playlist() && t.title.to_lowercase().contains(&name_lower))
                .collect();

            if matches.len() == 1 {
                log::info!(
                    "Single playlist match found: {}. Playing directly.",
                    matches[0].title
                );
                player::play(&matches[0].path, &cfg, extra_mpv_args)?;
                return Ok(());
            }
        }
        tui::run_playlist_mode(&tracks, &cfg, extra_mpv_args)?;
    } else {
        tui::run_main_menu(&mut tracks, &mut cfg, extra_mpv_args)?;
    }

    Ok(())
}
