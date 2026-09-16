pub mod dirs;
pub mod filter;
pub mod icons;
pub(crate) mod items;
pub mod radio;
pub mod runner;
pub mod search;
pub mod tags;
pub mod tracks;

pub use dirs::{add_directory, remove_directory, run_manage_dirs_mode};
pub use filter::apply_cli_filters;
pub use icons::Icons;
pub use radio::run_radio_mode;
pub use runner::{run_skim_multi_selection, run_skim_simple};
pub use search::run_search_mode;
pub use tags::{run_post_filter_action, run_tag_mode};
pub use tracks::{run_dir_mode, run_playlist_mode, run_track_mode};

use crate::config;
use crate::indexer;
use crate::player;
use crate::ui;
use anyhow::Result;
use directories::ProjectDirs;

pub fn run_main_menu(
    tracks: &mut Vec<indexer::Track>,
    cfg: &mut config::Config,
    extra_args: &[String],
) -> Result<()> {
    loop {
        let icons = Icons::new(cfg.nerd_fonts);
        let options = vec![
            "1) Directory Mode",
            "2) Track Mode",
            "3) Playlist Mode",
            "4) Tag Filter Mode",
            "5) Play All Mode",
            "6) Search & Stream URL",
            "7) Radio Mode",
            "8) Settings",
            "q) Quit",
        ];
        let mode_prompt = icons.prompt(icons.mode_prompt(), "Pick mode");
        let selected = run_skim_simple(options, &mode_prompt);

        if let Some(ref s) = selected {
            let is_local_mode = s.starts_with("1)")
                || s.starts_with("2)")
                || s.starts_with("3)")
                || s.starts_with("4)")
                || s.starts_with("5)");

            if is_local_mode && tracks.is_empty() {
                let _ = run_skim_simple(
                    vec!["q) Back"],
                    "No tracks in library! Use Settings (8) to add directories.",
                );
                continue;
            }
        }

        match selected.as_deref() {
            Some(s) if s.starts_with("1)") => run_dir_mode(tracks, cfg, extra_args)?,
            Some(s) if s.starts_with("2)") => run_track_mode(tracks, cfg, extra_args)?,
            Some(s) if s.starts_with("3)") => run_playlist_mode(tracks, cfg, extra_args)?,
            Some(s) if s.starts_with("4)") => run_tag_mode(tracks, cfg, None, extra_args)?,
            Some(s) if s.starts_with("5)") => {
                let paths: Vec<String> = tracks.iter().map(|t| t.path.clone()).collect();
                player::play_files(&paths, cfg, extra_args)?;
            }
            Some(s) if s.starts_with("6)") => {
                run_search_mode(cfg, None, extra_args)?;
            }
            Some(s) if s.starts_with("7)") => {
                run_radio_mode(cfg, extra_args, None)?;
            }
            Some(s) if s.starts_with("8)") => run_settings_menu(tracks, cfg)?,
            Some(s) if s.starts_with("q)") => break,
            None => break,
            _ => {}
        }
    }
    Ok(())
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

        let icons = Icons::new(cfg.nerd_fonts);
        let settings_prompt = icons.prompt(icons.settings(), "Settings");
        let selection = run_skim_simple(options, &settings_prompt);
        match selection.as_deref() {
            Some(s) if s.contains("Manage Directories") => {
                if run_manage_dirs_mode(cfg)? {
                    config::save(cfg)?;
                    ui::success("Configuration saved.");
                    ui::info("Syncing library index with updated directories...");
                    *tracks = indexer::scan(cfg, false)?;
                    indexer::save(tracks)?;
                    ui::success(format!("Library synced ({} tracks).", tracks.len()));
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            }

            Some(s) if s.contains("Edit Config") => {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
                    if cfg!(windows) {
                        "notepad".to_string()
                    } else {
                        "nano".to_string()
                    }
                });
                let config_path = ProjectDirs::from("com", "furqanhun", "mpv-music")
                    .unwrap()
                    .config_dir()
                    .join("config.toml");

                std::process::Command::new(editor)
                    .arg(&config_path)
                    .status()?;

                *cfg = config::load(None)?;
                ui::success("Configuration reloaded from disk.");
                std::thread::sleep(std::time::Duration::from_millis(900));
            }
            Some(s) if s.contains("Delete Config") => {
                let config_path = ProjectDirs::from("com", "furqanhun", "mpv-music")
                    .unwrap()
                    .config_dir()
                    .join("config.toml");
                if config_path.exists() {
                    std::fs::remove_file(&config_path)?;
                    ui::warning("Configuration deleted. Resetting to defaults...");
                    *cfg = config::load(None)?;
                    ui::success("Default configuration loaded.");
                } else {
                    ui::warning("No configuration file found.");
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            Some(s) if s.contains("View Log") => {
                let data_dir = ProjectDirs::from("com", "furqanhun", "mpv-music")
                    .unwrap()
                    .data_dir()
                    .to_path_buf();
                let logs = crate::app::logging::list_log_files(&data_dir);

                if logs.is_empty() {
                    ui::warning("Log file does not exist.");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }

                let viewer = std::env::var("PAGER").unwrap_or_else(|_| {
                    if cfg!(windows) {
                        "notepad.exe".to_string()
                    } else {
                        "less".to_string()
                    }
                });

                let target_file = if logs.len() == 1 {
                    logs[0].clone()
                } else {
                    let items: Vec<String> = logs
                        .iter()
                        .enumerate()
                        .map(|(idx, path)| crate::app::logging::format_log_display(path, idx == 0))
                        .collect();

                    let prompt = "Select Log Session > ";
                    let options: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                    if let Some(selected) = runner::run_skim_simple(options, prompt) {
                        if let Some(idx) = items.iter().position(|item| item == &selected) {
                            logs[idx].clone()
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                };

                let _ = std::process::Command::new(viewer).arg(target_file).status();
            }
            Some(s) if s.contains("Delete Log") => {
                let data_dir = ProjectDirs::from("com", "furqanhun", "mpv-music")
                    .unwrap()
                    .data_dir()
                    .to_path_buf();
                let logs = crate::app::logging::list_log_files(&data_dir);

                if logs.is_empty() {
                    ui::warning("No log file found to delete.");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }

                if logs.len() == 1 {
                    if std::fs::remove_file(&logs[0]).is_ok() {
                        ui::success("Log file deleted.");
                    } else {
                        ui::error("Failed to delete log file.");
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }

                let opts = [
                    format!("1) Delete all {} log files", logs.len()),
                    "2) Select individual log files".to_string(),
                ];
                let pick = runner::run_skim_simple(
                    opts.iter().map(|s| s.as_str()).collect(),
                    &icons.prompt(icons.trash(), "Delete Logs"),
                );

                match pick.as_deref() {
                    Some(s) if s.starts_with("1)") => {
                        let mut count = 0;
                        for file in &logs {
                            if std::fs::remove_file(file).is_ok() {
                                count += 1;
                            }
                        }
                        ui::success(format!("Deleted all {} log file(s).", count));
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                    Some(s) if s.starts_with("2)") => {
                        let items: Vec<String> = logs
                            .iter()
                            .enumerate()
                            .map(|(idx, path)| {
                                crate::app::logging::format_log_display(path, idx == 0)
                            })
                            .collect();

                        let prompt = "Delete Logs (TAB to select, ENTER to delete) > ";
                        if let Some(selected) =
                            runner::run_skim_multi_selection(items.clone(), prompt)
                            && !selected.is_empty()
                        {
                            let mut count = 0;
                            for sel in selected {
                                if let Some(idx) = items.iter().position(|item| item == &sel)
                                    && std::fs::remove_file(&logs[idx]).is_ok()
                                {
                                    count += 1;
                                }
                            }
                            ui::success(format!("Deleted {} log file(s).", count));
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    }
                    _ => {}
                }
            }

            Some(s) if s.contains("Refresh Index") => {
                ui::info("Refreshing library index (Fast)...");
                *tracks = indexer::scan(cfg, false)?;
                indexer::save(tracks)?;
                ui::success(format!("Index refreshed ({} tracks).", tracks.len()));
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            Some(s) if s.contains("Rebuild Index") => {
                ui::info("Rebuilding library index from scratch (Full)...");
                *tracks = indexer::scan(cfg, true)?;
                indexer::save(tracks)?;
                ui::success(format!("Index rebuilt ({} tracks).", tracks.len()));
                std::thread::sleep(std::time::Duration::from_secs(1));
            }

            Some(s) if s.starts_with("q)") => break,
            None => break,
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_url_detection_https() {
        let line = "https://youtube.com/watch?v=test";
        assert!(line.starts_with("https://"));
    }

    #[test]
    fn test_not_a_url() {
        let line = "/home/user/Music/song.mp3";
        let is_url = line.starts_with("http://")
            || line.starts_with("https://")
            || line.starts_with("ftp://");
        assert!(!is_url);
    }

    #[test]
    fn test_absolute_path_unix() {
        let path = std::path::PathBuf::from("/home/user/song.mp3");
        assert!(path.is_absolute());
    }

    #[test]
    fn test_relative_path_parent() {
        let path = std::path::PathBuf::from("../Music/song.mp3");
        assert!(!path.is_absolute());
    }
}
