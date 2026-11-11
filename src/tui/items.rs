use crate::indexer;
use crate::search;
use skim::prelude::*;
use std::borrow::Cow;

pub struct TrackItem {
    pub track: indexer::Track,
    pub display_text: String,
    pub icon: &'static str,
}

impl TrackItem {
    pub fn new(track: indexer::Track, icon: &'static str) -> Self {
        let display_text = format!("{} - {}", track.artist, track.title);
        Self {
            track,
            display_text,
            icon,
        }
    }
}

impl SkimItem for TrackItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display_text)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.track.path)
    }

    fn preview(&self, _ctx: PreviewContext) -> ItemPreview {
        let ext = std::path::Path::new(&self.track.path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("???")
            .to_uppercase();

        let type_str = if self.track.is_playlist() {
            "Playlist"
        } else if self.track.is_video() {
            "Video"
        } else {
            "Audio"
        };
        let icon = self.icon;

        let mut text = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mArtist:\x1b[0m   {}\n  \x1b[1;32mAlbum:\x1b[0m    {}\n  \x1b[1;35mGenre:\x1b[0m    {}",
            icon, self.track.title, self.track.artist, self.track.album, self.track.genre
        );

        if !self.track.is_playlist() {
            if self.track.duration > 0 {
                text.push_str(&format!(
                    "\n  \x1b[1;33mDuration:\x1b[0m {}",
                    indexer::format_duration(self.track.duration)
                ));
            }
            if let Some(quality) =
                indexer::format_quality(self.track.sample_rate, self.track.bitrate)
            {
                text.push_str(&format!("\n  \x1b[1;36mQuality:\x1b[0m  {}", quality));
            }
        }

        text.push_str(&format!(
            "\n  \x1b[1;34mType:\x1b[0m     {} ({})\n\n  \x1b[90mPath: {}\x1b[0m",
            type_str, ext, self.track.path
        ));

        ItemPreview::AnsiText(text)
    }
}

pub struct TagItem {
    pub name: String,
    pub count: usize,
    pub samples: Vec<String>,
    pub icon: String,
    pub display_text: String,
}

impl TagItem {
    pub fn new(name: String, count: usize, samples: Vec<String>, icon: String) -> Self {
        let display_text = format!("{} ({})", name, count);
        Self {
            name,
            count,
            samples,
            icon,
            display_text,
        }
    }
}

impl SkimItem for TagItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display_text)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let max_samples = 15;
        let mut sample_text = String::new();
        let shown = self.samples.len().min(max_samples);
        for (i, song) in self.samples.iter().take(shown).enumerate() {
            sample_text.push_str(&format!("  {}. {}\n", i + 1, song));
        }

        if self.count > shown {
            let remaining = self.count - shown;
            let noun = if remaining == 1 { "track" } else { "tracks" };
            sample_text.push_str(&format!(
                "  \x1b[90m... and {} more {}\x1b[0m\n",
                remaining, noun
            ));
        }

        let section_header = if self.count <= max_samples {
            "Tracks:"
        } else {
            "Sample Tracks:"
        };

        let output = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mTotal Tracks:\x1b[0m {}\n\n  \x1b[1;32m{}\x1b[0m\n{}",
            self.icon, self.name, self.count, section_header, sample_text
        );
        ItemPreview::AnsiText(output)
    }
}

pub struct DirItem {
    pub dirname: String,
    pub path: String,
    pub count: usize,
    pub samples: Vec<String>,
    pub subfolders: Vec<(String, usize)>,
    pub root_info: Option<String>,
    pub icon: &'static str,
    pub is_root: bool,
    pub display_text: String,
}

impl DirItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dirname: String,
        path: String,
        count: usize,
        samples: Vec<String>,
        subfolders: Vec<(String, usize)>,
        root_info: Option<String>,
        icon: &'static str,
        is_root: bool,
    ) -> Self {
        let display_text = if is_root {
            format!("[Root] {} ({})", dirname, count)
        } else {
            format!("{} ({})", dirname, count)
        };
        let formatted_dirname = if is_root {
            format!("[Root] {}", dirname)
        } else {
            dirname
        };
        Self {
            dirname: formatted_dirname,
            path,
            count,
            samples,
            subfolders,
            root_info,
            icon,
            is_root,
            display_text,
        }
    }
}

impl SkimItem for DirItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display_text)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.path)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let mut lines = Vec::new();
        lines.push(format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n",
            self.icon, self.dirname
        ));

        if self.is_root {
            lines.push(format!("  \x1b[1;33mRoot:\x1b[0m  {}", self.path));
        } else if let Some(ref root) = self.root_info {
            lines.push(format!("  \x1b[1;33mRoot:\x1b[0m  {}", root));
            lines.push(format!("  \x1b[1;33mPath:\x1b[0m  {}", self.path));
        } else {
            lines.push(format!("  \x1b[1;33mPath:\x1b[0m  {}", self.path));
        }

        let tracks_noun = if self.count == 1 { "track" } else { "tracks" };
        lines.push(format!(
            "  \x1b[1;33mFiles:\x1b[0m {} {}",
            self.count, tracks_noun
        ));

        if !self.subfolders.is_empty() {
            let max_subfolders = 12;
            let count_sub = self.subfolders.len();
            lines.push(String::new());
            lines.push(format!("  \x1b[1;32mSubfolders ({}):\x1b[0m", count_sub));
            for (sub_name, sub_count) in self.subfolders.iter().take(max_subfolders) {
                let noun = if *sub_count == 1 { "track" } else { "tracks" };
                lines.push(format!(
                    "    {} {} \x1b[90m({} {})\x1b[0m",
                    self.icon, sub_name, sub_count, noun
                ));
            }
            if count_sub > max_subfolders {
                let rem = count_sub - max_subfolders;
                lines.push(format!(
                    "    \x1b[90m... and {} more subfolders\x1b[0m",
                    rem
                ));
            }
        }

        if !self.samples.is_empty() {
            let max_samples = 15;
            lines.push(String::new());
            let section_header = if !self.subfolders.is_empty() {
                format!("Direct Files ({}):", self.samples.len())
            } else if self.count <= max_samples {
                "Contents:".to_string()
            } else {
                "Sample Contents:".to_string()
            };

            lines.push(format!("  \x1b[1;32m{}\x1b[0m", section_header));
            let shown = self.samples.len().min(max_samples);
            for (i, song) in self.samples.iter().take(shown).enumerate() {
                lines.push(format!("    {}. {}", i + 1, song));
            }
            if self.samples.len() > max_samples {
                let rem = self.samples.len() - max_samples;
                let noun = if rem == 1 { "file" } else { "files" };
                lines.push(format!("    \x1b[90m... and {} more {}\x1b[0m", rem, noun));
            }
        }

        ItemPreview::AnsiText(lines.join("\n"))
    }
}

pub struct PlaylistItem {
    pub name: String,
    pub path: String,
    pub count: usize,
    pub preview_lines: Vec<String>,
    pub icon: &'static str,
}

impl PlaylistItem {
    pub fn new(
        name: String,
        path: String,
        count: usize,
        preview_lines: Vec<String>,
        icon: &'static str,
    ) -> Self {
        Self {
            name,
            path,
            count,
            preview_lines,
            icon,
        }
    }
}

impl SkimItem for PlaylistItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.path)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let max_samples = 15;
        let mut content = String::new();
        let shown = self.preview_lines.len().min(max_samples);
        for (i, line) in self.preview_lines.iter().take(shown).enumerate() {
            content.push_str(&format!("  {}. {}\n", i + 1, line));
        }

        if self.count > shown {
            let remaining = self.count - shown;
            let noun = if remaining == 1 { "entry" } else { "entries" };
            content.push_str(&format!(
                "  \x1b[90m... and {} more {}\x1b[0m\n",
                remaining, noun
            ));
        }

        if content.is_empty() {
            content.push_str("  (Empty or Binary Playlist)\n");
        }

        let section_header = if self.count <= max_samples {
            "Tracks:"
        } else {
            "Sample Tracks:"
        };

        let output = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mPath:\x1b[0m {}\n  \x1b[1;33mEntries:\x1b[0m {}\n\n  \x1b[1;32m{}\x1b[0m\n{}",
            self.icon, self.name, self.path, self.count, section_header, content
        );
        ItemPreview::AnsiText(output)
    }
}

pub struct SearchItem {
    pub result: search::SearchResult,
    pub playlist_icon: &'static str,
    pub video_icon: &'static str,
}

impl SearchItem {
    pub fn new(
        result: search::SearchResult,
        playlist_icon: &'static str,
        video_icon: &'static str,
    ) -> Self {
        Self {
            result,
            playlist_icon,
            video_icon,
        }
    }
}

impl SkimItem for SearchItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.result.title)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.result.url)
    }

    fn preview(&self, _ctx: PreviewContext) -> ItemPreview {
        let (icon, type_str) = if self.result.is_playlist {
            (self.playlist_icon, "Playlist / Mix")
        } else if self.result.duration == "LIVE" || self.result.duration == "LIVE/???" {
            (self.video_icon, "Live Stream")
        } else {
            (self.video_icon, "Video")
        };

        let details = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mChannel:\x1b[0m  {}\n  \x1b[1;33mViews:\x1b[0m    {}\n  \x1b[1;33mDuration:\x1b[0m {}\n  \x1b[1;34mType:\x1b[0m      {}\n\n  \x1b[90mURL: {}\x1b[0m",
            icon,
            self.result.title,
            self.result.uploader,
            self.result.view_count,
            self.result.duration,
            type_str,
            self.result.url
        );
        ItemPreview::AnsiText(details)
    }
}

pub struct MenuItem {
    pub text: String,
    pub id: String,
}

impl SkimItem for MenuItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.text)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.id)
    }

    fn preview(&self, _ctx: PreviewContext) -> ItemPreview {
        ItemPreview::Text(self.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::icons::Icons;

    #[test]
    fn test_search_item_preview_live_stream() {
        let live_result = search::SearchResult {
            title: "Lofi Live".to_string(),
            url: "https://youtube.com/watch?v=live123".to_string(),
            uploader: "Lofi Channel".to_string(),
            duration: "LIVE".to_string(),
            view_count: "N/A".to_string(),
            is_playlist: false,
        };
        let item = SearchItem::new(live_result, "icon_pl", "icon_vid");
        let preview = item.preview(PreviewContext {
            query: "",
            cmd_query: "",
            width: 80,
            height: 24,
            current_index: 0,
            current_selection: "",
            selected_indices: &[],
            selections: &[],
        });
        if let ItemPreview::AnsiText(details) = preview {
            assert!(details.contains("Live Stream"));
            assert!(details.contains("LIVE"));
        } else {
            panic!("Expected AnsiText preview");
        }
    }

    #[test]
    fn test_search_item_preview_regular_video() {
        let video_result = search::SearchResult {
            title: "Regular Track".to_string(),
            url: "https://youtube.com/watch?v=vod123".to_string(),
            uploader: "Artist Channel".to_string(),
            duration: "03:45".to_string(),
            view_count: "1.2M".to_string(),
            is_playlist: false,
        };
        let item = SearchItem::new(video_result, "icon_pl", "icon_vid");
        let preview = item.preview(PreviewContext {
            query: "",
            cmd_query: "",
            width: 80,
            height: 24,
            current_index: 0,
            current_selection: "",
            selected_indices: &[],
            selections: &[],
        });
        if let ItemPreview::AnsiText(details) = preview {
            assert!(details.contains("Video"));
            assert!(details.contains("03:45"));
        } else {
            panic!("Expected AnsiText preview");
        }
    }

    #[test]
    fn test_tag_item_text_zero_allocation_borrow() {
        let icons = Icons::new(crate::config::NerdFontMode::Normal);
        let item = TagItem::new(
            "Rock".to_string(),
            42,
            vec!["Song 1".to_string()],
            icons.genre().to_string(),
        );
        let text_cow = item.text();
        assert_eq!(text_cow, "Rock (42)");
        assert!(matches!(text_cow, Cow::Borrowed(_)));
    }

    #[test]
    fn test_dir_item_text_zero_allocation_borrow() {
        let icons = Icons::new(crate::config::NerdFontMode::Normal);
        let item = DirItem::new(
            "Favorites".to_string(),
            "/music/fav".to_string(),
            15,
            vec![],
            vec![],
            None,
            icons.folder(),
            false,
        );
        let text_cow = item.text();
        assert_eq!(text_cow, "Favorites (15)");
        assert!(matches!(text_cow, Cow::Borrowed(_)));
    }

    #[test]
    fn test_dir_item_root_badge() {
        let icons = Icons::new(crate::config::NerdFontMode::Normal);
        let item = DirItem::new(
            "musicdd".to_string(),
            "/music/musicdd".to_string(),
            42,
            vec![],
            vec![],
            None,
            icons.folder(),
            true,
        );
        let text_cow = item.text();
        assert_eq!(text_cow, "[Root] musicdd (42)");
        assert_eq!(item.dirname, "[Root] musicdd");
        assert!(item.is_root);
    }

    #[test]
    fn test_dir_item_adaptive_preview_container_leaf_mixed() {
        let dummy_ctx = || PreviewContext {
            query: "",
            cmd_query: "",
            width: 80,
            height: 24,
            current_index: 0,
            current_selection: "",
            selected_indices: &[],
            selections: &[],
        };
        let icons = Icons::new(crate::config::NerdFontMode::Normal);

        // 1. Container directory (e.g. root with only subfolders)
        let container = DirItem::new(
            "musicdd".to_string(),
            "/music/musicdd".to_string(),
            30,
            vec![],
            vec![("Ado".to_string(), 20), ("Queen".to_string(), 10)],
            None,
            icons.folder(),
            true,
        );
        if let ItemPreview::AnsiText(p) = container.preview(dummy_ctx()) {
            assert!(p.contains("Root:"));
            assert!(p.contains("/music/musicdd"));
            assert!(p.contains("Subfolders (2):"));
            assert!(p.contains(&format!("{} Ado", icons.folder())));
            assert!(p.contains(&format!("{} Queen", icons.folder())));
            assert!(!p.contains("Contents:"));
            assert!(!p.contains("Direct Files"));
        } else {
            panic!("Expected AnsiText preview");
        }

        // 2. Leaf directory (only direct files)
        let leaf = DirItem::new(
            "Kyogen".to_string(),
            "/music/musicdd/Ado/Kyogen".to_string(),
            2,
            vec!["01 - Song.flac".to_string(), "02 - Song.flac".to_string()],
            vec![],
            Some("musicdd (/music/musicdd)".to_string()),
            icons.folder(),
            false,
        );
        if let ItemPreview::AnsiText(p) = leaf.preview(dummy_ctx()) {
            assert!(p.contains("Root:"));
            assert!(p.contains("musicdd (/music/musicdd)"));
            assert!(p.contains("Path:"));
            assert!(p.contains("/music/musicdd/Ado/Kyogen"));
            assert!(p.contains("Contents:"));
            assert!(p.contains("1. 01 - Song.flac"));
            assert!(!p.contains("Subfolders"));
        } else {
            panic!("Expected AnsiText preview");
        }

        // 3. Mixed directory (both subfolders and direct files, with Nerd Font icon)
        let mixed = DirItem::new(
            "Music".to_string(),
            "/home/qan/Music".to_string(),
            12,
            vec!["Loose.mp3".to_string()],
            vec![("Rock".to_string(), 11)],
            None,
            icons.folder(),
            true,
        );
        if let ItemPreview::AnsiText(p) = mixed.preview(dummy_ctx()) {
            assert!(p.contains("Subfolders (1):"));
            assert!(p.contains(&format!("{} Rock", icons.folder())));
            assert!(p.contains("Direct Files (1):"));
            assert!(p.contains("1. Loose.mp3"));
        } else {
            panic!("Expected AnsiText preview");
        }
    }

    #[test]
    fn test_track_item_display_text_and_preview() {
        let dummy_ctx = || PreviewContext {
            query: "",
            cmd_query: "",
            width: 80,
            height: 24,
            current_index: 0,
            current_selection: "",
            selected_indices: &[],
            selections: &[],
        };
        let icons = Icons::new(crate::config::NerdFontMode::Normal);

        // 1. Audio track with duration and quality specs
        let audio_track = indexer::Track {
            path: "/music/Kendrick - HUMBLE.flac".to_string(),
            title: "HUMBLE.".to_string(),
            artist: "Kendrick Lamar".to_string(),
            album: "DAMN.".to_string(),
            genre: "Hip-Hop".to_string(),
            media_type: indexer::MediaType::Audio,
            duration: 177,
            sample_rate: Some(44100),
            bitrate: Some(920),
            ..Default::default()
        };
        let audio_item = TrackItem::new(audio_track, icons.track());
        assert_eq!(audio_item.text(), "Kendrick Lamar - HUMBLE.");
        if let ItemPreview::AnsiText(preview) = audio_item.preview(dummy_ctx()) {
            assert!(preview.contains("Duration:"));
            assert!(preview.contains("02:57"));
            assert!(preview.contains("Quality:"));
            assert!(preview.contains("44.1 kHz • 920 kbps"));
            assert!(preview.contains("Audio (FLAC)"));
        } else {
            panic!("Expected AnsiText preview");
        }

        // 2. Track with 0 duration (unknown)
        let unknown_track = indexer::Track {
            path: "/music/Unknown.mp3".to_string(),
            title: "Unknown".to_string(),
            artist: "Artist".to_string(),
            media_type: indexer::MediaType::Audio,
            duration: 0,
            ..Default::default()
        };
        let unknown_item = TrackItem::new(unknown_track, icons.track());
        assert_eq!(unknown_item.text(), "Artist - Unknown");
        if let ItemPreview::AnsiText(preview) = unknown_item.preview(dummy_ctx()) {
            assert!(!preview.contains("Duration:"));
            assert!(!preview.contains("Quality:"));
            assert!(preview.contains("Audio (MP3)"));
        } else {
            panic!("Expected AnsiText preview");
        }

        // 3. Playlist track
        let playlist_track = indexer::Track {
            path: "/playlists/Chill.m3u".to_string(),
            title: "Chill".to_string(),
            artist: "Playlist".to_string(),
            media_type: indexer::MediaType::Playlist,
            duration: 0,
            ..Default::default()
        };
        let pl_item = TrackItem::new(playlist_track, icons.playlist());
        assert_eq!(pl_item.text(), "Playlist - Chill");
        if let ItemPreview::AnsiText(preview) = pl_item.preview(dummy_ctx()) {
            assert!(!preview.contains("Duration:"));
            assert!(!preview.contains("Quality:"));
            assert!(preview.contains("Playlist (M3U)"));
        } else {
            panic!("Expected AnsiText preview");
        }
    }
}
