use clap::Parser;

#[derive(Parser, Clone, Debug)]
#[command(
    name = "mpv-music",
    author,
    version,
    about = "mpv-music - A TUI-based music player wrapper for MPV",
    after_help = r#"
SEARCH & OPERATORS:
  Global Search:      mpv-music "dean july"                (unified search across title, artist, album)
  OR Union:           mpv-music "natori || ado"            (aliases: ||, OR, |, comma)
  AND Intersection:   mpv-music "ado && miku"              (aliases: &&, AND, space, semicolon)
  Flag OR Aliases:    mpv-music -a "ado || eve" -g "rock, pop"
  Multi-Flag Union:   mpv-music -a ado -g rock --or        (pools tracks matching either flag)

INTERACTIVE TUI:
  Navigation:         Type to filter, Up/Down arrow keys or Ctrl+N/Ctrl+P to navigate
  Multi-Select:       TAB toggle, Ctrl+A select all, Ctrl+D deselect matching, Alt+D deselect all
  Playback:           ENTER to confirm & play, ESC to go back/exit"#,
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[arg(
        index = 1,
        help = "Search library across Title/Artist/Album (e.g. 'dean july', 'ado || natori') or directly play a file, directory, or URL"
    )]
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
        help = "Open Playlist Mode. Opens picker if empty or filter query if given."
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
        help = "Specify the player binary (defaults to 'mpv', compatible forks supported)"
    )]
    pub player: Option<String>,

    #[arg(
        long,
        value_name = "BIN",
        help = "Specify the yt-dlp binary (defaults to 'yt-dlp', compatible forks supported)"
    )]
    pub ytdlp: Option<String>,

    #[arg(
        long = "loop",
        num_args = 0..=1,
        default_missing_value = "inf",
        help = "Enable looping ('inf'/'playlist'/'all', 'track'/'file'/'one', 'no'/'off'/false, or a NUMBER)"
    )]
    pub loop_arg: Option<String>,

    #[arg(long, help = "Disable all looping")]
    pub no_loop: bool,

    #[arg(long, help = "Loop the current track (Repeat One)")]
    pub repeat: bool,

    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        help = "Enable gapless playback ('default'/'auto'/'weak', true/'yes'/'on', false/'no'/'off'/'none')"
    )]
    pub gapless: Option<String>,

    #[arg(long, help = "Disable gapless playback")]
    pub no_gapless: bool,

    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "default",
        help = "Set ReplayGain mode ('default'/'auto', 'album', 'track', 'no'/'off'/'none'/false)"
    )]
    pub replaygain: Option<String>,

    #[arg(long, help = "Disable ReplayGain volume normalization")]
    pub no_replaygain: bool,

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
        help = "Filter by Genre (supports OR aliases like 'pop, rock' or 'pop || rock'). Opens picker if empty."
    )]
    pub genre: Option<Option<String>>,

    #[arg(
        short = 'a',
        long,
        num_args = 0..=1,
        help = "Filter by Artist (supports OR aliases like 'ado, natori' or 'ado || natori'). Opens picker if empty."
    )]
    pub artist: Option<Option<String>>,

    #[arg(
        short = 'b',
        long,
        num_args = 0..=1,
        help = "Filter by Album (supports OR aliases like 'album a || album b'). Opens picker if empty."
    )]
    pub album: Option<Option<String>>,

    #[arg(
        short = 't',
        long,
        num_args = 0..=1,
        help = "Filter by Title (supports OR aliases like 'song a || song b'). Opens Track Mode if empty."
    )]
    pub title: Option<Option<String>>,
    #[arg(
        long,
        help = "Combine multiple filter flags via OR (Union) instead of default AND (Intersection)"
    )]
    pub or: bool,

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
        help = "Search YouTube directly for tracks & live streams (e.g. --yt 'lofi'). Requires yt-dlp."
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
