use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetKind {
    LocalFile = 0,
    GenericUrl = 1,
    YouTube = 2,
}

impl TargetKind {
    pub fn is_network(&self) -> bool {
        *self >= TargetKind::GenericUrl
    }

    pub fn is_youtube(&self) -> bool {
        *self == TargetKind::YouTube
    }
}

pub fn classify_target(s: &str) -> TargetKind {
    if s.contains("youtube.com") || s.contains("youtu.be") {
        TargetKind::YouTube
    } else if s.starts_with("http") || s.starts_with("ftp") {
        TargetKind::GenericUrl
    } else {
        TargetKind::LocalFile
    }
}

// to find the "heaviest" URL
pub fn inspect_playlist_content(path_str: &str, config: &Config) -> Option<String> {
    let path = std::path::Path::new(path_str);

    let ext = path.extension()?.to_str()?.to_lowercase();
    if !config.playlist_exts.contains(&ext) {
        return None;
    }

    let mut best_match: Option<String> = None;
    let mut max_kind = TargetKind::LocalFile;

    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let trim = line.trim();
            let kind = classify_target(trim);

            if kind > max_kind {
                max_kind = kind;
                best_match = Some(trim.to_string());
                if max_kind == TargetKind::YouTube {
                    break;
                }
            }
        }
    }
    best_match
}

pub fn resolve_target_optimization(target: &str, config: &Config) -> String {
    if let Some(inner_url) = inspect_playlist_content(target, config) {
        log::debug!(
            "Playlist content scan found network link. optimizing for: {}",
            inner_url
        );
        inner_url
    } else {
        target.to_string()
    }
}

pub fn find_representative_target(paths: &[String]) -> Option<&str> {
    let mut best_target = paths.first().map(|s| s.as_str());
    let mut max_kind = TargetKind::LocalFile;

    for path in paths {
        let kind = classify_target(path);
        if kind > max_kind {
            max_kind = kind;
            best_target = Some(path.as_str());
            if max_kind == TargetKind::YouTube {
                break;
            }
        }
    }
    best_target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_youtube() {
        assert_eq!(
            classify_target("https://youtube.com/watch?v=test"),
            TargetKind::YouTube
        );
        assert_eq!(
            classify_target("https://youtu.be/test123"),
            TargetKind::YouTube
        );
        assert_eq!(
            classify_target("http://youtube.com/playlist"),
            TargetKind::YouTube
        );
    }

    #[test]
    fn test_classify_generic_url() {
        assert_eq!(
            classify_target("https://example.com/song.mp3"),
            TargetKind::GenericUrl
        );
        assert_eq!(
            classify_target("http://radio.com/stream"),
            TargetKind::GenericUrl
        );
        assert_eq!(
            classify_target("ftp://server.com/file"),
            TargetKind::GenericUrl
        );
    }

    #[test]
    fn test_classify_local_file() {
        assert_eq!(
            classify_target("/home/user/music.mp3"),
            TargetKind::LocalFile
        );
        assert_eq!(classify_target("./local/file.flac"), TargetKind::LocalFile);
        assert_eq!(
            classify_target("C:\\Music\\song.mp3"),
            TargetKind::LocalFile
        );
    }

    #[test]
    fn test_classify_empty() {
        assert_eq!(classify_target(""), TargetKind::LocalFile);
    }

    #[test]
    fn test_classify_priority_order() {
        let youtube = classify_target("https://youtube.com/test");
        let http = classify_target("https://example.com/test");
        let local = classify_target("/path/to/file");

        assert!(youtube > http);
        assert!(http > local);
    }

    #[test]
    fn test_find_representative_target() {
        let files = vec![
            "/path/to/song.mp3".to_string(),
            "https://example.com/radio".to_string(),
            "https://youtube.com/watch?v=123".to_string(),
        ];
        assert_eq!(
            find_representative_target(&files),
            Some("https://youtube.com/watch?v=123")
        );

        let local_only = vec![
            "/path/to/one.mp3".to_string(),
            "/path/to/two.flac".to_string(),
        ];
        assert_eq!(
            find_representative_target(&local_only),
            Some("/path/to/one.mp3")
        );

        let empty: Vec<String> = Vec::new();
        assert_eq!(find_representative_target(&empty), None);
    }
}
