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

        let type_str = if self.track.is_video() {
            "Video"
        } else {
            "Audio"
        };
        let icon = self.icon;

        let text = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mArtist:\x1b[0m {}\n  \x1b[1;32mAlbum:\x1b[0m  {}\n  \x1b[1;35mGenre:\x1b[0m  {}\n  \x1b[1;34mType:\x1b[0m   {} ({})\n\n  \x1b[90mPath: {}\x1b[0m",
            icon,
            self.track.title,
            self.track.artist,
            self.track.album,
            self.track.genre,
            type_str,
            ext,
            self.track.path
        );
        ItemPreview::AnsiText(text)
    }
}

pub struct TagItem {
    pub name: String,
    pub count: usize,
    pub samples: Vec<String>,
    pub icon: String,
}

impl TagItem {
    pub fn new(name: String, count: usize, samples: Vec<String>, icon: String) -> Self {
        Self {
            name,
            count,
            samples,
            icon,
        }
    }
}

impl SkimItem for TagItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} ({})", self.name, self.count))
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
    pub icon: &'static str,
}

impl DirItem {
    pub fn new(
        dirname: String,
        path: String,
        count: usize,
        samples: Vec<String>,
        icon: &'static str,
    ) -> Self {
        Self {
            dirname,
            path,
            count,
            samples,
            icon,
        }
    }
}

impl SkimItem for DirItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} ({})", self.dirname, self.count))
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.path)
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
            let noun = if remaining == 1 { "file" } else { "files" };
            sample_text.push_str(&format!(
                "  \x1b[90m... and {} more {}\x1b[0m\n",
                remaining, noun
            ));
        }

        let section_header = if self.count <= max_samples {
            "Contents:"
        } else {
            "Sample Contents:"
        };

        let output = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mPath:\x1b[0m {}\n  \x1b[1;33mFiles:\x1b[0m {}\n\n  \x1b[1;32m{}\x1b[0m\n{}",
            self.icon, self.dirname, self.path, self.count, section_header, sample_text
        );
        ItemPreview::AnsiText(output)
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
}
