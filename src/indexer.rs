use anyhow::{Context, Result};
use directories::ProjectDirs;
use indicatif::{ProgressBar, ProgressStyle};
use lofty::config::ParseOptions;
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
use crate::ui;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    #[default]
    Audio,
    Video,
    Playlist,
}

impl MediaType {
    pub fn is_video(&self) -> bool {
        matches!(self, Self::Video)
    }

    pub fn is_playlist(&self) -> bool {
        matches!(self, Self::Playlist)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Playlist => "playlist",
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Track {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub mtime: u64,
    pub size: u64,
    pub media_type: MediaType,
}

impl Track {
    pub fn is_playlist(&self) -> bool {
        self.media_type.is_playlist()
    }

    pub fn is_video(&self) -> bool {
        self.media_type.is_video()
    }
}

fn to_set(exts: &[String]) -> HashSet<String> {
    exts.iter().map(|s| s.trim().to_lowercase()).collect()
}

pub(crate) fn strip_leading_track_number(filename: &str) -> &str {
    let trimmed = filename.trim_start();
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return trimmed;
    }

    let mut i = 0;
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1].is_ascii_digit() {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    } else {
        let mut digits_before_dash = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            digits_before_dash += 1;
            i += 1;
        }

        if digits_before_dash > 0 && i < bytes.len() && bytes[i] == b'-' {
            let next_i = i + 1;
            let mut digits_after_dash = 0;
            let mut j = next_i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                digits_after_dash += 1;
                j += 1;
            }
            if digits_before_dash <= 2 && digits_after_dash > 0 && digits_after_dash <= 3 {
                i = j;
            }
        } else if digits_before_dash == 0 || digits_before_dash > 3 {
            return trimmed;
        }
    }

    if i == 0 {
        return trimmed;
    }

    let remainder = &trimmed[i..];
    if let Some(rest) = remainder.strip_prefix(" - ") {
        rest.trim_start()
    } else if let Some(rest) = remainder.strip_prefix(". ") {
        rest.trim_start()
    } else if let Some(rest) = remainder.strip_prefix("._") {
        rest.trim_start()
    } else if let Some(rest) = remainder.strip_prefix('.') {
        rest.trim_start()
    } else if let Some(rest) = remainder.strip_prefix('_') {
        rest.trim_start()
    } else if let Some(rest) = remainder.strip_prefix('-') {
        rest.trim_start()
    } else if let Some(rest) = remainder.strip_prefix(' ') {
        if !remainder.contains(" - ") {
            rest.trim_start()
        } else {
            trimmed
        }
    } else {
        trimmed
    }
}

pub(crate) fn parse_filename_metadata(filename: &str) -> (String, String) {
    let cleaned = strip_leading_track_number(filename);
    if let Some((parsed_artist, parsed_title)) = cleaned.split_once(" - ") {
        let artist_trimmed = parsed_artist.trim();
        if artist_trimmed.chars().all(|c| c.is_ascii_digit()) {
            (String::new(), parsed_title.trim().to_string())
        } else {
            (artist_trimmed.to_string(), parsed_title.trim().to_string())
        }
    } else {
        (String::new(), cleaned.trim().to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatternToken {
    Artist,
    Title,
    Album,
    Track,
    Genre,
    Ignore,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ExtractedMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
}

pub(crate) fn parse_custom_pattern(filename: &str, pattern: &str) -> Option<ExtractedMetadata> {
    let mut tokens: Vec<(PatternToken, Option<String>)> = Vec::new();
    let mut prefix: Option<String> = None;
    let mut suffix: Option<String> = None;

    let mut rem = pattern;
    let mut current_literal = String::new();
    let mut has_token = false;

    while !rem.is_empty() {
        if let Some(start_brace) = rem.find('{') {
            current_literal.push_str(&rem[..start_brace]);
            let after_start = &rem[start_brace + 1..];
            if let Some(end_brace) = after_start.find('}') {
                let raw_token = &after_start[..end_brace];
                if raw_token.contains('{') {
                    current_literal.push('{');
                    rem = after_start;
                    continue;
                }

                let token_name = raw_token.trim().to_ascii_lowercase();
                let token = match token_name.as_str() {
                    "artist" => PatternToken::Artist,
                    "title" => PatternToken::Title,
                    "album" => PatternToken::Album,
                    "track" | "num" => PatternToken::Track,
                    "genre" => PatternToken::Genre,
                    _ => PatternToken::Ignore,
                };

                if !has_token {
                    if !current_literal.is_empty() {
                        prefix = Some(std::mem::take(&mut current_literal));
                    }
                } else if let Some((_, prev_delim)) = tokens.last_mut() {
                    *prev_delim = Some(std::mem::take(&mut current_literal));
                }

                tokens.push((token, None));
                has_token = true;
                rem = &after_start[end_brace + 1..];
            } else {
                current_literal.push_str(rem);
                break;
            }
        } else {
            current_literal.push_str(rem);
            break;
        }
    }

    if !current_literal.is_empty() {
        if has_token {
            suffix = Some(current_literal);
        } else {
            return None;
        }
    }

    if tokens.is_empty() {
        return None;
    }

    let mut file_rem = filename.trim();
    if let Some(p) = &prefix {
        file_rem = file_rem.strip_prefix(p)?.trim_start();
    }
    if let Some(s) = &suffix {
        if let Some(stripped) = file_rem.strip_suffix(s) {
            file_rem = stripped.trim_end();
        } else if s.starts_with('.') && !s[1..].contains('.') && !file_rem.ends_with(s) {
            // Tolerates file extensions specified in user pattern against stem filenames
        } else {
            return None;
        }
    }

    let mut meta = ExtractedMetadata::default();
    let set_token_val = |meta: &mut ExtractedMetadata, tok: PatternToken, val: &str| {
        let v = val.trim().to_string();
        match tok {
            PatternToken::Title => meta.title = v,
            PatternToken::Artist => meta.artist = v,
            PatternToken::Album => meta.album = v,
            PatternToken::Genre => meta.genre = v,
            PatternToken::Track | PatternToken::Ignore => {}
        }
    };

    if tokens.len() == 1 {
        set_token_val(&mut meta, tokens[0].0, file_rem);
        return Some(meta);
    }

    let first_is_title_only = tokens[0].0 == PatternToken::Title
        && tokens[1..].iter().all(|(t, _)| *t != PatternToken::Title);

    if first_is_title_only {
        for i in (1..tokens.len()).rev() {
            let delim = tokens[i - 1].1.as_deref()?;
            let (rest, val) = file_rem.rsplit_once(delim)?;
            set_token_val(&mut meta, tokens[i].0, val);
            file_rem = rest;
        }
        set_token_val(&mut meta, tokens[0].0, file_rem);
    } else {
        for (tok, delim_opt) in tokens.iter().take(tokens.len() - 1) {
            let delim = delim_opt.as_deref()?;
            let (val, rest) = file_rem.split_once(delim)?;
            set_token_val(&mut meta, *tok, val);
            file_rem = rest;
        }
        set_token_val(&mut meta, tokens.last()?.0, file_rem);
    }

    Some(meta)
}

pub(crate) fn parse_filename_with_pattern(
    filename: &str,
    pattern: &str,
) -> (String, String, String, String) {
    let p_trimmed = pattern.trim();
    if !p_trimmed.is_empty()
        && !p_trimmed.eq_ignore_ascii_case("default")
        && let Some(meta) = parse_custom_pattern(filename, p_trimmed)
        && !meta.title.is_empty()
    {
        return (meta.title, meta.artist, meta.album, meta.genre);
    }

    let (parsed_artist, parsed_title) = parse_filename_metadata(filename);
    (parsed_title, parsed_artist, String::new(), String::new())
}

pub(crate) fn infer_parent_dirs_metadata(
    path: &std::path::Path,
    music_dirs: &[std::path::PathBuf],
) -> (Option<String>, Option<String>) {
    let parent = match path.parent() {
        Some(p) => p,
        None => return (None, None),
    };

    let is_music_dir = |p: &std::path::Path| {
        music_dirs.iter().any(|d| {
            d == p
                || d.components().eq(p.components())
                || match (d.canonicalize(), p.canonicalize()) {
                    (Ok(d_canon), Ok(p_canon)) => d_canon == p_canon,
                    _ => false,
                }
        })
    };

    if is_music_dir(parent) {
        return (None, None);
    }

    let parent_name = match parent.file_name() {
        Some(name) => name.to_string_lossy().trim().to_string(),
        None => return (None, None),
    };

    if parent_name.is_empty() {
        return (None, None);
    }

    if let Some(gp) = parent.parent()
        && !is_music_dir(gp)
        && let Some(gp_name) = gp.file_name()
    {
        let gp_artist = gp_name.to_string_lossy().trim().to_string();
        if !gp_artist.is_empty() {
            return (Some(gp_artist), Some(parent_name));
        }
    }

    (Some(parent_name), None)
}

pub fn scan(config: &Config, force: bool) -> Result<Vec<Track>> {
    if config.music_dirs.is_empty() {
        ui::warning("No music directories configured.");
        ui::suggestion(
            "Run 'mpv-music --add-dir <PATH>' or 'mpv-music --manage-dirs' to add your music folders.",
        );
        return Ok(Vec::new());
    }

    log::info!("Starting library scan. Force reindex: {}", force);

    let audio_exts = to_set(&config.audio_exts);
    let video_exts = to_set(&config.video_exts);
    let playlist_exts = to_set(&config.playlist_exts);

    let (old_cache, recovery_map) = if !force {
        if let Ok((old_tracks, _)) = load_index() {
            log::info!("Cache loaded. Found {} existing entries", old_tracks.len());

            let mut path_map = HashMap::new();
            let mut attr_map = HashMap::new();

            for t in old_tracks {
                path_map.insert(t.path.clone(), t.clone());

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

    let scan_hidden = config.scan_hidden_dirs;
    let tracks: Vec<Track> = config
        .music_dirs
        .iter()
        .flat_map(|dir| {
            log::info!("Walking directory: {:?}", dir);
            WalkDir::new(dir)
                .into_iter()
                .filter_entry(move |e| {
                    if scan_hidden {
                        return true;
                    }
                    let is_hidden = e
                        .file_name()
                        .to_str()
                        .map(|s| s.starts_with('.') && s != "." && s != "..")
                        .unwrap_or(false);
                    !is_hidden
                })
                .filter_map(|e| e.ok())
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
                MediaType::Audio
            } else if playlist_exts.contains(&ext) {
                MediaType::Playlist
            } else if config.video_ok && video_exts.contains(&ext) {
                MediaType::Video
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

            if media_type == MediaType::Playlist {
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

                let parse_opts = ParseOptions::new()
                    .read_properties(false)
                    .read_cover_art(false);
                let probe_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    Probe::open(path)
                        .map(|p| p.options(parse_opts))
                        .and_then(|p| p.read())
                }));

                match probe_res {
                    Ok(Ok(tagged_file)) => {
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
                    Ok(Err(e)) => {
                        log::warn!("Metadata probe failed for '{}': {}", path_str, e);
                    }
                    Err(_) => {
                        log::warn!("Metadata probe panicked on audio file '{}'", path_str);
                    }
                }
            }

            if title.is_empty() {
                let filename = path.file_stem()?.to_string_lossy().to_string();

                let (parsed_title, parsed_artist, parsed_album, parsed_genre) =
                    parse_filename_with_pattern(&filename, &config.filename_pattern);

                title = parsed_title;
                if artist.is_empty() && !parsed_artist.is_empty() {
                    artist = parsed_artist;
                }
                if album.is_empty() && !parsed_album.is_empty() {
                    album = parsed_album;
                }
                if genre.is_empty() && !parsed_genre.is_empty() {
                    genre = parsed_genre;
                }
            }

            if artist.is_empty() || album.is_empty() {
                let (dir_artist, dir_album) = infer_parent_dirs_metadata(path, &config.music_dirs);
                if artist.is_empty()
                    && let Some(a) = dir_artist
                {
                    artist = a;
                }
                if album.is_empty()
                    && let Some(alb) = dir_album
                {
                    album = alb;
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
                media_type,
            })
        })
        .collect();

    pb.finish_with_message(format!("Indexed {} tracks", tracks.len()));
    log::info!(
        "Indexing session finished. Total valid tracks found: {}",
        tracks.len()
    );

    if !config.music_dirs.is_empty() {
        ui::newline();
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
    fn test_parse_no_artist() {
        let (artist, title) = parse_filename_metadata("JustASong.flac");
        assert_eq!(artist, "");
        assert_eq!(title, "JustASong.flac");
    }

    #[test]
    fn test_parse_multiple_dashes() {
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
        assert_eq!(track.media_type, MediaType::Audio);
    }

    #[test]
    fn test_parse_filename_brackets_and_tags() {
        let (artist, title) =
            parse_filename_metadata("[1080p] Radiohead - Creep (Acoustic Version)");
        assert_eq!(artist, "[1080p] Radiohead");
        assert_eq!(title, "Creep (Acoustic Version)");
    }

    #[test]
    fn test_parse_filename_emojis_and_cjk() {
        let (artist, title) = parse_filename_metadata("米津玄師 - Lemon 🍋");
        assert_eq!(artist, "米津玄師");
        assert_eq!(title, "Lemon 🍋");
    }

    #[test]
    fn test_strip_leading_track_numbers() {
        assert_eq!(strip_leading_track_number("01 - Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("01. Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("01.Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("01._Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("01_Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("01-Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("01 Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("1-02 - Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("2-10. Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("A01 - Usseewa"), "Usseewa");
        assert_eq!(strip_leading_track_number("B1. Usseewa"), "Usseewa");
        assert_eq!(
            strip_leading_track_number("01 - Ado - Usseewa"),
            "Ado - Usseewa"
        );
        assert_eq!(
            strip_leading_track_number("02. Ado - Readymade"),
            "Ado - Readymade"
        );
        assert_eq!(
            strip_leading_track_number("100 gecs - Money Machine"),
            "100 gecs - Money Machine"
        );
        assert_eq!(
            strip_leading_track_number("2Pac - Changes"),
            "2Pac - Changes"
        );
        assert_eq!(
            strip_leading_track_number("1975 - Somebody Else"),
            "1975 - Somebody Else"
        );
        assert_eq!(
            strip_leading_track_number("50 Cent - In Da Club"),
            "50 Cent - In Da Club"
        );
        assert_eq!(
            strip_leading_track_number("3OH!3 - DONTTRUSTME"),
            "3OH!3 - DONTTRUSTME"
        );
    }

    #[test]
    fn test_parse_filename_metadata_with_track_numbers() {
        let (artist, title) = parse_filename_metadata("01 - Ado - Usseewa");
        assert_eq!(artist, "Ado");
        assert_eq!(title, "Usseewa");

        let (artist, title) = parse_filename_metadata("01 - Usseewa");
        assert_eq!(artist, "");
        assert_eq!(title, "Usseewa");

        let (artist, title) = parse_filename_metadata("01. Usseewa");
        assert_eq!(artist, "");
        assert_eq!(title, "Usseewa");

        let (artist, title) = parse_filename_metadata("01 Usseewa");
        assert_eq!(artist, "");
        assert_eq!(title, "Usseewa");

        let (artist, title) = parse_filename_metadata("02. Ado - Readymade");
        assert_eq!(artist, "Ado");
        assert_eq!(title, "Readymade");

        let (artist, title) = parse_filename_metadata("01_Ado - Usseewa");
        assert_eq!(artist, "Ado");
        assert_eq!(title, "Usseewa");

        let (artist, title) = parse_filename_metadata("100 gecs - Money Machine");
        assert_eq!(artist, "100 gecs");
        assert_eq!(title, "Money Machine");

        let (artist, title) = parse_filename_metadata("50 Cent - In Da Club");
        assert_eq!(artist, "50 Cent");
        assert_eq!(title, "In Da Club");

        let (artist, title) = parse_filename_metadata("01 - 02 - Usseewa");
        assert_eq!(artist, "");
        assert_eq!(title, "Usseewa");
    }

    #[test]
    fn test_infer_parent_dirs_metadata() {
        let music_dir = std::path::PathBuf::from("/home/user/Music");
        let dirs = vec![music_dir.clone()];

        // Structure: Music/Artist/Album/Track.mp3
        let path1 = music_dir
            .join("Ado")
            .join("Kyougen")
            .join("01 - Usseewa.mp3");
        let (artist, album) = infer_parent_dirs_metadata(&path1, &dirs);
        assert_eq!(artist.as_deref(), Some("Ado"));
        assert_eq!(album.as_deref(), Some("Kyougen"));

        // Structure: Music/Artist/Track.mp3
        let path2 = music_dir.join("Ado").join("01 - Usseewa.mp3");
        let (artist, album) = infer_parent_dirs_metadata(&path2, &dirs);
        assert_eq!(artist.as_deref(), Some("Ado"));
        assert_eq!(album, None);

        // Structure: Music/Track.mp3 (directly inside configured music dir)
        let path3 = music_dir.join("01 - Usseewa.mp3");
        let (artist, album) = infer_parent_dirs_metadata(&path3, &dirs);
        assert_eq!(artist, None);
        assert_eq!(album, None);

        // Trailing slash handling in configured music_dirs
        let dirs_with_slash = vec![std::path::PathBuf::from("/home/user/Music/")];
        let (artist, album) = infer_parent_dirs_metadata(&path1, &dirs_with_slash);
        assert_eq!(artist.as_deref(), Some("Ado"));
        assert_eq!(album.as_deref(), Some("Kyougen"));
    }

    #[test]
    fn test_parse_custom_pattern_standard() {
        let meta =
            parse_custom_pattern("01 - Ado - Usseewa", "{track} - {artist} - {title}").unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa");

        let meta = parse_custom_pattern("Ado - Kyougen - Usseewa", "{artist} - {album} - {title}")
            .unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.album, "Kyougen");
        assert_eq!(meta.title, "Usseewa");

        let meta = parse_custom_pattern("01. Usseewa", "{track}. {title}").unwrap();
        assert_eq!(meta.artist, "");
        assert_eq!(meta.title, "Usseewa");

        let meta = parse_custom_pattern("Usseewa - Live - Ado", "{title} - {artist}").unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa - Live");

        let meta =
            parse_custom_pattern("[01] Ado - Usseewa", "[{track}] {artist} - {title}").unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa");

        let meta = parse_custom_pattern(
            "01 - Ado - Usseewa (J-Pop)",
            "{num} - {artist} - {title} ({genre})",
        )
        .unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa");
        assert_eq!(meta.genre, "J-Pop");
    }

    #[test]
    fn test_parse_filename_with_pattern_fallbacks() {
        // Default pattern uses smart heuristics
        let (title, artist, album, genre) =
            parse_filename_with_pattern("01 - Ado - Usseewa", "default");
        assert_eq!(artist, "Ado");
        assert_eq!(title, "Usseewa");
        assert_eq!(album, "");
        assert_eq!(genre, "");

        // Empty pattern uses smart heuristics
        let (title, artist, _, _) = parse_filename_with_pattern("01. Readymade", "");
        assert_eq!(artist, "");
        assert_eq!(title, "Readymade");

        // Custom pattern matches successfully
        let (title, artist, _, _) =
            parse_filename_with_pattern("01 - Ado - Usseewa", "{track} - {artist} - {title}");
        assert_eq!(artist, "Ado");
        assert_eq!(title, "Usseewa");

        // Custom pattern mismatch gracefully falls back to default heuristics
        let (title, artist, _, _) =
            parse_filename_with_pattern("Usseewa", "{track} - {artist} - {title}");
        assert_eq!(artist, "");
        assert_eq!(title, "Usseewa");

        let (title, artist, _, _) =
            parse_filename_with_pattern("01 - Usseewa", "{track} - {artist} - {title}");
        assert_eq!(artist, "");
        assert_eq!(title, "Usseewa");

        let (title, artist, _, _) =
            parse_filename_with_pattern("100 gecs - Money Machine", "{track} - {artist} - {title}");
        assert_eq!(artist, "100 gecs");
        assert_eq!(title, "Money Machine");
    }

    #[test]
    fn test_parse_custom_pattern_edge_cases() {
        // Single token
        let meta = parse_custom_pattern("01 - Usseewa", "{title}").unwrap();
        assert_eq!(meta.title, "01 - Usseewa");

        // Underscore delimiters
        let meta = parse_custom_pattern("01_Ado_Usseewa", "{track}_{artist}_{title}").unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa");

        // Case insensitivity of tokens
        let meta =
            parse_custom_pattern("01 - Ado - Usseewa", "{TRACK} - {ARTIST} - {TITLE}").unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa");

        // Unknown tokens ignored cleanly
        let meta = parse_custom_pattern(
            "01 - Ado - Usseewa - 2022",
            "{track} - {artist} - {title} - {year}",
        )
        .unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa");

        // Extension tolerance in custom patterns against stem or full filename
        let meta =
            parse_custom_pattern("01 - Ado - Usseewa", "{track} - {artist} - {title}.mp3").unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa");

        let meta =
            parse_custom_pattern("01 - Ado - Usseewa.mp3", "{track} - {artist} - {title}.mp3")
                .unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa");

        // Whitespace tolerance around delimiters
        let meta =
            parse_custom_pattern("01  -  Ado  -  Usseewa", "{track} - {artist} - {title}").unwrap();
        assert_eq!(meta.artist, "Ado");
        assert_eq!(meta.title, "Usseewa");

        // Malformed pattern (unclosed brace) returns None
        assert!(parse_custom_pattern("01 - Ado - Usseewa", "{track - {title}").is_none());

        // Literal pattern with no tokens returns None
        assert!(parse_custom_pattern("01 - Ado - Usseewa", "Plain Text Pattern").is_none());
    }

    #[test]
    fn test_jsonl_corruption_line_recovery() {
        let valid_json = r#"{"path":"/a.mp3","title":"A","artist":"B","album":"C","genre":"D","mtime":1,"size":2,"media_type":"audio"}"#;
        let corrupted_json = r#"{"path":"/broken.mp3","title": [invalid json syntax"#;
        let empty_line = "   ";

        assert!(serde_json::from_str::<Track>(valid_json).is_ok());
        assert!(serde_json::from_str::<Track>(corrupted_json).is_err());
        assert!(serde_json::from_str::<Track>(empty_line).is_err());
    }
}
