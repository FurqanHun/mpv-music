use crate::cli::Cli;
use crate::config::Config;
use anyhow::Result;
use flexi_logger::{FileSpec, Logger, LoggerHandle, WriteMode, style};
use std::path::{Path, PathBuf};

pub fn logs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("logs")
}

pub fn list_log_files(data_dir: &Path) -> Vec<PathBuf> {
    let dir = logs_dir(data_dir);
    let mut files = Vec::new();

    let legacy = data_dir.join("mpv-music.log");
    if legacy.exists() {
        files.push(legacy);
    }

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().is_some_and(|e| e == "log") {
                files.push(p);
            }
        }
    }

    files.sort_by(|a, b| {
        let time_a = a.metadata().and_then(|m| m.modified()).ok();
        let time_b = b.metadata().and_then(|m| m.modified()).ok();
        match (time_a, time_b) {
            (Some(ta), Some(tb)) => tb.cmp(&ta),
            _ => b.file_name().cmp(&a.file_name()),
        }
    });

    files
}

pub fn prune_old_logs(data_dir: &Path, keep_count: usize) -> Result<usize> {
    let files = list_log_files(data_dir);
    let mut removed = 0;

    if files.len() > keep_count {
        for file in &files[keep_count..] {
            if std::fs::remove_file(file).is_ok() {
                removed += 1;
            }
        }
    }

    Ok(removed)
}

pub fn format_log_display(path: &Path, is_latest: bool) -> String {
    let metadata = path.metadata().ok();
    let size_str = match metadata.as_ref().map(|m| m.len()) {
        Some(bytes) if bytes >= 1_048_576 => format!("{:.1} MB", bytes as f64 / 1_048_576.0),
        Some(bytes) if bytes >= 1024 => format!("{:.1} KB", bytes as f64 / 1024.0),
        Some(bytes) => format!("{} B", bytes),
        None => "0 B".to_string(),
    };

    let time_str = metadata
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|| {
            path.file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        });

    let tag = if is_latest { " (Latest)" } else { "" };
    format!("{}{} • {}", time_str, tag, size_str)
}

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

    let mut logger = Logger::try_with_str(log_filter)?.format_for_stderr(|w, _now, record| {
        if record.target() == "mpv_music::ui" {
            return Ok(());
        }
        let level = record.level();
        write!(
            w,
            "[{}] {}",
            style(level).paint(level.as_str()),
            record.args()
        )
    });

    if cfg.enable_file_logging {
        let dir = logs_dir(log_dir);
        std::fs::create_dir_all(&dir)?;

        let legacy = log_dir.join("mpv-music.log");
        if legacy.exists() {
            let _ = std::fs::remove_file(&legacy);
        }

        let retain_prior = cfg.max_log_sessions.max(1).saturating_sub(1);
        let _ = prune_old_logs(log_dir, retain_prior);

        let now = chrono::Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S");
        let pid = std::process::id();
        let basename = format!("session_{}_{}", timestamp, pid);

        logger = logger
            .log_to_file(
                FileSpec::default()
                    .directory(&dir)
                    .basename(basename)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_and_prune_logs() {
        let temp_dir =
            std::env::temp_dir().join(format!("mpv_music_log_test_{}", std::process::id()));
        let logs = logs_dir(&temp_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&logs).unwrap();

        let file1 = logs.join("session_20260101_100000_100.log");
        let file2 = logs.join("session_20260101_110000_101.log");
        let file3 = logs.join("session_20260101_120000_102.log");
        let file4 = logs.join("session_20260101_130000_103.log");
        let file5 = logs.join("session_20260101_140000_104.log");

        std::fs::write(&file1, "session 1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(&file2, "session 2").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(&file3, "session 3").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(&file4, "session 4").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(&file5, "session 5").unwrap();

        let listed = list_log_files(&temp_dir);
        assert_eq!(listed.len(), 5);
        assert_eq!(listed[0], file5);
        assert_eq!(listed[4], file1);

        let pruned = prune_old_logs(&temp_dir, 3).unwrap();
        assert_eq!(pruned, 2);

        let remaining = list_log_files(&temp_dir);
        assert_eq!(remaining.len(), 3);
        assert!(remaining.contains(&file5));
        assert!(remaining.contains(&file4));
        assert!(remaining.contains(&file3));
        assert!(!file1.exists());
        assert!(!file2.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_format_log_display() {
        let temp_file =
            std::env::temp_dir().join(format!("test_format_{}.log", std::process::id()));
        std::fs::write(&temp_file, "hello logs").unwrap();

        let formatted = format_log_display(&temp_file, true);
        assert!(formatted.contains("(Latest)"));
        assert!(formatted.contains("10 B"));

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_list_log_files_ignores_foreign_files() {
        let temp = tempfile::tempdir().unwrap();
        let logs = logs_dir(temp.path());
        std::fs::create_dir_all(&logs).unwrap();

        std::fs::write(logs.join("session_1.log"), "log1").unwrap();
        std::fs::write(logs.join("notes.txt"), "text").unwrap();
        std::fs::write(logs.join("temp.tmp"), "tmp").unwrap();
        std::fs::create_dir_all(logs.join("nested_folder")).unwrap();

        let listed = list_log_files(temp.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0], logs.join("session_1.log"));
    }

    #[test]
    fn test_list_log_files_includes_legacy_log() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("mpv-music.log");
        std::fs::write(&legacy, "legacy content").unwrap();

        let logs = logs_dir(temp.path());
        std::fs::create_dir_all(&logs).unwrap();
        let session = logs.join("session_2026.log");
        std::fs::write(&session, "session content").unwrap();

        let listed = list_log_files(temp.path());
        assert_eq!(listed.len(), 2);
        assert!(listed.contains(&legacy));
        assert!(listed.contains(&session));
    }

    #[test]
    fn test_prune_old_logs_exact_boundary() {
        let temp = tempfile::tempdir().unwrap();
        let logs = logs_dir(temp.path());
        std::fs::create_dir_all(&logs).unwrap();

        for i in 0..3 {
            std::fs::write(
                logs.join(format!("session_{}.log", i)),
                format!("content {}", i),
            )
            .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let pruned = prune_old_logs(temp.path(), 3).unwrap();
        assert_eq!(pruned, 0);
        assert_eq!(list_log_files(temp.path()).len(), 3);
    }

    #[test]
    fn test_remove_oldest_logs_partial_via_prune() {
        let temp = tempfile::tempdir().unwrap();
        let logs = logs_dir(temp.path());
        std::fs::create_dir_all(&logs).unwrap();

        let mut files = Vec::new();
        for i in 0..4 {
            let f = logs.join(format!("session_{}.log", i));
            std::fs::write(&f, format!("content {}", i)).unwrap();
            files.push(f);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Keeping 2 newest means deleting the 2 oldest
        let removed = prune_old_logs(temp.path(), 2).unwrap();
        assert_eq!(removed, 2);

        let remaining = list_log_files(temp.path());
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0], files[3]);
        assert_eq!(remaining[1], files[2]);
        assert!(!files[0].exists());
        assert!(!files[1].exists());
    }

    #[test]
    fn test_prune_all_logs_to_zero() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("mpv-music.log");
        std::fs::write(&legacy, "legacy").unwrap();

        let logs = logs_dir(temp.path());
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join("session_1.log"), "session 1").unwrap();
        std::fs::write(logs.join("session_2.log"), "session 2").unwrap();

        let removed = prune_old_logs(temp.path(), 0).unwrap();
        assert_eq!(removed, 3);
        assert!(!legacy.exists());
        assert!(list_log_files(temp.path()).is_empty());
    }

    #[test]
    fn test_format_log_display_legacy_and_session() {
        let temp = tempfile::tempdir().unwrap();
        let logs = logs_dir(temp.path());
        std::fs::create_dir_all(&logs).unwrap();

        let session_file = logs.join("session_20260916_123045_99999.log");
        std::fs::write(&session_file, "some logs").unwrap();

        let display = format_log_display(&session_file, false);
        assert!(display.contains("9 B"));
        assert!(!display.contains("(Latest)"));

        let legacy_file = temp.path().join("mpv-music.log");
        std::fs::write(&legacy_file, "legacy data 123").unwrap();
        let legacy_display = format_log_display(&legacy_file, true);
        assert!(legacy_display.contains("15 B"));
        assert!(legacy_display.contains("(Latest)"));
    }
}
