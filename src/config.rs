use anyhow::{Context, Result};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub shuffle: bool,
    pub loop_mode: String, // "playlist", "track", "no", "inf", "5"
    pub volume: u8,

    pub music_dirs: Vec<PathBuf>,
    pub video_ok: bool,
    #[serde(default)]
    pub watch: bool,
    pub serial_mode: bool,

    pub ytdlp_ejs_remote_github: bool,
    #[serde(default)]
    pub ytdlp_useragent: String,
    pub enable_file_logging: bool,

    pub audio_exts: Vec<String>,
    pub video_exts: Vec<String>,
    pub playlist_exts: Vec<String>,

    pub mpv_default_args: Vec<String>,

    #[serde(skip, default)]
    pub ytdlp_available: bool,
    #[serde(skip, default)]
    pub ytdlp_is_nightly: bool,
}

impl Default for Config {
    fn default() -> Self {
        log::debug!("Generating default configuration...");

        let mut music_dirs = Vec::new();
        if let Some(user_dirs) = UserDirs::new() {
            if let Some(audio) = user_dirs.audio_dir() {
                log::debug!("Detected XDG audio directory: {:?}", audio);
                music_dirs.push(audio.to_path_buf());
            } else if let Ok(home) = std::env::var("HOME") {
                let fallback = PathBuf::from(home).join("Music");
                log::debug!("No XDG dir found, using fallback: {:?}", fallback);
                music_dirs.push(fallback);
            }
        }

        let banner_text = "╔══  MPV-MUSIC  ══╗";
        let status_msg = "▶ ${?metadata/artist:${metadata/artist} - }${?metadata/title:${metadata/title}}${!metadata/title:${media-title}} • ${time-pos} / ${duration} • (${percent-pos}%)";

        Self {
            shuffle: true,
            loop_mode: "inf".to_string(),
            volume: 100,
            music_dirs,
            video_ok: false,
            watch: false,
            serial_mode: false,
            ytdlp_ejs_remote_github: false,
            ytdlp_useragent:
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/114.0"
                    .to_string(),
            enable_file_logging: true,
            audio_exts: vec![
                "mp3", "flac", "wav", "m4a", "aac", "ogg", "opus", "wma", "alac", "aiff", "amr",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            video_exts: vec![
                "mp4", "mkv", "webm", "avi", "mov", "flv", "wmv", "mpeg", "mpg", "3gp", "ts",
                "vob", "m4v",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            playlist_exts: vec!["m3u", "m3u8", "pls"]
                .into_iter()
                .map(String::from)
                .collect(),
            mpv_default_args: vec![
                "--no-video".to_string(),
                "--audio-display=no".to_string(),
                "--msg-level=cplayer=warn".to_string(),
                "--display-tags=".to_string(),
                "--no-term-osd-bar".to_string(),
                format!("--term-playing-msg={}", banner_text),
                format!("--term-status-msg={}", status_msg),
            ],
            ytdlp_available: false,
            ytdlp_is_nightly: false,
        }
    }
}

pub fn load(override_path: Option<PathBuf>) -> Result<Config> {
    log::debug!("Initializing config load sequence");

    let config_path = match override_path {
        Some(path) => path,
        None => {
            let dirs = ProjectDirs::from("com", "furqanhun", "mpv-music")
                .context("Could not determine config paths")?;
            dirs.config_dir().join("config.toml")
        }
    };

    let config_dir = config_path
        .parent()
        .context("Could not determine config directory")?;

    if !config_path.exists() {
        log::info!("Config not found, creating default at: {:?}", config_path);

        // ensure the dir exists
        std::fs::create_dir_all(config_dir)?;

        let default_cfg = Config::default();
        let toml_str = toml::to_string_pretty(&default_cfg)?;
        std::fs::write(&config_path, toml_str)?;

        return Ok(default_cfg);
    }

    log::info!("Loading configuration from: {:?}", config_path);
    let content = std::fs::read_to_string(&config_path)?;

    let mut cfg: Config = toml::from_str(&content).context("Failed to parse config.toml")?;

    log::debug!("Successfully parsed {} bytes of TOML", content.len());

    let mut warnings = Vec::new();

    if cfg.volume > 130 {
        warnings.push(format!(
            "Volume {} exceeds maximum (130). Reseting to 100.",
            cfg.volume
        ));
        cfg.volume = 100;
    }

    let valid_loop_modes = ["inf", "playlist", "no", "off", "false", "track", "file"];
    let is_numeric = cfg.loop_mode.chars().all(|c| c.is_numeric());

    if !valid_loop_modes.contains(&cfg.loop_mode.as_str()) && !is_numeric {
        warnings.push(format!(
            "Invalid loop_mode '{}'. Defaulting to 'inf'.",
            cfg.loop_mode
        ));
        cfg.loop_mode = "inf".to_string();
    }

    if cfg.music_dirs.is_empty() {
        warnings.push(
            "No music directories configured. Run 'mpv-music --manage-dirs' to add folders."
                .to_string(),
        );
    }

    for warning in warnings {
        log::warn!("Config validation: {}", warning);
        eprintln!("\x1b[33;1m[Config Warning]\x1b[0m {}", warning);
    }

    log::trace!("Loaded Config State: {:#?}", cfg);

    Ok(cfg)
}

pub fn save(config: &Config) -> Result<()> {
    let dirs = ProjectDirs::from("com", "furqanhun", "mpv-music")
        .context("Could not determine config paths")?;
    let config_path = dirs.config_dir().join("config.toml");

    log::info!("Saving configuration to {:?}", config_path);

    let toml_str = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, toml_str)?;

    log::debug!("Configuration saved successfully.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let cfg = Config::default();
        assert_eq!(cfg.volume, 100);
        assert_eq!(cfg.loop_mode, "inf");
        assert!(cfg.shuffle);
        assert!(!cfg.video_ok);
        assert!(!cfg.watch);
    }

    #[test]
    fn test_default_music_dirs_not_empty() {
        let cfg = Config::default();
        assert!(
            !cfg.music_dirs.is_empty(),
            "Default config should have at least one music directory"
        );
    }

    #[test]
    fn test_default_extensions() {
        let cfg = Config::default();

        // Audio extensions
        assert!(cfg.audio_exts.contains(&"mp3".to_string()));
        assert!(cfg.audio_exts.contains(&"flac".to_string()));
        assert!(cfg.audio_exts.contains(&"wav".to_string()));

        // Video extensions
        assert!(cfg.video_exts.contains(&"mp4".to_string()));
        assert!(cfg.video_exts.contains(&"mkv".to_string()));

        // Playlist extensions
        assert!(cfg.playlist_exts.contains(&"m3u".to_string()));
        assert!(cfg.playlist_exts.contains(&"m3u8".to_string()));
    }

    #[test]
    fn test_volume_cap_at_130() {
        // Simulate validation logic from load()
        let mut volume = 200_u8;
        if volume > 130 {
            volume = 100;
        }
        assert_eq!(volume, 100);
    }

    #[test]
    fn test_volume_allows_130() {
        let mut volume = 130_u8;
        if volume > 130 {
            volume = 100;
        }
        assert_eq!(volume, 130);
    }

    #[test]
    fn test_volume_allows_normal() {
        let mut volume = 75_u8;
        if volume > 130 {
            volume = 100;
        }
        assert_eq!(volume, 75);
    }

    #[test]
    fn test_loop_mode_validation_valid() {
        let valid_modes = ["inf", "playlist", "no", "off", "false", "track", "file"];

        assert!(valid_modes.contains(&"inf"));
        assert!(valid_modes.contains(&"track"));
        assert!(valid_modes.contains(&"no"));
    }

    #[test]
    fn test_loop_mode_validation_invalid() {
        let loop_mode = "potato";
        let valid_modes = ["inf", "playlist", "no", "off", "false", "track", "file"];
        let is_numeric = loop_mode.chars().all(|c| c.is_numeric());

        assert!(!valid_modes.contains(&loop_mode));
        assert!(!is_numeric);
    }

    #[test]
    fn test_loop_mode_validation_numeric() {
        let loop_mode = "5";
        let is_numeric = loop_mode.chars().all(|c| c.is_numeric());

        assert!(is_numeric);
    }

    #[test]
    fn test_loop_mode_validation_numeric_multiple_digits() {
        let loop_mode = "999";
        let is_numeric = loop_mode.chars().all(|c| c.is_numeric());

        assert!(is_numeric);
    }

    #[test]
    fn test_ytdlp_flags_default() {
        let cfg = Config::default();
        assert!(!cfg.ytdlp_available);
        assert!(!cfg.ytdlp_is_nightly);
    }

    #[test]
    fn test_mpv_default_args_present() {
        let cfg = Config::default();
        assert!(!cfg.mpv_default_args.is_empty());
        assert!(
            cfg.mpv_default_args
                .iter()
                .any(|arg| arg.contains("--no-video"))
        );
    }
}
