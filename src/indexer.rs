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

pub(crate) fn parse_filename_metadata(filename: &str) -> (String, String) {
    if let Some((parsed_artist, parsed_title)) = filename.split_once(" - ") {
        (
            parsed_artist.trim().to_string(),
            parsed_title.trim().to_string(),
        )
    } else {
        (String::new(), filename.to_string())
    }
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

    // load existing index for caching and smart recovery
    let (old_cache, recovery_map) = if !force {
        if let Ok((old_tracks, _)) = load_index() {
            log::info!("Cache loaded. Found {} existing entries", old_tracks.len());

            let mut path_map = HashMap::new();
            let mut attr_map = HashMap::new();

            for t in old_tracks {
                // primary cache: lookup by exact path
                path_map.insert(t.path.clone(), t.clone());

                // secondary cache: lookup by attributes (size + mtime + filename)
                if let Some(fname) = std::path::Path::new(&t.path).file_name() {
                    let key = (t.size, t.mtime, fname.to_string_lossy().to_string());
                    attr_map.entry(key).or_insert(t);
                }
            }
            (path_map, attr_map)
        } else {
            (HashMap::new(), HashMap::new())
        }
    } else {
        log::info!("Forced reindex requested. Ignoring existing cache");
        (HashMap::new(), HashMap::new())
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

            // log::trace!("Examining file: {:?}", path);

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
            if let Some(old_track) = old_cache
                .get(&path_str)
                .filter(|t| t.mtime == mtime && t.size == size)
            {
                log::debug!("Cache hit (Unchanged): {}", path_str);
                return Some(old_track.clone());
            }

            let filename = path.file_name()?.to_string_lossy().to_string();
            let recovery_key = (size, mtime, filename);
            if let Some(recovered) = recovery_map.get(&recovery_key) {
                log::debug!("Smart Recovery (Moved/Renamed): {}", path_str);
                let mut new_entry = recovered.clone();
                new_entry.path = path_str;
                return Some(new_entry);
            }

            log::debug!("Cache miss: Probing {}", path_str);

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
                let filename = path.file_stem()?.to_string_lossy().to_string();

                let (parsed_artist, parsed_title) = parse_filename_metadata(&filename);

                title = parsed_title;
                if artist.is_empty() && !parsed_artist.is_empty() {
                    artist = parsed_artist;
                }
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
                    "Corruption detected on line {}: {}. Marking for repair...",
                    line_count,
                    e
                );
                needs_repair = true;
            }
        }
    }

    if needs_repair {
        log::info!("Performing surgical repair on index...");
        save(&tracks)?;
    }

    log::debug!("Index loaded successfully. Loaded {} tracks.", tracks.len());
    Ok((tracks, needs_repair))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_format() {
        let (artist, title) = parse_filename_metadata("Kendrick Lamar - HUMBLE.mp3");
        assert_eq!(artist, "Kendrick Lamar");
        assert_eq!(title, "HUMBLE.mp3");
    }

    #[test]
    fn test_parse_with_whitespace() {
        let (artist, title) = parse_filename_metadata("  Daft Punk  -  Get Lucky  ");
        assert_eq!(artist, "Daft Punk");
        assert_eq!(title, "Get Lucky");
    }

    #[test]
    fn test_parse_no_artist() {
        let (artist, title) = parse_filename_metadata("JustASong.flac");
        assert_eq!(artist, "");
        assert_eq!(title, "JustASong.flac");
    }

    #[test]
    fn test_parse_multiple_dashes() {
        // Should only split on FIRST " - "
        let (artist, title) = parse_filename_metadata("Arctic Monkeys - Do I Wanna Know? - Live");
        assert_eq!(artist, "Arctic Monkeys");
        assert_eq!(title, "Do I Wanna Know? - Live");
    }

    #[test]
    fn test_parse_unicode() {
        let (artist, title) = parse_filename_metadata("Ado - うっせぇわ");
        assert_eq!(artist, "Ado");
        assert_eq!(title, "うっせぇわ");
    }

    #[test]
    fn test_parse_special_characters() {
        let (artist, title) = parse_filename_metadata("AC/DC - Back In Black");
        assert_eq!(artist, "AC/DC");
        assert_eq!(title, "Back In Black");
    }

    #[test]
    fn test_parse_numbers() {
        let (artist, title) = parse_filename_metadata("Twenty One Pilots - Stressed Out");
        assert_eq!(artist, "Twenty One Pilots");
        assert_eq!(title, "Stressed Out");
    }

    #[test]
    fn test_parse_single_character_artist() {
        let (artist, title) = parse_filename_metadata("K - Song Title");
        assert_eq!(artist, "K");
        assert_eq!(title, "Song Title");
    }

    #[test]
    fn test_track_creation() {
        let track = Track {
            path: "/music/song.mp3".to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            genre: "Test Genre".to_string(),
            mtime: 1234567890,
            size: 1024,
            media_type: "audio".to_string(),
        };

        assert_eq!(track.artist, "Test Artist");
        assert_eq!(track.title, "Test Song");
        assert_eq!(track.media_type, "audio");
        assert_eq!(track.size, 1024);
    }

    #[test]
    fn test_track_serialization() {
        let track = Track {
            path: "/music/test.mp3".to_string(),
            title: "Title".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            genre: "Genre".to_string(),
            mtime: 12345,
            size: 1000,
            media_type: "audio".to_string(),
        };

        // Should be able to serialize to JSON
        let json = serde_json::to_string(&track);
        assert!(json.is_ok());
    }

    #[test]
    fn test_track_deserialization() {
        let json = r#"{
            "path": "/test.mp3",
            "title": "Test",
            "artist": "Artist",
            "album": "Album",
            "genre": "Rock",
            "mtime": 123,
            "size": 500,
            "media_type": "audio"
        }"#;

        let track: Result<Track, _> = serde_json::from_str(json);
        assert!(track.is_ok());

        let track = track.unwrap();
        assert_eq!(track.artist, "Artist");
        assert_eq!(track.genre, "Rock");
    }

    #[test]
    fn test_to_set_function() {
        let exts = vec!["mp3".to_string(), "flac".to_string(), "wav".to_string()];
        let set = to_set(&exts);

        assert!(set.contains("mp3"));
        assert!(set.contains("flac"));
        assert!(set.contains("wav"));
        assert!(!set.contains("mp4"));
    }

    #[test]
    fn test_to_set_case_insensitive() {
        let exts = vec!["MP3".to_string(), "FLAC".to_string()];
        let set = to_set(&exts);

        // to_set converts to lowercase
        assert!(set.contains("mp3"));
        assert!(set.contains("flac"));
    }

    #[test]
    fn test_to_set_empty() {
        let exts: Vec<String> = vec![];
        let set = to_set(&exts);

        assert!(set.is_empty());
    }
}
