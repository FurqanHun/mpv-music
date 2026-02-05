use anyhow::{Context, Result};
use directories::ProjectDirs;
use indicatif::{ProgressBar, ProgressStyle};
use lofty::prelude::*;
use lofty::probe::Probe;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

use crate::config::Config;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Track {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub mtime: u64,
    pub size: u64,
    pub media_type: String,
}

// split "mp3, flac" -> Set
fn to_set(exts: &[String]) -> HashSet<String> {
    exts.iter().map(|s| s.trim().to_lowercase()).collect()
}

pub fn scan(config: &Config, force: bool) -> Result<Vec<Track>> {
    if config.music_dirs.is_empty() {
        log::warn!("Scan aborted: No music directories configured.");
        eprintln!("   Run 'mpv-music --add-dir <PATH>' to add your music folder.");
        eprintln!("   Or use 'mpv-music --manage-dirs' for the menu.");
        return Ok(Vec::new());
    }

    log::info!("Starting library scan. Force reindex: {}", force);

    let audio_exts = to_set(&config.audio_exts);
    let video_exts = to_set(&config.video_exts);
    let playlist_exts = to_set(&config.playlist_exts);

    let old_cache: HashMap<String, Track> = if !force {
        log::debug!("Attempting to load existing cache for smart update");
        if let Ok((old_tracks, _)) = load_index() {
            log::info!("Cache loaded. Found {} existing entries", old_tracks.len());
            old_tracks
                .into_iter()
                .map(|t| (t.path.clone(), t))
                .collect()
        } else {
            log::debug!("No valid cache found. Proceeding with clean scan");
            HashMap::new()
        }
    } else {
        log::info!("Forced reindex requested. Ignoring existing cache");
        HashMap::new()
    };

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {pos} tracks ({per_sec})")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    // scan loop
    let tracks: Vec<Track> = config
        .music_dirs
        .iter()
        .flat_map(|dir| {
            log::info!("Walking directory: {:?}", dir);
            WalkDir::new(dir).into_iter().filter_map(|e| e.ok())
        })
        .par_bridge()
        .filter_map(|entry| {
            let path = entry.path();

            if !path.is_file() {
                return None;
            }

            log::trace!("Examining file: {:?}", path);

            let ext = path.extension()?.to_str()?.to_lowercase();

            let media_type = if audio_exts.contains(&ext) {
                "audio"
            } else if playlist_exts.contains(&ext) {
                "playlist"
            } else if config.video_ok && video_exts.contains(&ext) {
                "video"
            } else {
                // log::trace!("Skipping non-media extension: .{}", ext);
                return None;
            };

            pb.inc(1);

            let metadata = entry.metadata().ok()?;
            let mtime = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let size = metadata.len();
            let path_str = path.to_string_lossy().to_string();

            // smort check
            if let Some(old_track) = old_cache.get(&path_str) {
                if old_track.mtime == mtime && old_track.size == size {
                    log::debug!("Cache hit (Unchanged): {}", path_str);
                    return Some(old_track.clone());
                }
            }

            log::debug!("Cache miss/Dirty: Probing {}", path_str);

            let (mut title, mut artist, mut album, mut genre);

            if media_type == "playlist" {
                title = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                artist = "Playlist".to_string();
                album = "Playlists".to_string();
                genre = "Playlist".to_string();
            } else {
                title = String::new();
                artist = String::new();
                album = String::new();
                genre = String::new();

                match Probe::open(path).and_then(|p| p.read()) {
                    Ok(tagged_file) => {
                        if let Some(tag) = tagged_file
                            .primary_tag()
                            .or_else(|| tagged_file.first_tag())
                        {
                            title = tag.title().map(|s| s.to_string()).unwrap_or_default();
                            artist = tag.artist().map(|s| s.to_string()).unwrap_or_default();
                            album = tag.album().map(|s| s.to_string()).unwrap_or_default();
                            genre = tag.genre().map(|s| s.to_string()).unwrap_or_default();
                        }
                    }
                    Err(e) => {
                        log::warn!("Metadata probe failed for '{}': {}", path_str, e);
                    }
                }
            }

            if title.is_empty() {
                title = path.file_stem()?.to_string_lossy().to_string();
            }
            if artist.is_empty() {
                artist = "UNKNOWN".to_string();
            }
            if album.is_empty() {
                album = "UNKNOWN".to_string();
            }
            if genre.is_empty() {
                genre = "UNKNOWN".to_string();
            }

            Some(Track {
                path: path_str,
                title,
                artist,
                album,
                genre,
                mtime,
                size,
                media_type: media_type.to_string(),
            })
        })
        .collect();

    pb.finish_with_message(format!("Indexed {} tracks", tracks.len()));
    log::info!(
        "Indexing session finished. Total valid tracks found: {}",
        tracks.len()
    );

    if !config.music_dirs.is_empty() {
        println!();
    }

    Ok(tracks)
}

pub fn save(tracks: &[Track]) -> Result<()> {
    let dirs = ProjectDirs::from("com", "furqanhun", "mpv-music")
        .context("Could not determine data directory")?;

    let data_dir = dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;

    let index_path = dirs.data_dir().join("music_index.jsonl");
    log::info!(
        "Saving index ({} entries) to: {:?}",
        tracks.len(),
        index_path
    );

    let file = File::create(&index_path)?;
    let mut writer = BufWriter::new(file);

    for track in tracks {
        serde_json::to_writer(&mut writer, track)?;
        writeln!(writer)?;
    }

    writer.flush()?;
    log::debug!("Index flush to disk complete.");
    Ok(())
}

pub fn load_index() -> Result<(Vec<Track>, bool)> {
    let dirs = ProjectDirs::from("com", "furqanhun", "mpv-music")
        .context("Could not determine data directory")?;
    let index_path = dirs.data_dir().join("music_index.jsonl");

    if !index_path.exists() {
        log::debug!("No existing index file found at {:?}", index_path);
        return Ok((Vec::new(), false));
    }

    log::info!("Loading index file from: {:?}", index_path);
    let file = File::open(&index_path)?;
    let reader = BufReader::new(file);
    let mut tracks = Vec::new();
    let mut needs_repair = false;
    let mut line_count = 0;

    for line in std::io::BufRead::lines(reader) {
        line_count += 1;
        let l = line?;
        if l.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<Track>(&l) {
            Ok(t) => tracks.push(t),
            Err(e) => {
                log::warn!(
                    "Corruption detected on line {}: {}. Marking for surgical repair...",
                    line_count,
                    e
                );
                needs_repair = true;
            }
        }
    }

    if needs_repair {
        log::info!("Performing surgical repair on index (purging corrupt entries)...");
        save(&tracks)?;
    }

    log::debug!("Index loaded successfully. Loaded {} tracks.", tracks.len());
    Ok((tracks, needs_repair))
}
