use crate::cli::Cli;
use crate::config::Config;
use anyhow::Result;
use flexi_logger::{FileSpec, Logger, LoggerHandle, WriteMode, style};
use std::path::Path;

pub fn init(args: &Cli, cfg: &Config, log_dir: &Path) -> Result<LoggerHandle> {
    let log_filter = if cfg.enable_file_logging {
        if args.debug {
            "mpv_music=debug, warn"
        } else {
            "mpv_music=info, warn"
        }
    } else if args.debug {
        "mpv_music=debug, warn"
    } else if args.verbose > 0 {
        "mpv_music=info, warn"
    } else {
        "mpv_music=error, warn"
    };

    std::fs::create_dir_all(log_dir)?;
    let mut logger = Logger::try_with_str(log_filter)?.format_for_stderr(|w, _now, record| {
        let level = record.level();
        write!(
            w,
            "[{}] {}",
            style(level).paint(level.as_str()),
            record.args()
        )
    });

    if cfg.enable_file_logging {
        let log_path = log_dir.join("mpv-music.log");
        if log_path.exists() {
            let _ = std::fs::remove_file(&log_path);
        }

        logger = logger
            .log_to_file(
                FileSpec::default()
                    .directory(log_dir)
                    .basename("mpv-music")
                    .suffix("log")
                    .use_timestamp(false),
            )
            .format_for_files(flexi_logger::opt_format)
            .write_mode(WriteMode::Direct);
    }

    let is_interactive = args.target.is_none() || args.refresh_index;
    if args.debug {
        logger = logger.duplicate_to_stderr(flexi_logger::Duplicate::All);
    } else if args.verbose > 0 {
        logger = logger.duplicate_to_stderr(flexi_logger::Duplicate::Info);
    } else if is_interactive {
        logger = logger.duplicate_to_stderr(flexi_logger::Duplicate::Error);
    }

    let handle = logger.start()?;
    Ok(handle)
}
