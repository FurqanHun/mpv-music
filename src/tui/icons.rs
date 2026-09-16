use crate::config::NerdFontMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icons {
    pub mode: NerdFontMode,
}

impl Icons {
    pub fn new(mode: NerdFontMode) -> Self {
        Self { mode }
    }

    pub fn pad(&self, icon: &str) -> String {
        match self.mode {
            NerdFontMode::Normal => format!("{}  ", icon),
            _ => format!("{} ", icon),
        }
    }

    pub fn prompt(&self, icon: &str, label: &str) -> String {
        format!("{}{} > ", self.pad(icon), label)
    }

    pub fn track(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "🎵",
            _ => "\u{f001}",
        }
    }

    pub fn video(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "🎬",
            _ => "\u{f008}",
        }
    }

    pub fn video_stream(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "📺",
            _ => "\u{f008}",
        }
    }

    pub fn folder(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "📁",
            _ => "\u{f07b}",
        }
    }

    pub fn folder_open(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "📂",
            _ => "\u{f07c}",
        }
    }

    pub fn playlist(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "📜",
            _ => "\u{f0ca}",
        }
    }

    pub fn search(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "🔎",
            _ => "\u{f002}",
        }
    }

    pub fn target_search(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "🎯",
            _ => "\u{f002}",
        }
    }

    pub fn settings(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "⚙️",
            _ => "\u{f013}",
        }
    }

    pub fn mode_prompt(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "🎧",
            _ => "\u{f025}",
        }
    }

    pub fn filter(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "🔎",
            _ => "\u{f0b0}",
        }
    }

    pub fn genre(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "🏷️",
            _ => "\u{f02b}",
        }
    }

    pub fn artist(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "🎤",
            _ => "\u{f130}",
        }
    }

    pub fn album(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "💿",
            _ => "\u{f001}",
        }
    }

    pub fn trash(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "🗑️",
            _ => "\u{f1f8}",
        }
    }

    pub fn radio(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "📻",
            _ => "\u{f1be}",
        }
    }

    pub fn play(&self) -> &'static str {
        match self.mode {
            NerdFontMode::None => "▶",
            _ => "\u{f04b}",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icons_none_mode_returns_standard_unicode() {
        let icons = Icons::new(NerdFontMode::None);
        assert_eq!(icons.track(), "🎵");
        assert_eq!(icons.video(), "🎬");
        assert_eq!(icons.video_stream(), "📺");
        assert_eq!(icons.folder(), "📁");
        assert_eq!(icons.folder_open(), "📂");
        assert_eq!(icons.playlist(), "📜");
        assert_eq!(icons.search(), "🔎");
        assert_eq!(icons.settings(), "⚙️");
        assert_eq!(icons.radio(), "📻");
        assert_eq!(icons.play(), "▶");
    }

    #[test]
    fn test_icons_nerd_font_mode_returns_glyphs() {
        let icons_normal = Icons::new(NerdFontMode::Normal);
        assert_eq!(icons_normal.track(), "\u{f001}");
        assert_eq!(icons_normal.video(), "\u{f008}");
        assert_eq!(icons_normal.folder(), "\u{f07b}");
        assert_eq!(icons_normal.settings(), "\u{f013}");
        assert_eq!(icons_normal.radio(), "\u{f1be}");
        assert_eq!(icons_normal.play(), "\u{f04b}");

        let icons_mono = Icons::new(NerdFontMode::Mono);
        assert_eq!(icons_mono.track(), "\u{f001}");
        assert_eq!(icons_mono.radio(), "\u{f1be}");
    }

    #[test]
    fn test_icons_padding_and_prompt() {
        let normal = Icons::new(NerdFontMode::Normal);
        assert_eq!(normal.pad("X"), "X  ");
        assert_eq!(normal.prompt("X", "Search"), "X  Search > ");

        let none = Icons::new(NerdFontMode::None);
        assert_eq!(none.pad("X"), "X ");
        assert_eq!(none.prompt("X", "Tracks"), "X Tracks > ");
    }
}
