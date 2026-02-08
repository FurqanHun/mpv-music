use crate::cli::Cli;
use crate::config;
use crate::indexer;
use crate::player;
use crate::search;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use skim::prelude::*;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

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
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mChannel:\x1b[0m  {}\n  \x1b[1;33mViews:\x1b[0m    {}\n  \x1b[1;33mDuration:\x1b[0m {}\n  \x1b[1;34mType:\x1b[0m      {}\n\n  \x1b[90mURL: {}\x1b[0m",
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

pub fn run_main_menu(tracks: &mut Vec<indexer::Track>, cfg: &mut config::Config) -> Result<()> {
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

pub fn run_tag_mode(
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
pub fn run_tag_picker(tracks: &[indexer::Track], cfg: &config::Config, key: &str) -> Result<bool> {
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
                val
            };
            selected_names.contains(clean_val)
        })
        .cloned()
        .collect();

    run_post_filter_action(&filtered, cfg)?;

    Ok(true)
}

pub fn run_post_filter_action(tracks: &[indexer::Track], cfg: &config::Config) -> Result<()> {
    if tracks.is_empty() {
        return Ok(());
    }
    let paths: Vec<String> = tracks.iter().map(|t| t.path.clone()).collect();

    let opts = [
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

pub fn run_manage_dirs_mode(cfg: &mut config::Config) -> Result<bool> {
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

pub fn manage_add_loop(cfg: &mut config::Config) -> Result<bool> {
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

pub fn manage_remove_menu(cfg: &mut config::Config) -> Result<bool> {
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

pub fn add_directory(cfg: &mut config::Config, dir: String) -> Result<bool> {
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

pub fn remove_directory(cfg: &mut config::Config, dir: String) -> Result<bool> {
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

pub fn apply_cli_filters(
    tracks: &[indexer::Track],
    args: &Cli,
    exact: bool,
) -> Vec<indexer::Track> {
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
                                .split([';', ','])
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
                    .is_none_or(|ti| t.title.to_lowercase().contains(ti))
        })
        .cloned()
        .collect()
}

pub fn run_settings_menu(tracks: &mut Vec<indexer::Track>, cfg: &mut config::Config) -> Result<()> {
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

pub fn run_skim_simple(items: Vec<&str>, prompt: &str) -> Option<String> {
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

pub fn run_skim_multi_selection(items: Vec<String>, prompt: &str) -> Option<Vec<String>> {
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for i in items {
        tx.send(vec![Arc::new(MenuItem {
            text: i.clone(),
            id: i,
        })])
        .unwrap();
    }
    drop(tx);

    let opts = SkimOptionsBuilder::default()
        .height("50%".to_string())
        .reverse(true)
        .prompt(prompt.to_string())
        .inline_info(true)
        .multi(true)
        .build()
        .unwrap();

    if let Ok(output) = Skim::run_with(opts, Some(rx)) {
        if output.is_abort {
            return None;
        }

        let selections: Vec<String> = output
            .selected_items
            .iter()
            .map(|i| i.text().to_string())
            .collect();

        if selections.is_empty() {
            None
        } else {
            Some(selections)
        }
    } else {
        None
    }
}

pub fn run_track_mode(tracks: &[indexer::Track], cfg: &config::Config) -> Result<()> {
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
        .header(Some("   Artist              Title".to_string()))
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

pub fn run_dir_mode(tracks: &[indexer::Track], cfg: &config::Config) -> Result<()> {
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
            path,
            count,
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

pub fn run_playlist_mode(tracks: &[indexer::Track], cfg: &config::Config) -> Result<()> {
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

pub fn run_search_mode(cfg: &config::Config, initial_query: Option<String>) -> Result<()> {
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

    if let Ok(output) = Skim::run_with(opts, Some(rx)) {
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
