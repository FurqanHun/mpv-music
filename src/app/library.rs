use crate::cli::Cli;
use crate::config::Config;
use crate::indexer::{self, Track};
use crate::player;
use crate::ui;
use anyhow::Result;
use std::path::PathBuf;

pub fn is_local_cli_mode_requested(args: &Cli) -> bool {
    args.genre.is_some()
        || args.artist.is_some()
        || args.album.is_some()
        || args.title.is_some()
        || args.playlist.is_some()
        || args.play_all
        || args
            .target
            .as_ref()
            .is_some_and(|t| !crate::search::unified::is_direct_playback_target(t))
}

fn load_or_scan_library(cfg: &Config, args: &Cli) -> Result<Vec<Track>> {
    let (mut loaded_tracks, was_repaired) = indexer::load_index()?;

    if args.reindex {
        log::info!("Rebuilding index (Full)...");
        loaded_tracks = indexer::scan(cfg, true)?;
        indexer::save(&loaded_tracks)?;
    } else if args.refresh_index || was_repaired {
        if was_repaired {
            log::info!("Index corruption healed. Syncing...");
        } else {
            log::info!("Refreshing index...");
        }
        loaded_tracks = indexer::scan(cfg, false)?;
        indexer::save(&loaded_tracks)?;
    } else if loaded_tracks.is_empty() {
        log::info!("Index empty. First scan...");
        loaded_tracks = indexer::scan(cfg, true)?;
        indexer::save(&loaded_tracks)?;
    }

    Ok(loaded_tracks)
}

pub fn load_or_scan_tracks(
    cfg: &Config,
    args: &Cli,
    extra_mpv_args: &[String],
) -> Result<Option<Vec<Track>>> {
    let tracks: Vec<Track>;

    if let Some(target) = args.target.as_deref() {
        let path = PathBuf::from(target);

        if path.is_dir() {
            let target_canonical = dunce::canonicalize(&path).unwrap_or(path);
            let target_str = target_canonical.to_string_lossy();

            let mut temp_cfg = cfg.clone();
            temp_cfg.music_dirs = vec![target_canonical.clone()];

            let session_tracks = indexer::scan(&temp_cfg, true)?;

            if session_tracks.is_empty() {
                ui::warning(format!("No music files found in: {}", target_str));
                return Ok(None);
            }
            tracks = session_tracks;
        } else if crate::search::unified::is_direct_playback_target(target) {
            if target.starts_with("http://") || target.starts_with("https://") {
                ui::info(format!(
                    "Loading stream from '{}' and launching player...",
                    target
                ));
            }
            player::play(target, cfg, extra_mpv_args)?;
            return Ok(None);
        } else {
            tracks = load_or_scan_library(cfg, args)?;
        }
    } else {
        tracks = load_or_scan_library(cfg, args)?;
    }

    if tracks.is_empty() && is_local_cli_mode_requested(args) {
        ui::warning(
            "No music found in local library. Run with --manage-dirs or add dirs to config.",
        );
        #[cfg(windows)]
        ui::prompt_exit();
        return Ok(None);
    }

    Ok(Some(tracks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_is_local_cli_mode_requested() {
        let args_plain = Cli::parse_from(["mpv-music"]);
        assert!(!is_local_cli_mode_requested(&args_plain));

        let args_genre = Cli::parse_from(["mpv-music", "--genre", "rock"]);
        assert!(is_local_cli_mode_requested(&args_genre));

        let args_play_all = Cli::parse_from(["mpv-music", "--play-all"]);
        assert!(is_local_cli_mode_requested(&args_play_all));
    }
}
