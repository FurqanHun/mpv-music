use clap::Parser;

#[derive(Parser, Clone, Debug)]
#[command(
    name = "mpv-music",
    author,
    version,
    about = "mpv-music - A TUI-based music player wrapper for MPV",
    after_help = "In interactive menus: use arrow keys/typing to filter, ENTER to select, ESC to go back.",
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[arg(index = 1, help = "Directly play a file, directory, or URL")]
    pub target: Option<String>,

    #[arg(
        short = 'r',
        long,
        help = "Update index (incremental scan). Detects new/changed files."
    )]
    pub refresh_index: bool,

    #[arg(long, help = "Force a full re-scan of the library.")]
    pub reindex: bool,

    #[cfg(feature = "update")]
    #[arg(short = 'u', long, help = "Update the application")]
    pub update: bool,

    #[cfg(feature = "update")]
    #[arg(short = 'y', long, help = "Auto-confirm the update prompt")]
    pub yes: bool,

    #[arg(
        long,
        num_args = 1..,
        value_name = "PATH",
        help = "Add directory (e.g. --add-dir /music /other)"
    )]
    pub add_dir: Option<Vec<String>>,

    #[arg(
        long,
        num_args = 1..,
        value_name = "PATH",
        visible_alias = "rm-dir",
        help = "Remove directory"
    )]
    pub remove_dir: Option<Vec<String>>,

    #[arg(long, help = "Open the Interactive Directory Manager")]
    pub manage_dirs: bool,

    #[arg(
        short = 'c',
        long,
        value_name = "EDITOR",
        num_args = 0..=1,
        help = "Edit config file"
    )]
    pub config: Option<Option<String>>,

    #[arg(long, visible_alias = "rm-conf", help = "Delete config file (Reset)")]
    pub remove_config: bool,

    #[arg(
        long,
        value_name = "PAGER",
        num_args = 0..=1,
        help = "View logs"
    )]
    pub log: Option<Option<String>>,

    #[arg(
        long,
        visible_alias = "rm-log",
        value_name = "COUNT",
        num_args = 0..=1,
        help = "Delete log files (deletes all if no count given, or oldest N logs)"
    )]
    pub remove_log: Option<Option<usize>>,

    #[arg(short = 'p', long, help = "Play all tracks immediately")]
    pub play_all: bool,

    #[arg(
            short = 'l',
            long,
            num_args = 0..=1,
            help = "Open Playlist Mode. Opens picker if no value given."
        )]
    pub playlist: Option<Option<String>>,

    #[arg(long, help = "Allow video files")]
    pub video_ok: bool,

    #[arg(
        long,
        help = "Force disable video files (overrides config/negate --video-ok)"
    )]
    pub no_video: bool,

    #[arg(short = 'w', long, help = "Play with video window enabled")]
    pub watch: bool,

    #[arg(long, help = "Force audio-only (overrides config if watch=true)")]
    pub no_watch: bool,

    #[arg(
        long,
        value_name = "BIN",
        help = "Specify the player binary (defaults to 'mpv')"
    )]
    pub player: Option<String>,

    #[arg(
        long,
        value_name = "BIN",
        help = "Specify the yt-dlp binary (defaults to 'yt-dlp')"
    )]
    pub ytdlp: Option<String>,

    #[arg(
            long = "loop",
            num_args = 0..=1,
            default_missing_value = "inf",
            help = "Enable looping ('inf', 'no', 'track', or a NUMBER)"
        )]
    pub loop_arg: Option<String>,

    #[arg(long, help = "Disable all looping")]
    pub no_loop: bool,

    #[arg(long, help = "Loop the current track (Repeat One)")]
    pub repeat: bool,

    #[arg(
        short = 'e',
        long,
        value_name = "EXT1,EXT2",
        help = "Override allowed extensions"
    )]
    pub ext: Option<String>,

    #[arg(
        short = 'g',
        long,
        num_args = 0..=1,
        help = "Filter by Genre (e.g. -g 'Pop,Rock')"
    )]
    pub genre: Option<Option<String>>,

    #[arg(
        short = 'a',
        long,
        num_args = 0..=1,
        help = "Filter by Artist (e.g. -a 'ado,gentle')"
    )]
    pub artist: Option<Option<String>>,

    #[arg(
        short = 'b',
        long,
        num_args = 0..=1,
        help = "Filter by Album"
    )]
    pub album: Option<Option<String>>,

    #[arg(
            short = 't',
            long,
            num_args = 0..=1,
            help = "Filter by Title (Partial). Opens Track Mode if no value given."
        )]
    pub title: Option<Option<String>>,

    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Display Verbose Information")]
    pub verbose: u8,
    #[arg(short = 'd', long, help = "Debug mode")]
    pub debug: bool,
    #[arg(long, help = "Set volume (0-100)")]
    pub volume: Option<u8>,
    #[arg(short = 's', long, help = "Shuffle")]
    pub shuffle: bool,
    #[arg(long, help = "No Shuffle")]
    pub no_shuffle: bool,
    #[arg(long, help = "Force serial (single-threaded) processing")]
    pub serial: bool,
    #[arg(
            long,
            visible_alias = "yt",
            num_args = 0..=1,
            help = "Search YouTube directly (e.g. --yt 'lofi') Requires yt-dlp."
        )]
    pub search: Option<Option<String>>,
    #[arg(
        long,
        num_args = 0..=1,
        help = "Open Radio Mode directly or play a specific station (e.g. jpop, vocaloid)"
    )]
    pub radio: Option<Option<String>>,
    #[arg(long, allow_hyphen_values = true, num_args = 1.., help = "Pass arguments to mpv")]
    pub mpv_args: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_flags_defaults() {
        let args = Cli::parse_from(["mpv-music"]);
        assert!(args.target.is_none());
        assert!(!args.play_all);
        assert!(!args.debug);
        assert_eq!(args.verbose, 0);
        assert!(!args.serial);
        assert!(!args.video_ok);
        assert!(!args.watch);
        assert!(args.remove_log.is_none());
        assert!(args.log.is_none());
    }

    #[test]
    fn test_cli_positional_target() {
        let args = Cli::parse_from(["mpv-music", "https://youtube.com/watch?v=123"]);
        assert_eq!(
            args.target.as_deref(),
            Some("https://youtube.com/watch?v=123")
        );

        let file_args = Cli::parse_from(["mpv-music", "/path/to/song.flac"]);
        assert_eq!(file_args.target.as_deref(), Some("/path/to/song.flac"));
    }

    #[test]
    fn test_cli_aliases() {
        let rm_log = Cli::parse_from(["mpv-music", "--rm-log"]);
        assert!(rm_log.remove_log.is_some());

        let rm_dir = Cli::parse_from(["mpv-music", "--rm-dir", "/path"]);
        assert_eq!(rm_dir.remove_dir, Some(vec!["/path".to_string()]));

        let rm_conf = Cli::parse_from(["mpv-music", "--rm-conf"]);
        assert!(rm_conf.remove_config);

        let yt_search = Cli::parse_from(["mpv-music", "--yt", "lofi"]);
        assert_eq!(yt_search.search, Some(Some("lofi".to_string())));
    }

    #[test]
    fn test_cli_remove_log_options() {
        let rm_all = Cli::parse_from(["mpv-music", "--remove-log"]);
        assert_eq!(rm_all.remove_log, Some(None));

        let rm_count = Cli::parse_from(["mpv-music", "--remove-log", "5"]);
        assert_eq!(rm_count.remove_log, Some(Some(5)));
    }

    #[test]
    fn test_cli_filter_args_with_and_without_value() {
        let g_empty = Cli::parse_from(["mpv-music", "-g"]);
        assert_eq!(g_empty.genre, Some(None));

        let g_val = Cli::parse_from(["mpv-music", "-g", "Rock,Pop"]);
        assert_eq!(g_val.genre, Some(Some("Rock,Pop".to_string())));

        let a_val = Cli::parse_from(["mpv-music", "-a", "Queen"]);
        assert_eq!(a_val.artist, Some(Some("Queen".to_string())));

        let b_val = Cli::parse_from(["mpv-music", "-b", "Greatest Hits"]);
        assert_eq!(b_val.album, Some(Some("Greatest Hits".to_string())));

        let t_val = Cli::parse_from(["mpv-music", "-t", "Bohemian"]);
        assert_eq!(t_val.title, Some(Some("Bohemian".to_string())));
    }

    #[test]
    fn test_cli_flags_toggles() {
        let args = Cli::parse_from([
            "mpv-music",
            "-p",
            "-s",
            "-w",
            "--video-ok",
            "--serial",
            "-vv",
            "--volume",
            "90",
            "--repeat",
        ]);

        assert!(args.play_all);
        assert!(args.shuffle);
        assert!(args.watch);
        assert!(args.video_ok);
        assert!(args.serial);
        assert_eq!(args.verbose, 2);
        assert_eq!(args.volume, Some(90));
        assert!(args.repeat);
    }
}
