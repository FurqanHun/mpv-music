use super::icons::Icons;
use super::items::TagItem;
use super::runner::run_skim_simple;
use super::tracks::run_track_mode;
use crate::config;
use crate::indexer;
use crate::player;
use anyhow::{Context, Result};
use skim::prelude::*;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};

pub fn run_tag_mode(
    tracks: &[indexer::Track],
    cfg: &config::Config,
    force_key: Option<&str>,
    extra_args: &[String],
) -> Result<()> {
    // if a key is forced (like from cli -g), we don't loop/menu, just run once
    if let Some(k) = force_key {
        let _ = run_tag_picker(tracks, cfg, k, extra_args)?;
        return Ok(());
    }

    loop {
        let icons = Icons::new(cfg.nerd_fonts);
        let choices = vec!["1) Genre", "2) Artist", "3) Album", "q) Back"];
        let filter_prompt = icons.prompt(icons.filter(), "Filter by");
        let choice = run_skim_simple(choices, &filter_prompt);

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
        if run_tag_picker(tracks, cfg, key, extra_args)? {
            return Ok(());
        }
    }
}

// helper to keep the logic clean, returns true if action taken, false if aborted (ESC).
pub fn run_tag_picker(
    tracks: &[indexer::Track],
    cfg: &config::Config,
    key: &str,
    extra_args: &[String],
) -> Result<bool> {
    let icons = Icons::new(cfg.nerd_fonts);
    let (icon, prompt) = match key {
        "genre" => (icons.genre(), icons.prompt(icons.genre(), "Pick Genre")),
        "artist" => (icons.artist(), icons.prompt(icons.artist(), "Pick Artist")),
        "album" => (icons.album(), icons.prompt(icons.album(), "Pick Album")),
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

    let mut sorted_keys: Vec<_> = counts.keys().collect();
    sorted_keys.sort();

    let items: Vec<TagItem> = sorted_keys
        .into_iter()
        .map(|k| {
            let count = *counts.get(k).unwrap();
            let sample_list = samples.get(k).unwrap().clone();

            TagItem {
                name: k.clone(),
                count,
                samples: sample_list,
                icon: icon.to_string(),
            }
        })
        .collect();

    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt(&prompt)
        .preview("")
        .reverse(true)
        //.typos(2)
        .inline_info(true)
        .build()
        .unwrap();

    let output = Skim::run_items(opts, items).ok().context("Skim failed")?;

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

    // Reference approach: just collect references, no cloning here.
    let filtered: Vec<&indexer::Track> = tracks
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
        .collect();

    run_post_filter_action(&filtered, cfg, extra_args)?;

    Ok(true)
}

pub fn run_post_filter_action<T>(
    tracks: &[T],
    cfg: &config::Config,
    extra_args: &[String],
) -> Result<()>
where
    T: Borrow<indexer::Track>,
{
    if tracks.is_empty() {
        return Ok(());
    }

    if tracks.len() == 1 {
        let t = tracks[0].borrow();
        player::play(&t.path, cfg, extra_args)?;
        return Ok(());
    }

    let paths: Vec<String> = tracks.iter().map(|t| t.borrow().path.clone()).collect();

    let opts = [
        format!("1) Play all {} tracks", tracks.len()),
        "2) Select individual tracks".to_string(),
    ];
    let pick = run_skim_simple(opts.iter().map(|s| s.as_str()).collect(), "What's next? ");
    match pick.as_deref() {
        Some(s) if s.starts_with("1)") => player::play_files(&paths, cfg, extra_args),
        Some(s) if s.starts_with("2)") => run_track_mode(tracks, cfg, extra_args),
        _ => Ok(()),
    }
}
