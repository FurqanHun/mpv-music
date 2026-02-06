mod config;
mod dep_check;
mod indexer;
mod player;
mod search;

use anyhow::{Context, Result};
use clap::Parser;
use directories::ProjectDirs;
use flexi_logger::{FileSpec, Logger, WriteMode, style};
use skim::prelude::*;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Clone, Debug)]
#[command(
    name = "mpv-music",
    author,
    version,
    about = "mpv-music - A TUI-based music player wrapper for MPV",
    rename_all = "kebab-case"
)]
struct Cli {
    #[arg(index = 1, help = "Directly play a file, directory, or URL")]
    target: Option<String>,

    // indexing
    #[arg(
        short = 'r',
        long,
        help = "Update index (incremental scan). Detects new/changed files."
    )]
    refresh_index: bool,

    #[arg(long, help = "Force a full re-scan of the library.")]
    reindex: bool,

    // actions
    #[arg(short = 'u', long, help = "Update the application")]
    update: bool,

    #[arg(
        long,
        num_args = 1..,
        value_name = "PATH",
        help = "Add directory (e.g. --add-dir /music /other)"
    )]
    add_dir: Option<Vec<String>>,

    #[arg(
        long,
        num_args = 1..,
        value_name = "PATH",
        visible_alias = "rm-dir",
        help = "Remove directory"
    )]
    remove_dir: Option<Vec<String>>,

    #[arg(long, help = "Open the Interactive Directory Manager")]
    manage_dirs: bool,

    // conf/log
    #[arg(
        short = 'c',
        long,
        value_name = "EDITOR",
        num_args = 0..=1,
        help = "Edit config file"
    )]
    config: Option<Option<String>>,

    #[arg(long, visible_alias = "rm-conf", help = "Delete config file (Reset)")]
    remove_config: bool,

    #[arg(
        long,
        value_name = "PAGER",
        num_args = 0..=1,
        help = "View logs"
    )]
    log: Option<Option<String>>,

    #[arg(long, visible_alias = "rm-log", help = "Delete log file")]
    remove_log: bool,

    // playback
    #[arg(short = 'p', long, help = "Play all tracks immediately")]
    play_all: bool,

    #[arg(
            short = 'l',
            long,
            num_args = 0..=1,
            help = "Open Playlist Mode. Opens picker if no value given."
        )]
    playlist: Option<Option<String>>,

    #[arg(long, help = "Allow video files")]
    video_ok: bool,

    #[arg(
            long = "loop",
            num_args = 0..=1,
            default_missing_value = "inf",
            help = "Enable looping ('inf', 'no', 'track', or a NUMBER)"
        )]
    loop_arg: Option<String>,

    #[arg(long, help = "Disable all looping")]
    no_loop: bool,

    #[arg(long, help = "Loop the current track (Repeat One)")]
    repeat: bool,

    #[arg(
        short = 'e',
        long,
        value_name = "EXT1,EXT2",
        help = "Override allowed extensions"
    )]
    ext: Option<String>,

    // filters (comma supported)
    #[arg(
        short = 'g',
        long,
        num_args = 0..=1,
        help = "Filter by Genre (e.g. -g 'Pop,Rock')"
    )]
    genre: Option<Option<String>>,

    #[arg(
        short = 'a',
        long,
        num_args = 0..=1,
        help = "Filter by Artist (e.g. -a 'ado,gentle')"
    )]
    artist: Option<Option<String>>,

    #[arg(
        short = 'b',
        long,
        num_args = 0..=1,
        help = "Filter by Album"
    )]
    album: Option<Option<String>>,

    #[arg(
            short = 't',
            long,
            num_args = 0..=1,
            help = "Filter by Title (Partial). Opens Track Mode if no value given."
        )]
    title: Option<Option<String>>,

    // sys
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Display Verbose Information")]
    verbose: u8,
    #[arg(short = 'd', long, help = "Debug mode")]
    debug: bool,
    #[arg(long, help = "Set volume (0-100)")]
    volume: Option<u8>,
    #[arg(short = 's', long, help = "Shuffle")]
    shuffle: bool,
    #[arg(long, help = "No Shuffle")]
    no_shuffle: bool,
    #[arg(long, help = "Force serial (single-threaded) processing")]
    serial: bool,
    #[arg(
            long,
            visible_alias = "yt",
            num_args = 0..=1,
            help = "Search YouTube directly (e.g. --yt 'lofi') Requires yt-dlp."
        )]
    search: Option<Option<String>>,
}

// skim item wrappers

struct TrackItem {
    track: indexer::Track,
    display_text: String,
}

impl SkimItem for TrackItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display_text)
    }
    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.track.path)
    }
    fn preview(&self, _ctx: PreviewContext) -> ItemPreview {
        let ext = std::path::Path::new(&self.track.path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("???")
            .to_uppercase();

        let type_str = if self.track.media_type == "video" {
            "Video"
        } else {
            "Audio"
        };
        let icon = if self.track.media_type == "video" {
            "🎬"
        } else {
            "🎵"
        };

        let text = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mArtist:\x1b[0m {}\n  \x1b[1;32mAlbum:\x1b[0m  {}\n  \x1b[1;35mGenre:\x1b[0m  {}\n  \x1b[1;34mType:\x1b[0m   {} ({})\n\n  \x1b[90mPath: {}\x1b[0m",
            icon,
            self.track.title,
            self.track.artist,
            self.track.album,
            self.track.genre,
            type_str,
            ext,
            self.track.path
        );
        ItemPreview::AnsiText(text)
    }
}

struct TagItem {
    name: String,
    count: usize,
    samples: Vec<String>,
    icon: String,
}

impl SkimItem for TagItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} ({})", self.name, self.count))
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let mut sample_text = String::new();
        for (i, song) in self.samples.iter().enumerate() {
            if i >= 10 {
                break;
            } // limit to 10
            sample_text.push_str(&format!("  {}. {}\n", i + 1, song));
        }

        let output = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mTotal Tracks:\x1b[0m {}\n\n  \x1b[1;32mSample Tracks:\x1b[0m\n{}",
            self.icon, self.name, self.count, sample_text
        );
        ItemPreview::AnsiText(output)
    }
}

struct DirItem {
    dirname: String,
    path: String,
    count: usize,
    samples: Vec<String>,
}

impl SkimItem for DirItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} ({})", self.dirname, self.count))
    }
    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.path)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let mut sample_text = String::new();
        for (i, song) in self.samples.iter().enumerate() {
            if i >= 10 {
                break;
            }
            sample_text.push_str(&format!("  {}. {}\n", i + 1, song));
        }

        let output = format!(
            "\n  📁 \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mPath:\x1b[0m {}\n  \x1b[1;33mFiles:\x1b[0m {}\n\n  \x1b[1;32mContents:\x1b[0m\n{}",
            self.dirname, self.path, self.count, sample_text
        );
        ItemPreview::AnsiText(output)
    }
}

struct PlaylistItem {
    name: String,
    path: String,
    count: usize,
    preview_lines: Vec<String>,
}

impl SkimItem for PlaylistItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.path)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let mut content = String::new();
        for (i, line) in self.preview_lines.iter().enumerate() {
            if i >= 10 {
                break;
            }
            content.push_str(&format!("  {}. {}\n", i + 1, line));
        }

        if content.is_empty() {
            content.push_str("  (Empty or Binary Playlist)\n");
        }

        let output = format!(
            "\n  📜 \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mPath:\x1b[0m {}\n  \x1b[1;33mEntries:\x1b[0m {}\n\n  \x1b[1;32mFirst Few Tracks:\x1b[0m\n{}",
            self.name, self.path, self.count, content
        );
        ItemPreview::AnsiText(output)
    }
}

struct SearchItem {
    result: search::SearchResult,
}

impl SkimItem for SearchItem {
    fn text(&self) -> Cow<'_, str> {
        // list, just lil bit
        Cow::Borrowed(&self.result.title)
    }
    fn output(&self) -> Cow<'_, str> {
        // url for the player
        Cow::Borrowed(&self.result.url)
    }

    fn preview(&self, _ctx: PreviewContext) -> ItemPreview {
        let (icon, type_str) = if self.result.is_playlist {
            ("📜", "Playlist / Mix")
        } else {
            ("📺", "Video")
        };

        let details = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mChannel:\x1b[0m  {}\n  \x1b[1;33mViews:\x1b[0m    {}\n  \x1b[1;33mDuration:\x1b[0m {}\n  \x1b[1;34mType:\x1b[0m     {}\n\n  \x1b[90mURL: {}\x1b[0m",
            icon,
            self.result.title,
            self.result.uploader,
            self.result.view_count,
            self.result.duration,
            type_str,
            self.result.url
        );
        ItemPreview::AnsiText(details)
    }
}

struct MenuItem {
    text: String,
    id: String,
}
impl SkimItem for MenuItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.text)
    }
    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.id)
    }
    fn preview(&self, _ctx: PreviewContext) -> ItemPreview {
        ItemPreview::Text(self.id.clone())
    }
}

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
    } else {
        if args.debug {
            "mpv_music=debug, warn"
        } else if args.verbose > 0 {
            "mpv_music=info, warn"
        } else {
            "mpv_music=error, warn"
        }
    };

    std::fs::create_dir_all(&log_dir)?;
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
                    .directory(&log_dir)
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
    if args.update {
        println!("Update logic not implemented yet.");
        println!("GitHub releases: https://github.com/FurqanHun/mpv-music/releases");
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
            if add_directory(&mut cfg, dir.clone())? {
                config_changed = true;
            }
        }
    }
    if let Some(dirs) = &args.remove_dir {
        for dir in dirs {
            if remove_directory(&mut cfg, dir.clone())? {
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
        if run_manage_dirs_mode(&mut cfg)? {
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
        run_tag_mode(&tracks, &cfg, Some("genre"))?;
        return Ok(());
    }
    if let Some(None) = args.artist {
        log::info!("Empty artist flag. Opening Artist Picker.");
        run_tag_mode(&tracks, &cfg, Some("artist"))?;
        return Ok(());
    }
    if let Some(None) = args.album {
        log::info!("Empty album flag. Opening Album Picker.");
        run_tag_mode(&tracks, &cfg, Some("album"))?;
        return Ok(());
    }
    if let Some(None) = args.title {
        log::info!("Empty title flag. Opening Track Mode.");
        run_track_mode(&tracks, &cfg)?;
        return Ok(());
    }
    if let Some(None) = args.search {
        log::info!("Empty search flag. Opening YouTube Search.");
        run_search_mode(&cfg, None)?;
        return Ok(());
    }
    if let Some(None) = args.playlist {
        log::info!("Empty playlist flag. Opening Playlist Picker.");
        run_playlist_mode(&tracks, &cfg)?;
        return Ok(());
    }

    // main search and filter logic
    if args.genre.is_some() || args.artist.is_some() || args.album.is_some() || args.title.is_some()
    {
        let is_multi_value_search = args
            .artist
            .as_ref()
            .and_then(|o| o.as_ref())
            .map_or(false, |s| s.contains(','))
            || args
                .genre
                .as_ref()
                .and_then(|o| o.as_ref())
                .map_or(false, |s| s.contains(','))
            || args
                .album
                .as_ref()
                .and_then(|o| o.as_ref())
                .map_or(false, |s| s.contains(','));

        // stage 1: exact match
        let mut filtered = if !is_multi_value_search {
            apply_cli_filters(&tracks, &args, true)
        } else {
            Vec::new()
        };

        // stage 2: partial match / ambiguity handling
        if filtered.is_empty() {
            log::debug!("Exact match skipped or failed, trying partial...");
            let partials = apply_cli_filters(&tracks, &args, false);

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
                    _ => {}
                }
            }

            // if we have ambiguity AND it wasn't a multi-search, ask the user
            // if it WAS a multi-search, we assume they want all of them
            if unique_options.len() > 1 && !active_key.is_empty() && !is_multi_value_search {
                let mut options_vec: Vec<&str> =
                    unique_options.iter().map(|s| s.as_str()).collect();
                options_vec.sort();

                if let Some(clarified) =
                    run_skim_simple(options_vec, &format!("Which {} did you mean? ", active_key))
                {
                    let mut temp_args = args.clone();
                    match active_key {
                        "artist" => temp_args.artist = Some(Some(clarified)),
                        "genre" => temp_args.genre = Some(Some(clarified)),
                        "album" => temp_args.album = Some(Some(clarified)),
                        _ => {}
                    }
                    filtered = apply_cli_filters(&tracks, &temp_args, true);
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
        if args.play_all {
            let paths: Vec<String> = filtered.iter().map(|t| t.path.clone()).collect();
            player::play_files(&paths, &cfg)?;
        } else {
            run_post_filter_action(&filtered, &cfg)?;
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
        run_playlist_mode(&tracks, &cfg)?;
    } else {
        run_main_menu(&mut tracks, &mut cfg)?;
    }

    Ok(())
}

fn run_main_menu(tracks: &mut Vec<indexer::Track>, cfg: &mut config::Config) -> Result<()> {
    loop {
        let options = vec![
            "1) Directory Mode",
            "2) Track Mode",
            "3) Playlist Mode",
            "4) Tag Filter Mode",
            "5) Play All Mode",
            "6) Search & Stream URL",
            "7) Settings",
            "q) Quit",
        ];
        let selected = run_skim_simple(options, "🎧 Pick mode > ");
        match selected.as_deref() {
            Some(s) if s.starts_with("1)") => run_dir_mode(tracks, cfg)?,
            Some(s) if s.starts_with("2)") => run_track_mode(tracks, cfg)?,
            Some(s) if s.starts_with("3)") => run_playlist_mode(tracks, cfg)?,
            Some(s) if s.starts_with("4)") => run_tag_mode(tracks, cfg, None)?,
            Some(s) if s.starts_with("5)") => {
                let paths: Vec<String> = tracks.iter().map(|t| t.path.clone()).collect();
                player::play_files(&paths, cfg)?;
            }
            Some(s) if s.starts_with("6)") => {
                run_search_mode(cfg, None)?;
            }
            Some(s) if s.starts_with("7)") => run_settings_menu(tracks, cfg)?,
            Some(s) if s.starts_with("q)") => break,
            None => break,
            _ => {}
        }
    }
    Ok(())
}

fn run_tag_mode(
    tracks: &[indexer::Track],
    cfg: &config::Config,
    force_key: Option<&str>,
) -> Result<()> {
    // if a key is forced (like from cli -g), we don't loop/menu, just run once
    if let Some(k) = force_key {
        let _ = run_tag_picker(tracks, cfg, k)?;
        return Ok(());
    }

    loop {
        let choices = vec!["1) Genre", "2) Artist", "3) Album", "q) Back"];
        let choice = run_skim_simple(choices, "🔎 Filter by > ");

        let key = match choice.as_deref() {
            Some(s) if s.contains("Genre") => "genre",
            Some(s) if s.contains("Artist") => "artist",
            Some(s) if s.contains("Album") => "album",
            Some(s) if s.contains("Back") || s.starts_with("q)") => return Ok(()),
            None => return Ok(()),
            _ => continue,
        };

        // true = selection was made and processed -> Exit to Main Menu.
        // false = user pressed ESC inside the list -> Loop back.
        if run_tag_picker(tracks, cfg, key)? {
            return Ok(());
        }
    }
}

// helper to keep the logic clean, returns true if action taken, false if aborted (ESC).
fn run_tag_picker(tracks: &[indexer::Track], cfg: &config::Config, key: &str) -> Result<bool> {
    let (icon, prompt) = match key {
        "genre" => ("🏷️", "🏷️  Pick Genre > "),
        "artist" => ("🎤", "🎤 Pick Artist > "),
        "album" => ("💿", "💿 Pick Album > "),
        _ => return Ok(false),
    };

    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut samples: HashMap<String, Vec<String>> = HashMap::new();

    for t in tracks {
        let val = match key {
            "genre" => &t.genre,
            "artist" => &t.artist,
            "album" => &t.album,
            _ => continue,
        };

        let clean_key = if val.trim().is_empty() {
            "UNKNOWN"
        } else {
            val.as_str()
        };

        *counts.entry(clean_key.to_string()).or_default() += 1;

        let sample_list = samples.entry(clean_key.to_string()).or_default();
        if sample_list.len() < 10 {
            sample_list.push(t.title.clone());
        }
    }

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    let mut sorted_keys: Vec<_> = counts.keys().collect();
    sorted_keys.sort();

    for k in sorted_keys {
        let count = *counts.get(k).unwrap();
        let sample_list = samples.get(k).unwrap().clone();

        tx.send(vec![Arc::new(TagItem {
            name: k.clone(),
            count,
            samples: sample_list,
            icon: icon.to_string(),
        })])
        .unwrap();
    }
    drop(tx);

    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt(prompt.to_string())
        .preview(Some("".to_string()))
        .reverse(true)
        .inline_info(true)
        .build()
        .unwrap();

    let output = Skim::run_with(opts, Some(rx)).ok().context("Skim failed")?;

    if output.is_abort {
        return Ok(false);
    }

    let selected_items = output.selected_items;
    if selected_items.is_empty() {
        return Ok(false);
    }

    // TagItem.text() returns "Name (Count)" and we need just "Name".
    let mut selected_names = HashSet::new();
    for item in selected_items {
        let text = item.text();
        let name = text.rsplit_once(" (").map(|(n, _)| n).unwrap_or(&text);
        selected_names.insert(name.to_string());
    }

    let filtered: Vec<indexer::Track> = tracks
        .iter()
        .filter(|t| {
            let val = match key {
                "genre" => &t.genre,
                "artist" => &t.artist,
                "album" => &t.album,
                _ => "",
            };
            let clean_val = if val.trim().is_empty() {
                "UNKNOWN"
            } else {
                &val[..]
            };
            selected_names.contains(clean_val)
        })
        .cloned()
        .collect();

    run_post_filter_action(&filtered, cfg)?;

    Ok(true)
}

fn run_post_filter_action(tracks: &[indexer::Track], cfg: &config::Config) -> Result<()> {
    if tracks.is_empty() {
        return Ok(());
    }
    let paths: Vec<String> = tracks.iter().map(|t| t.path.clone()).collect();

    let opts = vec![
        format!("1) Play all {} tracks", tracks.len()),
        "2) Select individual tracks".to_string(),
    ];
    let pick = run_skim_simple(opts.iter().map(|s| s.as_str()).collect(), "What's next? ");
    match pick.as_deref() {
        Some(s) if s.starts_with("1)") => player::play_files(&paths, cfg),
        Some(s) if s.starts_with("2)") => run_track_mode(tracks, cfg),
        _ => Ok(()),
    }
}

fn run_manage_dirs_mode(cfg: &mut config::Config) -> Result<bool> {
    let mut any_changes = false;

    loop {
        let count = cfg.music_dirs.len();
        let prompt = format!("📂 Manage ({} dirs) >   ", count);

        let options = vec!["1) Add Directory", "2) Remove Directory", "q) Back"];

        let sel = run_skim_simple(options, &prompt);
        match sel.as_deref() {
            Some(s) if s.starts_with("1)") => {
                // true = mark state as dirty
                if manage_add_loop(cfg)? {
                    any_changes = true;
                }
            }
            Some(s) if s.starts_with("2)") => {
                // true = mark state as dirty
                if manage_remove_menu(cfg)? {
                    any_changes = true;
                }
            }
            Some(s) if s.starts_with("q)") => break,
            None => break,
            _ => {}
        }
    }
    // true (only if user actually touched the config)
    Ok(any_changes)
}

fn manage_add_loop(cfg: &mut config::Config) -> Result<bool> {
    println!("\n📂 --- Add Directory Mode ---");
    println!("Type a full path and press ENTER.");
    println!("Press ENTER (empty) to go back.\n");

    let mut changed = false;

    loop {
        print!("(Add) Path > ");
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let path_str = input.trim().to_string();

        if path_str.is_empty() {
            break;
        }

        // true if added a new path
        if add_directory(cfg, path_str)? {
            changed = true;
        } else {
            // failed (typo/duplicate), sleep briefly for UX
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    }
    Ok(changed)
}

fn manage_remove_menu(cfg: &mut config::Config) -> Result<bool> {
    if cfg.music_dirs.is_empty() {
        println!("No directories to remove.");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return Ok(false);
    }

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for dir in &cfg.music_dirs {
        let dir_str = dir.to_string_lossy().to_string();
        tx.send(vec![Arc::new(MenuItem {
            text: dir_str.clone(),
            id: dir_str,
        })])
        .unwrap();
    }
    drop(tx);

    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt("🗑️  Remove >   ".to_string())
        .header(Some(
            "   Select directories to remove (TAB to select)".to_string(),
        ))
        .reverse(true)
        .inline_info(true)
        .build()
        .unwrap();

    let output = Skim::run_with(opts, Some(rx)).ok().context("Skim failed")?;

    if output.is_abort {
        return Ok(false);
    }

    let selected_items = output.selected_items;
    if selected_items.is_empty() {
        return Ok(false);
    }

    let mut changed = false;
    println!("\nProcessing removals...");
    for item in selected_items {
        let path_str = item.output().to_string();
        if remove_directory(cfg, path_str)? {
            changed = true;
        }
    }

    if changed {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(changed)
}

fn add_directory(cfg: &mut config::Config, dir: String) -> Result<bool> {
    let path_buf = PathBuf::from(&dir);

    if !path_buf.exists() {
        println!("Path does not exist: \"{}\"", dir);
        return Ok(false);
    }

    if !path_buf.is_dir() {
        println!("Path is not a directory: \"{}\"", dir);
        return Ok(false);
    }

    if let Err(e) = std::fs::read_dir(&path_buf) {
        println!("Permission denied: Cannot access \"{}\"", dir);
        log::warn!("Access check failed for {:?}: {}", path_buf, e);
        return Ok(false);
    }

    let path = match std::fs::canonicalize(&path_buf) {
        Ok(p) => p,
        Err(e) => {
            println!("Failed to resolve absolute path: {}", e);
            return Ok(false);
        }
    };

    if !cfg.music_dirs.contains(&path) {
        cfg.music_dirs.push(path.clone());
        println!("Added: {:?}", path);
        Ok(true)
    } else {
        println!("Already exists: {:?}", path);
        Ok(false)
    }
}

fn remove_directory(cfg: &mut config::Config, dir: String) -> Result<bool> {
    let path = std::fs::canonicalize(&dir).unwrap_or_else(|_| PathBuf::from(&dir));
    let start_len = cfg.music_dirs.len();

    cfg.music_dirs.retain(|d| d != &path);

    if cfg.music_dirs.len() < start_len {
        println!("Removed: {:?}", path);
        Ok(true)
    } else {
        println!("Not found in config: {:?}", path);
        Ok(false)
    }
}

fn apply_cli_filters(tracks: &[indexer::Track], args: &Cli, exact: bool) -> Vec<indexer::Track> {
    // prepare search terms ONCE before iterating
    let prepare_terms = |arg: &Option<Option<String>>| -> Option<Vec<String>> {
        arg.as_ref().and_then(|opt| opt.as_ref()).map(|val| {
            val.to_lowercase()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
    };

    let genre_terms = prepare_terms(&args.genre);
    let artist_terms = prepare_terms(&args.artist);
    let album_terms = prepare_terms(&args.album);
    let title_term = args
        .title
        .as_ref()
        .and_then(|t| t.as_ref())
        .map(|s| s.to_lowercase());

    tracks
        .iter()
        .filter(|t| {
            let matches = |field: &str, terms: &Option<Vec<String>>| {
                if let Some(search_vals) = terms {
                    let field_lower = field.to_lowercase();

                    if exact {
                        // check if ANY search term matches ANY track tag exactly
                        // iterators to avoid allocating a new Vec for every track
                        search_vals.iter().any(|term| {
                            field_lower
                                .split(|c| c == ';' || c == ',')
                                .map(|s| s.trim())
                                .any(|tag| tag == term)
                        })
                    } else {
                        // partial match
                        search_vals.iter().any(|term| field_lower.contains(term))
                    }
                } else {
                    true
                }
            };

            matches(&t.genre, &genre_terms)
                && matches(&t.artist, &artist_terms)
                && matches(&t.album, &album_terms)
                && title_term
                    .as_ref()
                    .map_or(true, |ti| t.title.to_lowercase().contains(ti))
        })
        .cloned()
        .collect()
}

fn run_settings_menu(tracks: &mut Vec<indexer::Track>, cfg: &mut config::Config) -> Result<()> {
    loop {
        let options = vec![
            "1) Manage Directories",
            "2) Edit Config File",
            "3) Delete Config File (Reset)",
            "4) View Log File",
            "5) Delete Log File",
            "6) Refresh Index (Fast)",
            "7) Rebuild Index (Full)",
            "q) Back",
        ];

        let selection = run_skim_simple(options, "⚙️ Settings > ");
        match selection.as_deref() {
            // dirs
            Some(s) if s.contains("Manage Directories") => {
                if run_manage_dirs_mode(cfg)? {
                    println!("Syncing changes...");
                    *tracks = indexer::scan(cfg, false)?;
                    indexer::save(tracks)?;
                }
            }

            // conf management
            Some(s) if s.contains("Edit Config") => {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
                let config_path = ProjectDirs::from("com", "furqanhun", "mpv-music")
                    .unwrap()
                    .config_dir()
                    .join("config.toml");

                std::process::Command::new(editor)
                    .arg(&config_path)
                    .status()?;

                // reload to apply changes immediately
                *cfg = config::load(None)?;
                println!("Config reloaded from disk.");
                // pause so user sees the message
                std::thread::sleep(std::time::Duration::from_millis(800));
            }
            Some(s) if s.contains("Delete Config") => {
                let config_path = ProjectDirs::from("com", "furqanhun", "mpv-music")
                    .unwrap()
                    .config_dir()
                    .join("config.toml");
                if config_path.exists() {
                    std::fs::remove_file(&config_path)?;
                    println!("Config deleted. Loading defaults...");
                    // reload = generate the defualt
                    *cfg = config::load(None)?;
                } else {
                    println!("No config file found.");
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            // log management
            Some(s) if s.contains("View Log") => {
                let log_path = ProjectDirs::from("com", "furqanhun", "mpv-music")
                    .unwrap()
                    .data_dir()
                    .join("mpv-music.log");
                let viewer = std::env::var("PAGER").unwrap_or_else(|_| "less".to_string());
                if log_path.exists() {
                    std::process::Command::new(viewer).arg(log_path).status()?;
                } else {
                    println!("Log file does not exist.");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
            Some(s) if s.contains("Delete Log") => {
                let log_path = ProjectDirs::from("com", "furqanhun", "mpv-music")
                    .unwrap()
                    .data_dir()
                    .join("mpv-music.log");
                if log_path.exists() {
                    std::fs::remove_file(log_path)?;
                    println!("Log file nuked.");
                } else {
                    println!("No log file to delete.");
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            // maintain index
            Some(s) if s.contains("Refresh Index") => {
                println!("Refreshing index...");
                *tracks = indexer::scan(cfg, false)?;
                indexer::save(tracks)?;
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            Some(s) if s.contains("Rebuild Index") => {
                println!("Rebuilding index...");
                *tracks = indexer::scan(cfg, true)?;
                indexer::save(tracks)?;
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            Some(s) if s.starts_with("q)") => break,
            None => break,
            _ => {}
        }
    }
    Ok(())
}

// skim impl

fn run_skim_simple(items: Vec<&str>, prompt: &str) -> Option<String> {
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for i in items {
        tx.send(vec![Arc::new(MenuItem {
            text: i.to_string(),
            id: "".to_string(),
        })])
        .unwrap();
    }
    drop(tx);

    let opts = SkimOptionsBuilder::default()
        .height("50%".to_string())
        .reverse(true)
        .prompt(prompt.to_string())
        .inline_info(true)
        .build()
        .unwrap();

    let output = Skim::run_with(opts, Some(rx)).ok()?;
    if output.is_abort {
        return None;
    }

    output.selected_items.first().map(|i| i.text().to_string())
}

fn run_track_mode(tracks: &[indexer::Track], cfg: &config::Config) -> Result<()> {
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for track in tracks {
        if track.media_type == "playlist" {
            continue;
        }
        let display = format!("{} - {}", track.artist, track.title);
        tx.send(vec![Arc::new(TrackItem {
            track: track.clone(),
            display_text: display,
        })])
        .unwrap();
    }
    drop(tx);

    let opts = SkimOptionsBuilder::default()
        .height("100%".to_string())
        .multi(true)
        .preview(Some("".to_string()))
        .prompt("🎵 Tracks > ".to_string())
        .header(Some("   Artist             Title".to_string()))
        .reverse(true)
        .inline_info(true)
        .build()
        .unwrap();

    let output = Skim::run_with(opts, Some(rx)).ok().context("Skim failed")?;
    if output.is_abort {
        return Ok(());
    }

    let paths: Vec<String> = output
        .selected_items
        .iter()
        .map(|i| i.output().to_string())
        .collect();
    if paths.is_empty() {
        return Ok(());
    }

    player::play_files(&paths, cfg)?;
    Ok(())
}

fn run_dir_mode(tracks: &[indexer::Track], cfg: &config::Config) -> Result<()> {
    // Map: DirPath -> Vec<Filename>
    let mut dir_map: HashMap<String, Vec<String>> = HashMap::new();

    for t in tracks {
        let parent = std::path::Path::new(&t.path)
            .parent()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let file_name = std::path::Path::new(&t.path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "???".to_string());

        dir_map.entry(parent).or_default().push(file_name);
    }

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for (path, files) in dir_map {
        let count = files.len();
        let name = std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        tx.send(vec![Arc::new(DirItem {
            dirname: name,
            path: path,
            count: count,
            samples: files,
        })])
        .unwrap();
    }
    drop(tx);

    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt("📁 Folders > ".to_string())
        .header(Some("   Directory Name".to_string()))
        .reverse(true)
        .inline_info(true)
        .preview(Some("".to_string()))
        .build()
        .unwrap();

    let output = Skim::run_with(opts, Some(rx)).ok().context("Skim failed")?;
    if output.is_abort {
        return Ok(());
    }

    let mut files = Vec::new();
    for item in output.selected_items {
        let dir = item.output();
        for t in tracks {
            if t.path.starts_with(dir.as_ref()) {
                files.push(t.path.clone());
            }
        }
    }
    if files.is_empty() {
        return Ok(());
    }
    player::play_files(&files, cfg)
}

fn run_playlist_mode(tracks: &[indexer::Track], cfg: &config::Config) -> Result<()> {
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();

    for t in tracks {
        if t.media_type == "playlist" {
            // handle errors gracefully (eg, if moved) by defaulting to 0
            let (count, lines) = if let Ok(content) = std::fs::read_to_string(&t.path) {
                let all_valid_lines: Vec<String> = content
                    .lines()
                    .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                    .map(|s| s.to_string())
                    .collect();

                let total = all_valid_lines.len();
                let sample = all_valid_lines.into_iter().take(15).collect();

                (total, sample)
            } else {
                (0, vec!["(Could not read file)".to_string()])
            };

            tx.send(vec![Arc::new(PlaylistItem {
                name: t.title.clone(),
                path: t.path.clone(),
                count,
                preview_lines: lines,
            })])
            .unwrap();
        }
    }
    drop(tx);

    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt("📜 Playlists > ".to_string())
        .reverse(true)
        .inline_info(true)
        .preview(Some("".to_string()))
        .build()
        .unwrap();

    let output = Skim::run_with(opts, Some(rx)).ok().context("Skim failed")?;
    if output.is_abort {
        return Ok(());
    }

    if let Some(item) = output.selected_items.first() {
        player::play(&item.output(), cfg)?;
    }
    Ok(())
}

fn run_search_mode(cfg: &config::Config, initial_query: Option<String>) -> Result<()> {
    if !cfg.ytdlp_available {
        eprintln!("\n\x1b[33mFeature Unavailable:\x1b[0m yt-dlp is not installed.");
        eprintln!("Please install 'yt-dlp' to use Search and Streaming.");
        return Ok(());
    }

    let query = if let Some(q) = initial_query {
        q // quey passed via cli
    } else {
        println!("Search YouTube or Paste URL:");
        print!("🔎 > ");
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    if query.is_empty() {
        return Ok(());
    }

    if query.starts_with("http") {
        log::info!("Direct URL detected, playing...");
        player::play(&query, cfg)?;
        return Ok(());
    }

    println!("Fetching results for '{}'...", query);
    let results = search::search_youtube(&query, 25)?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for r in results {
        tx.send(vec![Arc::new(SearchItem { result: r })]).unwrap();
    }
    drop(tx);

    let opts = SkimOptionsBuilder::default()
        .height("100%".to_string())
        .multi(true)
        .prompt("🎯 Search > ".to_string())
        .reverse(true)
        .inline_info(true)
        .preview(Some("".to_string()))
        .build()
        .unwrap();

    if let Some(output) = Skim::run_with(opts, Some(rx)).ok() {
        if output.is_abort {
            return Ok(());
        }

        let selected_urls: Vec<String> = output
            .selected_items
            .iter()
            .map(|item| item.output().to_string())
            .collect();

        if !selected_urls.is_empty() {
            if selected_urls.len() == 1 {
                player::play(&selected_urls[0], cfg)?;
            } else {
                log::info!("Playing queue of {} tracks", selected_urls.len());
                player::play_files(&selected_urls, cfg)?;
            }
        }
    }
    Ok(())
}
