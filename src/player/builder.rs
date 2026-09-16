use crate::config::{Config, LoopMode, NerdFontMode};
use crate::tui::Icons;
use std::process::Command;

use super::radio::{
    apply_radio_args, generate_ipc_socket, is_radio_station, spawn_radio_sync_if_needed,
};
use super::target::classify_target;
use super::ytdlp::{check_deno_availability, has_command};

pub const DEFAULT_BANNER_TEXT: &str = " ╔══  MPV-MUSIC  ══╗";

pub fn default_banner_text(nerd_fonts: NerdFontMode) -> String {
    let icons = Icons::new(nerd_fonts);
    match nerd_fonts {
        NerdFontMode::None => DEFAULT_BANNER_TEXT.to_string(),
        _ => format!(
            " \u{2500}\u{2500} {} MPV-MUSIC \u{2500}\u{2500}",
            icons.track()
        ),
    }
}

pub fn default_status_msg(nerd_fonts: NerdFontMode) -> String {
    let icons = Icons::new(nerd_fonts);
    format!(
        " {} ${{?metadata/artist:${{metadata/artist}} - }}${{?metadata/title:${{metadata/title}}}}${{!metadata/title:${{media-title}}}} • ${{time-pos}} / ${{duration}} • (${{percent-pos}}%)",
        icons.play()
    )
}

#[derive(Debug)]
pub struct MpvCommandBuilder<'a> {
    config: &'a Config,
    extra_args: &'a [String],
    target: Option<&'a str>,
    playlist_path: Option<String>,
    optimization_target: Option<String>,
    ipc_socket: Option<String>,
}

impl<'a> MpvCommandBuilder<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            extra_args: &[],
            target: None,
            playlist_path: None,
            optimization_target: None,
            ipc_socket: None,
        }
    }

    pub fn target(mut self, target: &'a str) -> Self {
        self.target = Some(target);
        self
    }

    pub fn playlist(mut self, path: impl Into<String>) -> Self {
        self.playlist_path = Some(path.into());
        self
    }

    pub fn optimization_target(mut self, opt_target: String) -> Self {
        self.optimization_target = Some(opt_target);
        self
    }

    pub fn extra_args(mut self, args: &'a [String]) -> Self {
        self.extra_args = args;
        self
    }

    pub fn with_radio_sync(mut self) -> Self {
        if let Some(target) = self.target
            && is_radio_station(target)
        {
            self.ipc_socket = Some(generate_ipc_socket());
        }
        self
    }

    pub fn ipc_socket(&self) -> Option<&str> {
        self.ipc_socket.as_deref()
    }

    pub fn build(self) -> Command {
        let cmd_name = self.config.player_bin();
        let mut cmd = Command::new(cmd_name);

        apply_common_args(&mut cmd, self.config, self.extra_args);

        let opt_target = self.optimization_target.as_deref().or(self.target);
        if let Some(opt) = opt_target {
            apply_url_optimizations(&mut cmd, opt, self.config);
        }

        if let Some(ref socket) = self.ipc_socket {
            let target = self.target.unwrap_or_default();
            apply_radio_args(&mut cmd, target, socket, self.config);
            spawn_radio_sync_if_needed(target, socket);
        }

        if let Some(playlist) = self.playlist_path {
            cmd.arg(format!("--playlist={}", playlist));
        } else if let Some(target) = self.target {
            cmd.arg(target);
        }

        cmd
    }
}

pub fn apply_common_args(cmd: &mut Command, config: &Config, extra_args: &[String]) {
    log::debug!("Applying common MPV arguments from config");

    if config.watch {
        log::debug!("Visual mode enabled (--watch)");
        cmd.arg("--force-window=immediate");
        cmd.arg("--video=auto");
    } else {
        log::debug!("Audio-only mode (forcing video=no)");
        cmd.arg("--force-window=no");
        cmd.arg("--video=no");
        cmd.arg("--audio-display=no");
    }

    cmd.arg("--msg-level=cplayer=warn");
    cmd.arg("--display-tags=");
    cmd.arg("--no-term-osd-bar");

    let banner = default_banner_text(config.nerd_fonts);
    let is_debug = log::max_level() >= log::LevelFilter::Debug;
    if is_debug {
        log::debug!("Skipping screen clear to preserve logs");
        cmd.arg(format!("--term-playing-msg=\n{}\n", banner));
    } else {
        log::debug!("Injecting ANSI clear codes into banner");
        cmd.arg(format!(
            "--term-playing-msg=\x1b[H\x1b[2J\x1b[3J\n{}\n",
            banner
        ));
    }
    cmd.arg(format!(
        "--term-status-msg={}",
        default_status_msg(config.nerd_fonts)
    ));

    for arg in &config.mpv_args {
        if config.watch
            && (arg == "--no-video" || arg == "--video=no" || arg == "--audio-display=no")
        {
            log::debug!("Skipping '{}' because visual mode is active", arg);
            continue;
        }

        if arg.contains("--term-playing-msg=") {
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            if parts.len() == 2 {
                let banner_text = parts[1];
                if is_debug {
                    log::debug!("Skipping screen clear to preserve logs");
                    cmd.arg(format!("--term-playing-msg=\n{}\n", banner_text.trim()));
                } else {
                    log::debug!("Injecting ANSI clear codes into banner");
                    cmd.arg(format!(
                        "--term-playing-msg=\x1b[H\x1b[2J\x1b[3J\n{}\n",
                        banner_text.trim()
                    ));
                }
                continue;
            }
        }
        cmd.arg(arg);
    }

    log::debug!("Setting volume: {}", config.volume);
    cmd.arg(format!("--volume={}", config.volume));

    if config.shuffle {
        log::debug!("Shuffle enabled");
        cmd.arg("--shuffle");
    }

    log::debug!("Setting loop mode: {}", config.loop_mode);
    match config.loop_mode {
        LoopMode::Inf => {
            cmd.arg("--loop-playlist=inf");
        }
        LoopMode::Track => {
            cmd.arg("--loop-file=inf");
        }
        LoopMode::No => {
            cmd.arg("--loop-playlist=no");
            cmd.arg("--loop-file=no");
        }
        LoopMode::Count(n) => {
            cmd.arg(format!("--loop-playlist={}", n));
        }
    }

    if !extra_args.is_empty() {
        log::debug!("Injecting manual CLI overrides: {:?}", extra_args);
        for arg in extra_args {
            cmd.arg(arg);
        }
    }
}

pub fn apply_url_optimizations(cmd: &mut Command, target: &str, config: &Config) {
    let kind = classify_target(target);
    let is_youtube = kind.is_youtube();
    let is_url = kind.is_network();

    if is_url {
        log::debug!("Applying network stream optimizations");

        if target.contains("listen.moe") {
            log::debug!("LISTEN.moe detected: Forcing Live Mode (Cache=No, 1M Buffer)");
            cmd.arg("--cache=no");
            cmd.arg("--demuxer-max-bytes=1M");
            cmd.arg("--demuxer-max-back-bytes=0");
        }

        cmd.arg("--msg-level=ytdl_hook=info");
        let ytdlp_bin = config.ytdlp_bin();
        if ytdlp_bin != "yt-dlp" {
            log::debug!("Configuring custom yt-dlp binary for player: {}", ytdlp_bin);
            cmd.arg(format!(
                "--script-opts-append=ytdl_hook-ytdl_path={}",
                ytdlp_bin
            ));
        }

        if is_youtube {
            if !config.video_ok && !config.watch {
                log::debug!("YouTube detected & Audio Mode: forcing bestaudio format");
                cmd.arg("--ytdl-format=bestaudio/best");
            } else {
                log::debug!("YouTube detected & Video/Watch Mode: allowing default formats");
            }
        }

        let mut ytdl_opts = String::new();

        if target.contains("list=") {
            log::debug!("Playlist detected in URL, forcing yes-playlist");
            ytdl_opts.push_str("yes-playlist=,");
        }

        if is_youtube {
            log::debug!("Applying User-Agent from config");
            let ua = if config.ytdlp_useragent == "default" || config.ytdlp_useragent.is_empty() {
                crate::config::DEFAULT_YTDLP_USER_AGENT
            } else {
                &config.ytdlp_useragent
            };
            ytdl_opts.push_str(&format!("user-agent={},", ua));

            if config.ytdlp_ejs_remote_github && !config.ytdlp_is_nightly && ytdlp_bin == "yt-dlp" {
                log::debug!("Enabling remote EJS components");
                ytdl_opts.push_str("remote-components=ejs:github,");
            }

            if check_deno_availability(ytdlp_bin) {
                log::debug!("JS runtime check: Deno found (skipping fallbacks)");
            } else if has_command("node") {
                log::debug!("JS runtime check: node found");
                ytdl_opts.push_str("js-runtimes=node,");
            } else if has_command("qjs") || has_command("quickjs") {
                log::debug!("JS runtime check: quickjs found");
                ytdl_opts.push_str("js-runtimes=quickjs,");
            } else if has_command("bun") {
                log::debug!("JS runtime check: bun found");
                ytdl_opts.push_str("js-runtimes=bun,");
            } else {
                log::warn!("No JS runtime found (Deno/Node). YouTube playback may fail with 403.");
            }
        }

        if !ytdl_opts.is_empty() {
            let clean_opts = ytdl_opts.trim_end_matches(',');
            log::debug!("Applying ytdl-raw-options: {}", clean_opts);
            cmd.arg(format!("--ytdl-raw-options={}", clean_opts));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_url_optimizations_custom_ytdlp() {
        let config = Config {
            ytdlp: "custom-dlp".to_string(),
            ..Default::default()
        };
        let mut cmd = Command::new("mpv");
        apply_url_optimizations(&mut cmd, "https://youtube.com/watch?v=123", &config);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(
            args.iter()
                .any(|a| a == "--script-opts-append=ytdl_hook-ytdl_path=custom-dlp")
        );
    }

    #[test]
    fn test_apply_url_optimizations_default_ytdlp() {
        let config = Config::default();
        let mut cmd = Command::new("mpv");
        apply_url_optimizations(&mut cmd, "https://youtube.com/watch?v=123", &config);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(!args.iter().any(|a| a.contains("ytdl_hook-ytdl_path")));
    }

    #[test]
    fn test_apply_url_optimizations_custom_ytdlp_skips_remote_ejs() {
        let config = Config {
            ytdlp: "custom-fork".to_string(),
            ytdlp_ejs_remote_github: true,
            ..Default::default()
        };
        let mut cmd = Command::new("mpv");
        apply_url_optimizations(&mut cmd, "https://youtube.com/watch?v=123", &config);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(
            !args
                .iter()
                .any(|a| a.contains("remote-components=ejs:github"))
        );
    }

    #[test]
    fn test_apply_common_args_defaults() {
        let config = Config::default();
        let mut cmd = Command::new("mpv");
        apply_common_args(&mut cmd, &config, &[]);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.iter().any(|a| a == "--msg-level=cplayer=warn"));
        assert!(args.iter().any(|a| a == "--no-term-osd-bar"));
        assert!(args.iter().any(|a| a.contains("MPV-MUSIC")));
        assert!(args.iter().any(|a| a.contains("term-status-msg")));
    }

    #[test]
    fn test_apply_common_args_custom_mpv_args() {
        let config = Config {
            mpv_args: vec!["--gapless-audio=yes".to_string()],
            ..Default::default()
        };
        let mut cmd = Command::new("mpv");
        apply_common_args(&mut cmd, &config, &[]);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.iter().any(|a| a == "--gapless-audio=yes"));
    }

    #[test]
    fn test_apply_common_args_nerd_fonts() {
        let config_none = Config {
            nerd_fonts: NerdFontMode::None,
            ..Default::default()
        };
        let mut cmd_none = Command::new("mpv");
        apply_common_args(&mut cmd_none, &config_none, &[]);
        let args_none: Vec<String> = cmd_none
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        let status_none = args_none
            .iter()
            .find(|a| a.starts_with("--term-status-msg="))
            .unwrap();
        assert!(status_none.contains("▶"));
        let banner_none = args_none
            .iter()
            .find(|a| a.starts_with("--term-playing-msg="))
            .unwrap();
        assert!(banner_none.contains("╔══  MPV-MUSIC  ══╗"));

        let config_mono = Config {
            nerd_fonts: NerdFontMode::Mono,
            ..Default::default()
        };
        let mut cmd_mono = Command::new("mpv");
        apply_common_args(&mut cmd_mono, &config_mono, &[]);
        let args_mono: Vec<String> = cmd_mono
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        let status_mono = args_mono
            .iter()
            .find(|a| a.starts_with("--term-status-msg="))
            .unwrap();
        assert!(status_mono.contains("\u{f04b}"));
        let banner_mono = args_mono
            .iter()
            .find(|a| a.starts_with("--term-playing-msg="))
            .unwrap();
        assert!(banner_mono.contains("── \u{f001} MPV-MUSIC ──"));
    }

    #[test]
    fn test_builder_local_file() {
        let config = Config::default();
        let cmd = MpvCommandBuilder::new(&config)
            .target("/home/user/music/song.flac")
            .build();

        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();

        assert_eq!(cmd.get_program(), "mpv");
        assert_eq!(args.last().unwrap(), "/home/user/music/song.flac");
        assert!(args.iter().any(|a| a == "--video=no"));
        assert!(args.iter().any(|a| a == "--loop-playlist=inf"));
    }

    #[test]
    fn test_builder_playlist_and_extra_args() {
        let config = Config::default();
        let extra = vec!["--start=30".to_string(), "--speed=1.2".to_string()];
        let cmd = MpvCommandBuilder::new(&config)
            .playlist("/tmp/queue.m3u8")
            .extra_args(&extra)
            .build();

        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();

        assert!(args.iter().any(|a| a == "--playlist=/tmp/queue.m3u8"));
        assert!(args.iter().any(|a| a == "--start=30"));
        assert!(args.iter().any(|a| a == "--speed=1.2"));
    }

    #[test]
    fn test_builder_youtube_audio_mode() {
        let config = Config::default();
        let cmd = MpvCommandBuilder::new(&config)
            .target("https://youtube.com/watch?v=123")
            .build();

        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();

        assert!(args.iter().any(|a| a == "--ytdl-format=bestaudio/best"));
        assert_eq!(args.last().unwrap(), "https://youtube.com/watch?v=123");
    }
}
