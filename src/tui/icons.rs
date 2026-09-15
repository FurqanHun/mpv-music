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
