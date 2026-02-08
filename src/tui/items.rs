use crate::indexer;
use crate::search;
use skim::prelude::*;
use std::borrow::Cow;

pub struct TrackItem {
    pub track: indexer::Track,
    pub display_text: String,
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

        let type_str = if self.track.media_type == "video" {
            "Video"
        } else {
            "Audio"
        };
        let icon = if self.track.media_type == "video" {
            "🎬"
        } else {
            "🎵"
        };

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

impl SkimItem for TagItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} ({})", self.name, self.count))
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let mut sample_text = String::new();
        for (i, song) in self.samples.iter().enumerate() {
            if i >= 10 {
                break;
            } // limit to 10
            sample_text.push_str(&format!("  {}. {}\n", i + 1, song));
        }

        let output = format!(
            "\n  {} \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mTotal Tracks:\x1b[0m {}\n\n  \x1b[1;32mSample Tracks:\x1b[0m\n{}",
            self.icon, self.name, self.count, sample_text
        );
        ItemPreview::AnsiText(output)
    }
}

pub struct DirItem {
    pub dirname: String,
    pub path: String,
    pub count: usize,
    pub samples: Vec<String>,
}

impl SkimItem for DirItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} ({})", self.dirname, self.count))
    }
    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.path)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let mut sample_text = String::new();
        for (i, song) in self.samples.iter().enumerate() {
            if i >= 10 {
                break;
            }
            sample_text.push_str(&format!("  {}. {}\n", i + 1, song));
        }

        let output = format!(
            "\n  📁 \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mPath:\x1b[0m {}\n  \x1b[1;33mFiles:\x1b[0m {}\n\n  \x1b[1;32mContents:\x1b[0m\n{}",
            self.dirname, self.path, self.count, sample_text
        );
        ItemPreview::AnsiText(output)
    }
}

pub struct PlaylistItem {
    pub name: String,
    pub path: String,
    pub count: usize,
    pub preview_lines: Vec<String>,
}

impl SkimItem for PlaylistItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.path)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let mut content = String::new();
        for (i, line) in self.preview_lines.iter().enumerate() {
            if i >= 10 {
                break;
            }
            content.push_str(&format!("  {}. {}\n", i + 1, line));
        }

        if content.is_empty() {
            content.push_str("  (Empty or Binary Playlist)\n");
        }

        let output = format!(
            "\n  📜 \x1b[1;36m{}\x1b[0m\n\n  \x1b[1;33mPath:\x1b[0m {}\n  \x1b[1;33mEntries:\x1b[0m {}\n\n  \x1b[1;32mFirst Few Tracks:\x1b[0m\n{}",
            self.name, self.path, self.count, content
        );
        ItemPreview::AnsiText(output)
    }
}

pub struct SearchItem {
    pub result: search::SearchResult,
}

impl SkimItem for SearchItem {
    fn text(&self) -> Cow<'_, str> {
        // list, just lil bit
        Cow::Borrowed(&self.result.title)
    }
    fn output(&self) -> Cow<'_, str> {
        // url for the player
        Cow::Borrowed(&self.result.url)
    }

    fn preview(&self, _ctx: PreviewContext) -> ItemPreview {
        let (icon, type_str) = if self.result.is_playlist {
            ("📜", "Playlist / Mix")
        } else {
            ("📺", "Video")
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
