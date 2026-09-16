use crate::ui;
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

fn default_max_log_sessions() -> usize {
    3
}

pub const LEGACY_MPV_DEFAULT_ARGS: &[&str] = &[
    "--no-video",
    "--audio-display=no",
    "--msg-level=cplayer=warn",
    "--display-tags=",
    "--no-term-osd-bar",
    "--term-playing-msg=╔══  MPV-MUSIC  ══╗",
    "--term-status-msg=▶ ${?metadata/artist:${metadata/artist} - }${?metadata/title:${metadata/title}}${!metadata/title:${media-title}} • ${time-pos} / ${duration} • (${percent-pos}%)",
];

pub const KNOWN_CONFIG_KEYS: &[&str] = &[
    "shuffle",
    "loop_mode",
    "volume",
    "music_dirs",
    "video_ok",
    "watch",
    "scan_hidden_dirs",
    "serial_mode",
    "nerd_fonts",
    "ytdlp_ejs_remote_github",
    "ytdlp_useragent",
    "enable_file_logging",
    "max_log_sessions",
    "ytdlp",
    "player",
    "audio_exts",
    "video_exts",
    "playlist_exts",
    "mpv_args",
];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    #[default]
    Inf,
    Track,
    No,
    Count(u32),
}

impl std::fmt::Display for LoopMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inf => write!(f, "inf"),
            Self::Track => write!(f, "track"),
            Self::No => write!(f, "no"),
            Self::Count(n) => write!(f, "{}", n),
        }
    }
}

impl std::str::FromStr for LoopMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s_lower = s.trim().to_lowercase();
        match s_lower.as_str() {
            "inf" | "playlist" => Ok(Self::Inf),
            "track" | "file" => Ok(Self::Track),
            "no" | "off" | "false" => Ok(Self::No),
            other => {
                if let Ok(n) = other.parse::<u32>() {
                    Ok(Self::Count(n))
                } else {
                    Err(format!(
                        "Invalid loop_mode '{}'. Valid options: 'inf', 'track', 'no', or a number.",
                        other
                    ))
                }
            }
        }
    }
}

impl serde::Serialize for LoopMode {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub fn deserialize_loop_mode<'de, D>(deserializer: D) -> std::result::Result<LoopMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct LoopModeVisitor;

    impl<'de> serde::de::Visitor<'de> for LoopModeVisitor {
        type Value = LoopMode;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a loop mode string ('inf', 'track', 'no') or a count")
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<LoopMode, E>
        where
            E: serde::de::Error,
        {
            match value.parse::<LoopMode>() {
                Ok(mode) => Ok(mode),
                Err(_) => {
                    ui::warning(format!(
                        "Config: Invalid loop_mode '{}'. Defaulting to 'inf'.",
                        value
                    ));
                    Ok(LoopMode::Inf)
                }
            }
        }

        fn visit_i64<E>(self, value: i64) -> std::result::Result<LoopMode, E>
        where
            E: serde::de::Error,
        {
            if value >= 0 {
                Ok(LoopMode::Count(value as u32))
            } else {
                Ok(LoopMode::Inf)
            }
        }

        fn visit_u64<E>(self, value: u64) -> std::result::Result<LoopMode, E>
        where
            E: serde::de::Error,
        {
            Ok(LoopMode::Count(value as u32))
        }
    }

    deserializer.deserialize_any(LoopModeVisitor)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub shuffle: bool,
    #[serde(default, deserialize_with = "deserialize_loop_mode")]
    pub loop_mode: LoopMode,
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
    #[serde(default = "default_max_log_sessions")]
    pub max_log_sessions: usize,

    #[serde(default = "default_ytdlp")]
    pub ytdlp: String,

    #[serde(default = "default_player")]
    pub player: String,

    pub audio_exts: Vec<String>,
    pub video_exts: Vec<String>,
    pub playlist_exts: Vec<String>,

    #[serde(alias = "mpv_default_args", default)]
    pub mpv_args: Vec<String>,

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

        Self {
            shuffle: true,
            loop_mode: LoopMode::Inf,
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
            max_log_sessions: default_max_log_sessions(),
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
            mpv_args: Vec::new(),
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

    if content.contains("mpv_default_args") {
        log::info!("Detected legacy 'mpv_default_args' in config.toml; migrating to 'mpv_args'");
        let is_exact_legacy = cfg.mpv_args.len() == LEGACY_MPV_DEFAULT_ARGS.len()
            && cfg
                .mpv_args
                .iter()
                .zip(LEGACY_MPV_DEFAULT_ARGS.iter())
                .all(|(a, b)| a == *b);

        if is_exact_legacy {
            cfg.mpv_args.clear();
            warnings.push(
                "Migrated legacy mpv_default_args to built-in defaults (mpv_args = [])."
                    .to_string(),
            );
        } else {
            warnings.push("Migrated mpv_default_args key to mpv_args.".to_string());
        }
        needs_save = true;
    }

    if let Ok(table) = toml::from_str::<toml::Table>(&content) {
        let mut missing_keys = Vec::new();
        for &key in KNOWN_CONFIG_KEYS {
            if key == "mpv_args" && table.contains_key("mpv_default_args") {
                continue;
            }
            if !table.contains_key(key) {
                missing_keys.push(key);
            }
        }
        if !missing_keys.is_empty() {
            let is_verbose_or_debug = std::env::args().any(|a| {
                a == "-v" || a == "--verbose" || a.starts_with("-v") || a == "-d" || a == "--debug"
            });
            let missing_str = missing_keys.join(", ");
            if is_verbose_or_debug {
                ui::info(format!(
                    "Config: Auto-populated missing options with defaults: {}",
                    missing_str
                ));
            } else {
                log::info!(
                    "Config file missing keys: [{}]; auto-populating with defaults",
                    missing_str
                );
            }
            needs_save = true;
        }
    }

    if cfg.volume > 130 {
        warnings.push(format!(
            "Volume {} exceeds maximum (130). Reseting to 100.",
            cfg.volume
        ));
        cfg.volume = 100;
        needs_save = true;
    }

    if cfg.max_log_sessions == 0 {
        warnings.push(
            "max_log_sessions cannot be 0. Use 'enable_file_logging = false' to disable file logging. Resetting to 1."
                .to_string(),
        );
        cfg.max_log_sessions = 1;
        needs_save = true;
    }

    if cfg.music_dirs.is_empty() {
        warnings.push(
            "No music directories configured. Run 'mpv-music --manage-dirs' to add folders."
                .to_string(),
        );
    }

    for warning in warnings {
        ui::warning(format!("Config: {}", warning));
    }

    if needs_save
        && let Err(e) = save_to(&cfg, &config_path)
    {
        log::error!("Failed to save auto-corrected config: {}", e);
    }

    log::trace!("Loaded Config State: {:#?}", cfg);

    Ok(cfg)
}

pub fn save_to(config: &Config, config_path: &std::path::Path) -> Result<()> {
    log::info!("Saving configuration to {:?}", config_path);

    let toml_str = toml::to_string_pretty(config)?;
    std::fs::write(config_path, toml_str)?;

    log::debug!("Configuration saved successfully.");
    Ok(())
}

pub fn save(config: &Config) -> Result<()> {
    let dirs = ProjectDirs::from("com", "furqanhun", "mpv-music")
        .context("Could not determine config paths")?;
    let config_path = dirs.config_dir().join("config.toml");
    save_to(config, &config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let cfg = Config::default();
        assert_eq!(cfg.volume, 100);
        assert_eq!(cfg.loop_mode, LoopMode::Inf);
        assert!(cfg.shuffle);
        assert!(!cfg.video_ok);
        assert!(!cfg.watch);
        assert_eq!(cfg.max_log_sessions, 3);
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

        assert!(cfg.audio_exts.contains(&"mp3".to_string()));
        assert!(cfg.audio_exts.contains(&"flac".to_string()));
        assert!(cfg.audio_exts.contains(&"wav".to_string()));

        assert!(cfg.video_exts.contains(&"mp4".to_string()));
        assert!(cfg.video_exts.contains(&"mkv".to_string()));

        assert!(cfg.playlist_exts.contains(&"m3u".to_string()));
        assert!(cfg.playlist_exts.contains(&"m3u8".to_string()));
    }

    #[test]
    fn test_volume_cap_at_130() {
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
    fn test_max_log_sessions_clamped_at_1() {
        let mut max_sessions = 0_usize;
        if max_sessions == 0 {
            max_sessions = 1;
        }
        assert_eq!(max_sessions, 1);
    }

    #[test]
    fn test_loop_mode_validation_valid() {
        assert!("inf".parse::<LoopMode>().is_ok());
        assert!("playlist".parse::<LoopMode>().is_ok());
        assert!("track".parse::<LoopMode>().is_ok());
        assert!("file".parse::<LoopMode>().is_ok());
        assert!("no".parse::<LoopMode>().is_ok());
        assert!("off".parse::<LoopMode>().is_ok());
        assert!("false".parse::<LoopMode>().is_ok());
    }

    #[test]
    fn test_loop_mode_validation_invalid() {
        assert!("potato".parse::<LoopMode>().is_err());
        assert!("".parse::<LoopMode>().is_err());
        assert!("random_mode".parse::<LoopMode>().is_err());
    }

    #[test]
    fn test_loop_mode_validation_numeric() {
        assert_eq!("5".parse::<LoopMode>().unwrap(), LoopMode::Count(5));
    }

    #[test]
    fn test_loop_mode_validation_numeric_multiple_digits() {
        assert_eq!("999".parse::<LoopMode>().unwrap(), LoopMode::Count(999));
    }

    #[test]
    fn test_loop_mode_parsing() {
        assert_eq!("inf".parse::<LoopMode>().unwrap(), LoopMode::Inf);
        assert_eq!("playlist".parse::<LoopMode>().unwrap(), LoopMode::Inf);
        assert_eq!("track".parse::<LoopMode>().unwrap(), LoopMode::Track);
        assert_eq!("file".parse::<LoopMode>().unwrap(), LoopMode::Track);
        assert_eq!("no".parse::<LoopMode>().unwrap(), LoopMode::No);
        assert_eq!("off".parse::<LoopMode>().unwrap(), LoopMode::No);
        assert_eq!("false".parse::<LoopMode>().unwrap(), LoopMode::No);
        assert_eq!("5".parse::<LoopMode>().unwrap(), LoopMode::Count(5));
        assert_eq!("42".parse::<LoopMode>().unwrap(), LoopMode::Count(42));
    }

    #[test]
    fn test_loop_mode_display() {
        assert_eq!(LoopMode::Inf.to_string(), "inf");
        assert_eq!(LoopMode::Track.to_string(), "track");
        assert_eq!(LoopMode::No.to_string(), "no");
        assert_eq!(LoopMode::Count(7).to_string(), "7");
    }

    #[test]
    fn test_ytdlp_flags_default() {
        let cfg = Config::default();
        assert!(!cfg.ytdlp_available);
        assert!(!cfg.ytdlp_is_nightly);
    }

    #[test]
    fn test_mpv_args_default() {
        let cfg = Config::default();
        assert!(cfg.mpv_args.is_empty());
    }

    #[test]
    fn test_missing_keys_detection() {
        let partial_toml = "volume = 60\nshuffle = true\n";
        let table: toml::Table = toml::from_str(partial_toml).unwrap();
        let missing: Vec<&str> = KNOWN_CONFIG_KEYS
            .iter()
            .copied()
            .filter(|&k| !table.contains_key(k))
            .collect();
        assert!(missing.contains(&"player"));
        assert!(missing.contains(&"ytdlp"));
        assert!(missing.contains(&"mpv_args"));
        assert!(!missing.contains(&"volume"));
        assert!(!missing.contains(&"shuffle"));
    }

    #[test]
    fn test_mpv_default_args_alias() {
        let default_cfg = Config::default();
        let toml_str = toml::to_string_pretty(&default_cfg).unwrap();
        let legacy_toml = toml_str.replace(
            "mpv_args = []",
            "mpv_default_args = [\"--gapless-audio=yes\"]",
        );
        let cfg: Config = toml::from_str(&legacy_toml).unwrap();
        assert_eq!(cfg.mpv_args, vec!["--gapless-audio=yes"]);
    }

    #[test]
    fn test_legacy_mpv_default_args_migration() {
        let default_cfg = Config::default();
        let toml_str = toml::to_string_pretty(&default_cfg).unwrap();
        let legacy_array_str = serde_json::to_string(&LEGACY_MPV_DEFAULT_ARGS).unwrap();
        let legacy_toml = toml_str.replace(
            "mpv_args = []",
            &format!("mpv_default_args = {}", legacy_array_str),
        );

        let mut cfg: Config = toml::from_str(&legacy_toml).unwrap();
        assert_eq!(cfg.mpv_args.len(), LEGACY_MPV_DEFAULT_ARGS.len());

        if legacy_toml.contains("mpv_default_args") {
            let is_exact = cfg.mpv_args.len() == LEGACY_MPV_DEFAULT_ARGS.len()
                && cfg
                    .mpv_args
                    .iter()
                    .zip(LEGACY_MPV_DEFAULT_ARGS.iter())
                    .all(|(a, b)| a == *b);
            if is_exact {
                cfg.mpv_args.clear();
            }
        }
        assert!(cfg.mpv_args.is_empty());
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

        let custom_cfg = Config {
            player: "mpvnet".to_string(),
            ..Default::default()
        };
        assert_eq!(custom_cfg.player_bin(), "mpvnet");

        let custom_path = Config {
            player: "/usr/local/bin/my-mpv".to_string(),
            ..Default::default()
        };
        assert_eq!(custom_path.player_bin(), "/usr/local/bin/my-mpv");

        let custom_exe = Config {
            player: "mpv.exe".to_string(),
            ..Default::default()
        };
        assert_eq!(custom_exe.player_bin(), "mpv.exe");
    }

    #[test]
    fn test_ytdlp_configuration() {
        let default_cfg = Config::default();
        assert_eq!(default_cfg.ytdlp, "yt-dlp");
        assert_eq!(default_cfg.ytdlp_bin(), "yt-dlp");

        let custom_cfg = Config {
            ytdlp: "yt-dlp-nightly".to_string(),
            ..Default::default()
        };
        assert_eq!(custom_cfg.ytdlp_bin(), "yt-dlp-nightly");

        let custom_path = Config {
            ytdlp: "/usr/local/bin/yt-dlp".to_string(),
            ..Default::default()
        };
        assert_eq!(custom_path.ytdlp_bin(), "/usr/local/bin/yt-dlp");
    }

    #[test]
    fn test_load_autopopulates_missing_keys() {
        let temp_dir = std::env::temp_dir().join(format!(
            "mpv_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let config_file = temp_dir.join("config.toml");

        std::fs::write(&config_file, "volume = 77\nshuffle = false\n").unwrap();

        let loaded = load(Some(config_file.clone())).unwrap();
        assert_eq!(loaded.volume, 77);
        assert!(!loaded.shuffle);
        assert_eq!(loaded.player, "mpv");
        assert_eq!(loaded.ytdlp, "yt-dlp");

        let on_disk = std::fs::read_to_string(&config_file).unwrap();
        assert!(on_disk.contains("volume = 77"));
        assert!(on_disk.contains("player = \"mpv\""));
        assert!(on_disk.contains("ytdlp = \"yt-dlp\""));
        assert!(on_disk.contains("mpv_args = []"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
