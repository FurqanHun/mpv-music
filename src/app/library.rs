use crate::cli::Cli;
use crate::config::Config;
use crate::indexer::{self, Track};
use crate::player;
use anyhow::Result;
use std::path::PathBuf;

pub fn is_local_cli_mode_requested(args: &Cli) -> bool {
    args.genre.is_some()
        || args.artist.is_some()
        || args.album.is_some()
        || args.title.is_some()
        || args.playlist.is_some()
        || args.play_all
}

pub fn load_or_scan_tracks(
    cfg: &Config,
    args: &Cli,
    extra_mpv_args: &[String],
) -> Result<Option<Vec<Track>>> {
    let tracks: Vec<Track>;

    if let Some(target) = args.target.clone() {
        let path = PathBuf::from(&target);

        if path.is_dir() {
            log::info!("Session started for directory: {:?}", path);
            let target_canonical = dunce::canonicalize(&path).unwrap_or(path.clone());
            let target_str = target_canonical.to_string_lossy();

            let mut temp_cfg = cfg.clone();
            temp_cfg.music_dirs = vec![target_canonical.clone()];

            let session_tracks = indexer::scan(&temp_cfg, true)?;

            if session_tracks.is_empty() {
                eprintln!(
                    "\x1b[33;1m[Warning]\x1b[0m No music files found in: {}",
                    target_str
                );
                return Ok(None);
            }
            tracks = session_tracks;
        } else {
            player::play(&target, cfg, extra_mpv_args)?;
            return Ok(None);
        }
    } else {
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

        tracks = loaded_tracks;
    }

    if tracks.is_empty() && is_local_cli_mode_requested(args) {
        eprintln!(
            "\x1b[33;1m[Warning]\x1b[0m No music found in local library. Run with --manage-dirs or add dirs to config."
        );
        if cfg!(windows) {
            eprintln!("\nPress Enter to exit...");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
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
