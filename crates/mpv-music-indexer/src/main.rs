use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use lofty::prelude::*;
use lofty::probe::Probe;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
use std::io::{self, Write};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

// --- CLI Arguments ---
#[derive(Parser)]
#[command(author, version, about = "Blazing fast music indexer for mpv-music")]
struct Cli {
    /// Directories to index
    #[arg(required = true)]
    directories: Vec<String>,

    /// Include video files in the index
    #[arg(long)]
    video: bool,

    /// Force single-threaded mode
    #[arg(long)]
    serial: bool,

    /// Audio extensions
    #[arg(
        long,
        default_value = "mp3,flac,wav,m4a,aac,ogg,opus,wma,alac,aiff,amr"
    )]
    audio_exts: String,

    /// Video extensions
    #[arg(
        long,
        default_value = "mp4,mkv,webm,avi,mov,flv,wmv,mpeg,mpg,3gp,ts,vob,m4v"
    )]
    video_exts: String,

    /// Playlist extensions
    #[arg(long, default_value = "m3u,m3u8,pls")]
    playlist_exts: String,
}

// --- JSON Output Structure ---
#[derive(Serialize)]
struct Track {
    path: String,
    title: String,
    artist: String,
    album: String,
    genre: String,
    mtime: u64,
    size: u64,
    media_type: String,
}

// Helper to split "mp3, flac" or "mp3 flac" into a Set
fn parse_exts(input: &str) -> HashSet<String> {
    input
        .split(|c| c == ',' || c == ' ')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // This forces Rayon (and .par_bridge) to use exactly 1 thread.
    if cli.serial {
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build_global()
            .unwrap();
    }

    // Parse extensions from CLI args
    let audio_exts = parse_exts(&cli.audio_exts);
    let video_exts = parse_exts(&cli.video_exts);
    let playlist_exts = parse_exts(&cli.playlist_exts);

    // --- SETUP PROGRESS BAR ---
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {pos} tracks ({per_sec})")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    // --------------------------

    for dir in &cli.directories {
        // WalkDir follows symlinks (default is false)
        WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .par_bridge()
            .for_each(|entry| {
                let path = entry.path();

                if !path.is_file() {
                    return;
                }

                // Extension Check (Case-insensitive)
                let ext = match path.extension().and_then(|e| e.to_str()) {
                    Some(e) => e.to_lowercase(),
                    None => return,
                };

                // Determine Media Type
                let media_type = if audio_exts.contains(&ext) {
                    "audio"
                } else if playlist_exts.contains(&ext) {
                    "playlist"
                } else if cli.video && video_exts.contains(&ext) {
                    "video"
                } else {
                    return; // Skip unknown extensions
                };

                // TICK THE SPINNER (Only count valid files)
                // Rayon handles thread-safe incrementing automatically!
                pb.inc(1);

                // OPTIMIZATION: Use `entry.metadata()`
                // walkdir often already has this info cached from reading the directory.
                // This saves 1 System Call per file.
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => return,
                };

                let mtime = metadata
                    .modified()
                    .unwrap_or(SystemTime::UNIX_EPOCH)
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let size = metadata.len();
                let path_str = path.to_string_lossy().to_string();

                // Metadata Extraction Vars
                let (mut title, mut artist, mut album, mut genre);

                if media_type == "playlist" {
                    // Playlists: Title = Filename
                    title = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    artist = "Playlist".to_string();
                    album = "Playlists".to_string();
                    genre = "Playlist".to_string();
                } else {
                    // Initialize empty
                    title = String::new();
                    artist = String::new();
                    album = String::new();
                    genre = String::new();

                    // Try to read tags using Lofty
                    if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
                        // Try primary tag (ID3v2, Vorbis) then fallback to any available tag
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
                }

                // --- Fallback Logic ---
                if title.is_empty() {
                    title = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
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

                // Construct Object
                let track = Track {
                    path: path_str,
                    title,
                    artist,
                    album,
                    genre,
                    mtime,
                    size,
                    media_type: media_type.to_string(),
                };

                // OPTIMIZATION: Reduce Lock Contention (The "Buffer & Dump" Strategy)
                // We serialize to memory first (to_string) to avoid holding the stdout lock
                // while serde does its tiny writes. This prevents threads from fighting.
                if let Ok(json) = serde_json::to_string(&track) {
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    writeln!(handle, "{}", json).ok();
                }
            });
    }

    // Finish spinner
    pb.finish_with_message("Done");

    Ok(())
}
