use super::icons::Icons;
use super::items::MenuItem;
use super::runner::{run_skim_input_prompt, run_skim_simple};
use crate::config;
use crate::ui;
use anyhow::{Context, Result};
use skim::prelude::*;
use std::path::PathBuf;

pub fn run_manage_dirs_mode(cfg: &mut config::Config) -> Result<bool> {
    let mut any_changes = false;

    loop {
        let icons = Icons::new(cfg.nerd_fonts);
        let count = cfg.music_dirs.len();
        let prompt = format!(
            "{}Manage ({} dirs) >    ",
            icons.pad(icons.folder_open()),
            count
        );

        let options = vec![
            "1) Add Directory",
            "2) Remove Directory",
            "3) List Configured Directories",
            "q) Back",
        ];

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
            Some(s) if s.starts_with("3)") => {
                if cfg.music_dirs.is_empty() {
                    ui::warning("No music directories configured.");
                    std::thread::sleep(std::time::Duration::from_millis(1200));
                } else {
                    let dir_items: Vec<String> = cfg
                        .music_dirs
                        .iter()
                        .enumerate()
                        .map(|(i, d)| format!("{}. {} {}", i + 1, icons.folder(), d.display()))
                        .collect();
                    let list_prompt = format!(
                        "{}Directories ({}) > ",
                        icons.pad(icons.folder()),
                        cfg.music_dirs.len()
                    );
                    let _ = run_skim_simple(
                        dir_items.iter().map(|s| s.as_str()).collect(),
                        &list_prompt,
                    );
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
    let icons = Icons::new(cfg.nerd_fonts);
    let prompt = format!("{}(Add) Path > ", icons.pad(icons.folder_open()));
    let header = "Type a full path and press ENTER (empty to go back).";

    let mut changed = false;

    loop {
        let input = run_skim_input_prompt(&prompt, header);
        let path_str = match input {
            Some(s) if !s.is_empty() => s,
            _ => break,
        };

        // true if added a new path
        if add_directory(cfg, path_str)? {
            changed = true;
            std::thread::sleep(std::time::Duration::from_millis(1000));
        } else {
            // failed (typo/duplicate), sleep briefly for UX
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    }
    Ok(changed)
}

pub fn manage_remove_menu(cfg: &mut config::Config) -> Result<bool> {
    if cfg.music_dirs.is_empty() {
        ui::warning("No directories to remove.");
        std::thread::sleep(std::time::Duration::from_secs(1));
        return Ok(false);
    }

    let icons = Icons::new(cfg.nerd_fonts);
    let items: Vec<MenuItem> = cfg
        .music_dirs
        .iter()
        .map(|dir| {
            let dir_str = dir.display().to_string();
            MenuItem {
                text: format!("{} {}", icons.folder(), dir_str),
                id: dir_str,
            }
        })
        .collect();

    let remove_prompt = format!("{}Remove >    ", icons.pad(icons.trash()));
    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .prompt(&remove_prompt)
        .header("   Select directories to remove (TAB to select, ENTER to confirm)")
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

    let mut changed = false;
    ui::info("Processing removals...");
    for item in selected_items {
        let path_str = item.output().to_string();
        if remove_directory(cfg, path_str)? {
            changed = true;
        }
    }

    if changed {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }

    Ok(changed)
}

pub fn add_directory(cfg: &mut config::Config, dir: String) -> Result<bool> {
    let path_buf = PathBuf::from(&dir);

    if !path_buf.exists() {
        ui::error(format!("Path does not exist: \"{}\"", dir));
        return Ok(false);
    }

    if !path_buf.is_dir() {
        ui::error(format!("Path is not a directory: \"{}\"", dir));
        return Ok(false);
    }

    if let Err(e) = std::fs::read_dir(&path_buf) {
        ui::error(format!("Permission denied: Cannot access \"{}\"", dir));
        log::warn!("Access check failed for {:?}: {}", path_buf, e);
        return Ok(false);
    }

    let path = match dunce::canonicalize(&path_buf) {
        Ok(p) => p,
        Err(e) => {
            ui::error(format!("Failed to resolve absolute path: {}", e));
            return Ok(false);
        }
    };

    if !cfg.music_dirs.contains(&path) {
        cfg.music_dirs.push(path.clone());
        ui::success(format!("Added: {}", path.display()));
        Ok(true)
    } else {
        ui::warning(format!("Already exists: {}", path.display()));
        Ok(false)
    }
}

pub fn remove_directory(cfg: &mut config::Config, dir: String) -> Result<bool> {
    let path = dunce::canonicalize(&dir).unwrap_or_else(|_| PathBuf::from(&dir));
    let start_len = cfg.music_dirs.len();

    cfg.music_dirs.retain(|d| d != &path);

    if cfg.music_dirs.len() < start_len {
        ui::success(format!("Removed: {}", path.display()));
        Ok(true)
    } else {
        ui::warning(format!("Not found in config: {}", path.display()));
        Ok(false)
    }
}
