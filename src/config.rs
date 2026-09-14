use anyhow::{Context, Result};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_YTDLP_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:155.0) Gecko/20100101 Firefox/155.0";

fn default_ytdlp_useragent() -> String {
    "default".to_string()
}

fn default_ytdlp() -> String {
    "yt-dlp".to_string()
}

fn default_player() -> String {
    "mpv".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum NerdFontMode {
    #[default]
    None,
    Mono,
    Normal,
}

fn deserialize_nerd_fonts<'de, D>(deserializer: D) -> std::result::Result<NerdFontMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct NerdFontVisitor;

    impl<'de> serde::de::Visitor<'de> for NerdFontVisitor {
        type Value = NerdFontMode;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a boolean or string (\"none\", \"mono\", \"normal\", \"symbols\")")
        }

        fn visit_bool<E>(self, value: bool) -> std::result::Result<NerdFontMode, E>
        where
            E: serde::de::Error,
        {
            Ok(if value {
                NerdFontMode::Mono
            } else {
                NerdFontMode::None
            })
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<NerdFontMode, E>
        where
            E: serde::de::Error,
        {
            match value.to_lowercase().as_str() {
                "none" | "off" | "false" | "no" => Ok(NerdFontMode::None),
                "mono" | "true" | "yes" | "on" => Ok(NerdFontMode::Mono),
                "normal" | "symbols" | "prop" | "propo" => Ok(NerdFontMode::Normal),
                other => Err(E::custom(format!(
                    "Invalid nerd_fonts value '{}'. Valid options are: \"none\", \"mono\", \"normal\"",
                    other
                ))),
            }
        }
    }

    deserializer.deserialize_any(NerdFontVisitor)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub shuffle: bool,
    pub loop_mode: String, // "playlist", "track", "no", "inf", "5"
    pub volume: u8,

    pub music_dirs: Vec<PathBuf>,
    pub video_ok: bool,
    #[serde(default)]
    pub watch: bool,
    #[serde(default)]
    pub scan_hidden_dirs: bool,
    pub serial_mode: bool,

    #[serde(default, deserialize_with = "deserialize_nerd_fonts")]
    pub nerd_fonts: NerdFontMode,

    pub ytdlp_ejs_remote_github: bool,
    #[serde(default = "default_ytdlp_useragent")]
    pub ytdlp_useragent: String,
    pub enable_file_logging: bool,

    #[serde(default = "default_ytdlp")]
    pub ytdlp: String,

    #[serde(default = "default_player")]
    pub player: String,

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
            scan_hidden_dirs: false,
            serial_mode: false,
            nerd_fonts: NerdFontMode::None,
            ytdlp_ejs_remote_github: false,
            ytdlp_useragent: default_ytdlp_useragent(),
            enable_file_logging: true,
            ytdlp: default_ytdlp(),
            player: default_player(),
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

impl Config {
    pub fn player_bin(&self) -> &str {
        let p = self.player.trim();
        if p.is_empty() || p.eq_ignore_ascii_case("default") || p.eq_ignore_ascii_case("mpv") {
            if cfg!(windows) { "mpv.com" } else { "mpv" }
        } else {
            p
        }
    }

    pub fn ytdlp_bin(&self) -> &str {
        let y = self.ytdlp.trim();
        if y.is_empty() || y.eq_ignore_ascii_case("default") {
            "yt-dlp"
        } else {
            y
        }
    }
}

/// If no configuration exists, it creates one with default values.
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
    let mut needs_save = false;

    let legacy_ua =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/114.0";
    if cfg.ytdlp_useragent == legacy_ua {
        log::info!("Migrating legacy ytdlp_useragent to new default");
        cfg.ytdlp_useragent = default_ytdlp_useragent();
        warnings.push("Migrated legacy yt-dlp user agent to the new default.".to_string());
        needs_save = true;
    }

    if cfg.volume > 130 {
        warnings.push(format!(
            "Volume {} exceeds maximum (130). Reseting to 100.",
            cfg.volume
        ));
        cfg.volume = 100;
        needs_save = true;
    }

    let valid_loop_modes = ["inf", "playlist", "no", "off", "false", "track", "file"];
    let is_numeric = cfg.loop_mode.chars().all(|c| c.is_numeric());

    if !valid_loop_modes.contains(&cfg.loop_mode.as_str()) && !is_numeric {
        warnings.push(format!(
            "Invalid loop_mode '{}'. Defaulting to 'inf'.",
            cfg.loop_mode
        ));
        cfg.loop_mode = "inf".to_string();
        needs_save = true;
    }

    if cfg.music_dirs.is_empty() {
        warnings.push(
            "No music directories configured. Run 'mpv-music --manage-dirs' to add folders."
                .to_string(),
        );
    }

    for warning in warnings {
        log::warn!("Config validation: {}", warning);
        eprintln!("\x1b[33;1m[Warning]\x1b[0m Config: {}", warning);
    }

    if needs_save {
        if let Err(e) = save(&cfg) {
            log::error!("Failed to save auto-corrected config: {}", e);
        }
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

    #[test]
    fn test_nerd_fonts_deserialization() {
        #[derive(Deserialize)]
        struct TestCfg {
            #[serde(default, deserialize_with = "deserialize_nerd_fonts")]
            nerd_fonts: NerdFontMode,
        }

        let cases = vec![
            ("nerd_fonts = 'mono'", NerdFontMode::Mono),
            ("nerd_fonts = true", NerdFontMode::Mono),
            ("nerd_fonts = 'normal'", NerdFontMode::Normal),
            ("nerd_fonts = 'symbols'", NerdFontMode::Normal),
            ("nerd_fonts = 'none'", NerdFontMode::None),
            ("nerd_fonts = false", NerdFontMode::None),
            ("", NerdFontMode::None),
        ];

        for (toml_input, expected) in cases {
            let parsed: TestCfg = toml::from_str(toml_input)
                .unwrap_or_else(|e| panic!("Failed to parse '{}': {}", toml_input, e));
            assert_eq!(parsed.nerd_fonts, expected, "Failed for: {}", toml_input);
        }
    }

    #[test]
    fn test_player_configuration() {
        let default_cfg = Config::default();
        assert_eq!(default_cfg.player, "mpv");
        if cfg!(windows) {
            assert_eq!(default_cfg.player_bin(), "mpv.com");
        } else {
            assert_eq!(default_cfg.player_bin(), "mpv");
        }

        let mut custom_cfg = Config::default();
        custom_cfg.player = "mpvnet".to_string();
        assert_eq!(custom_cfg.player_bin(), "mpvnet");

        let mut custom_path = Config::default();
        custom_path.player = "/usr/local/bin/my-mpv".to_string();
        assert_eq!(custom_path.player_bin(), "/usr/local/bin/my-mpv");

        let mut custom_exe = Config::default();
        custom_exe.player = "mpv.exe".to_string();
        assert_eq!(custom_exe.player_bin(), "mpv.exe");
    }

    #[test]
    fn test_ytdlp_configuration() {
        let default_cfg = Config::default();
        assert_eq!(default_cfg.ytdlp, "yt-dlp");
        assert_eq!(default_cfg.ytdlp_bin(), "yt-dlp");

        let mut custom_cfg = Config::default();
        custom_cfg.ytdlp = "yt-dlp-nightly".to_string();
        assert_eq!(custom_cfg.ytdlp_bin(), "yt-dlp-nightly");

        let mut custom_path = Config::default();
        custom_path.ytdlp = "/usr/local/bin/yt-dlp".to_string();
        assert_eq!(custom_path.ytdlp_bin(), "/usr/local/bin/yt-dlp");
    }
}
