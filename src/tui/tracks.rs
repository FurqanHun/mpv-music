use super::icons::Icons;
use super::items::{DirItem, PlaylistItem, TrackItem};
use crate::config;
use crate::indexer;
use crate::player;
use anyhow::{Context, Result};
use skim::prelude::*;
use std::borrow::Borrow;
use std::collections::HashMap;

pub fn run_track_mode<T>(tracks: &[T], cfg: &config::Config, extra_args: &[String]) -> Result<()>
where
    T: Borrow<indexer::Track>,
{
    let icons = Icons::new(cfg.nerd_fonts);
    let skim_items: Vec<TrackItem> = tracks
        .iter()
        .filter_map(|item| {
            let track = item.borrow();
            if track.is_playlist() {
                return None;
            }
            let display = format!("{} - {}", track.artist, track.title);
            let icon = if track.is_video() {
                icons.video()
            } else {
                icons.track()
            };

            Some(TrackItem {
                track: track.clone(),
                display_text: display,
                icon,
            })
        })
        .collect();

    let track_prompt = icons.prompt(icons.track(), "Tracks");
    let opts = SkimOptionsBuilder::default()
        .height("100%")
        .multi(true)
        .preview("")
        .prompt(&track_prompt)
        .header("   Artist                Title")
        .reverse(true)
        //.typos(2)
        .inline_info(true)
        .build()
        .unwrap();

    let output = Skim::run_items(opts, skim_items)
        .ok()
        .context("Skim failed")?;
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

    player::play_files(&paths, cfg, extra_args)?;
    Ok(())
}

pub fn run_dir_mode(
    tracks: &[indexer::Track],
    cfg: &config::Config,
    extra_args: &[String],
) -> Result<()> {
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

    let icons = Icons::new(cfg.nerd_fonts);
    let skim_items: Vec<DirItem> = dir_map
        .into_iter()
        .map(|(path, files)| {
            let count = files.len();
            let name = std::path::Path::new(&path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            DirItem {
                dirname: name,
                path,
                count,
                samples: files,
                icon: icons.folder(),
            }
        })
        .collect();

    let folder_prompt = icons.prompt(icons.folder(), "Folders");
    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt(&folder_prompt)
        .header("   Directory Name")
        .reverse(true)
        //.typos(2)
        .inline_info(true)
        .preview("")
        .build()
        .unwrap();

    let output = Skim::run_items(opts, skim_items)
        .ok()
        .context("Skim failed")?;
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
    player::play_files(&files, cfg, extra_args)
}

pub fn run_playlist_mode(
    tracks: &[indexer::Track],
    cfg: &config::Config,
    extra_args: &[String],
) -> Result<()> {
    let icons = Icons::new(cfg.nerd_fonts);
    let skim_items: Vec<PlaylistItem> = tracks
        .iter()
        .filter_map(|t| {
            if t.is_playlist() {
                let (count, lines) = if let Ok(content) = std::fs::read_to_string(&t.path) {
                    let playlist_dir = std::path::Path::new(&t.path)
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."));

                    let all_valid_lines: Vec<String> = content
                        .lines()
                        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                        .filter_map(|line| {
                            let line_trim = line.trim();

                            if line_trim.starts_with("http://")
                                || line_trim.starts_with("https://")
                                || line_trim.starts_with("ftp://")
                            {
                                return Some(line_trim.to_string());
                            }

                            let path = std::path::PathBuf::from(line_trim);

                            if path.is_absolute() {
                                if path.exists() {
                                    Some(line_trim.to_string())
                                } else {
                                    log::debug!(
                                        "Skipping non-existent path in playlist: {}",
                                        line_trim
                                    );
                                    None
                                }
                            } else {
                                match dunce::canonicalize(playlist_dir.join(&path)) {
                                    Ok(canonical) => Some(canonical.to_string_lossy().to_string()),
                                    Err(_) => {
                                        log::debug!(
                                            "Could not resolve relative path in playlist: {}",
                                            line_trim
                                        );
                                        None
                                    }
                                }
                            }
                        })
                        .collect();

                    let total = all_valid_lines.len();
                    let sample = all_valid_lines.into_iter().take(15).collect();

                    (total, sample)
                } else {
                    (0, vec!["(Could not read file)".to_string()])
                };

                Some(PlaylistItem {
                    name: t.title.clone(),
                    path: t.path.clone(),
                    count,
                    preview_lines: lines,
                    icon: icons.playlist(),
                })
            } else {
                None
            }
        })
        .collect();

    let playlist_prompt = icons.prompt(icons.playlist(), "Playlists");
    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt(&playlist_prompt)
        .reverse(true)
        //.typos(2)
        .inline_info(true)
        .preview("")
        .build()
        .unwrap();

    let output = Skim::run_items(opts, skim_items)
        .ok()
        .context("Skim failed")?;
    if output.is_abort {
        return Ok(());
    }

    if let Some(item) = output.selected_items.first() {
        player::play(&item.output(), cfg, extra_args)?;
    }
    Ok(())
}
