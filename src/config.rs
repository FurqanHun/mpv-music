use anyhow::{Context, Result};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub shuffle: bool,
    pub loop_mode: String, // "playlist", "track", "no", "inf", "5"
    pub volume: u8,

    pub music_dirs: Vec<PathBuf>,
    pub video_ok: bool,
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
}

impl Default for Config {
    fn default() -> Self {
        log::debug!("Generating default configuration...");

        let mut music_dirs = Vec::new();
        if let Some(user_dirs) = UserDirs::new() {
            if let Some(audio) = user_dirs.audio_dir() {
                log::debug!("Detected XDG audio directory: {:?}", audio);
                music_dirs.push(audio.to_path_buf());
            } else {
                if let Ok(home) = std::env::var("HOME") {
                    let fallback = PathBuf::from(home).join("Music");
                    log::debug!("No XDG dir found, using fallback: {:?}", fallback);
                    music_dirs.push(fallback);
                }
            }
        }

        let banner_text = "╔══  MPV-MUSIC  ══╗";
        let status_msg = "▶ ${?metadata/artist:${metadata/artist} - }${?metadata/title:${metadata/title}}${!metadata/title:${media-title}} • ${time-pos} / ${duration} • (${percent-pos}%)";

        Self {
            shuffle: true,
            loop_mode: "inf".to_string(),
            volume: 60,
            music_dirs,
            video_ok: false,
            serial_mode: true,
            ytdlp_ejs_remote_github: true,
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

    let cfg: Config = toml::from_str(&content).context("Failed to parse config.toml")?;

    log::debug!("Successfully parsed {} bytes of TOML", content.len());

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
