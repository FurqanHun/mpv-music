use clap::Parser;

#[derive(Parser, Clone, Debug)]
#[command(
    name = "mpv-music",
    author,
    version,
    about = "mpv-music - A TUI-based music player wrapper for MPV",
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[arg(index = 1, help = "Directly play a file, directory, or URL")]
    pub target: Option<String>,

    // indexing
    #[arg(
        short = 'r',
        long,
        help = "Update index (incremental scan). Detects new/changed files."
    )]
    pub refresh_index: bool,

    #[arg(long, help = "Force a full re-scan of the library.")]
    pub reindex: bool,

    // actions
    #[cfg(feature = "update")]
    #[arg(short = 'u', long, help = "Update the application")]
    pub update: bool,

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

    // conf/log
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

    #[arg(long, visible_alias = "rm-log", help = "Delete log file")]
    pub remove_log: bool,

    // playback
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

    // filters (comma supported)
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

    // sys
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
        default_missing_value = "jpop",
        num_args = 0..=1,
        help = "Play listen.moe radio station (jpop or kpop)"
    )]
    pub radio: Option<String>,
}
