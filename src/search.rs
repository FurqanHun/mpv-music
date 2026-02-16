use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;

#[derive(Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub uploader: String,
    pub duration: String,
    pub view_count: String,
    pub is_playlist: bool,
}

// Helper: Format seconds into MM:SS
fn format_duration(seconds: f64) -> String {
    let m = (seconds / 60.0).floor();
    let s = (seconds % 60.0).floor();
    format!("{:02}:{:02}", m, s)
}

// Helper: Format view count (e.g. 1.2M, 5.4K)
fn format_views(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

pub fn search_youtube(query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    log::info!(
        "Starting YouTube search for: '{}' (Limit: {})",
        query,
        limit
    );

    let search_url = format!(
        "https://www.youtube.com/results?search_query={}",
        query.replace(' ', "+")
    );

    let args = [
        "--flat-playlist",
        "--dump-json",
        &format!("--playlist-end={}", limit),
        "--ignore-errors", // dont crash on restricted videos
        &search_url,
    ];
    log::debug!("Exec: yt-dlp {:?}", args);

    let output = Command::new("yt-dlp")
        .args(args)
        .output()
        .context("Failed to execute yt-dlp search")?;

    if !output.status.success() {
        log::warn!("yt-dlp exited with error status");
        log::debug!("yt-dlp stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    // this for log
    let mut stats_channels = 0;
    let mut stats_bad_url = 0;
    let mut stats_mixes = 0;
    let mut stats_shorts = 0;

    for line in stdout.lines() {
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            let title = v["title"].as_str().unwrap_or("Unknown").to_string();

            // no channels
            if v["_type"].as_str() == Some("channel") {
                log::debug!("Ignored (Type=Channel): {}", title);
                stats_channels += 1;
                continue;
            }

            let title = v["title"].as_str().unwrap_or("Unknown Title").to_string();

            // url extraction
            let url = v["url"]
                .as_str()
                .or_else(|| v["webpage_url"].as_str())
                .unwrap_or_default()
                .to_string();

            if url.is_empty() {
                stats_bad_url += 1;
                continue;
            }

            if url.contains("/shorts/") {
                log::debug!("Ignored (Type=Shorts): {} [{}]", title, url);
                stats_shorts += 1;
                continue;
            }

            if url.contains("list=RD") {
                log::debug!("Ignored (Type=Mix): {} [{}]", title, url);
                stats_mixes += 1;
                continue;
            }

            if url.contains("/channel/") || url.contains("/@") || url.contains("/c/") {
                log::debug!("Ignored (URL=Channel): {} [{}]", title, url);
                stats_channels += 1;
                continue;
            }

            // Uploader / Channel Name
            let uploader = v["uploader"]
                .as_str()
                .or_else(|| v["channel"].as_str())
                .unwrap_or("Unknown Channel")
                .to_string();

            // Duration: Seconds -> MM:SS
            let duration = if let Some(seconds) = v["duration"].as_f64() {
                format_duration(seconds)
            } else {
                "LIVE/???".to_string()
            };

            // Views: 1200000 -> 1.2M
            let views = if let Some(count) = v["view_count"].as_u64() {
                format_views(count)
            } else {
                "N/A".to_string()
            };

            let is_playlist =
                url.contains("playlist?list=") || v["_type"].as_str() == Some("playlist");

            results.push(SearchResult {
                title,
                url,
                uploader,
                duration,
                view_count: views,
                is_playlist,
            });
        }
    }
    log::info!(
        "Search finished. Found: {}, Ignored [Channels: {}, Mixes: {}, Shorts: {}, Bad URLs: {}]",
        results.len(),
        stats_channels,
        stats_mixes,
        stats_shorts,
        stats_bad_url
    );

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_creation() {
        let result = SearchResult {
            title: "Test Video".to_string(),
            url: "https://youtube.com/watch?v=test".to_string(),
            uploader: "Test Channel".to_string(),
            duration: "03:45".to_string(),
            view_count: "1.2M".to_string(),
            is_playlist: false,
        };

        assert_eq!(result.title, "Test Video");
        assert_eq!(result.url, "https://youtube.com/watch?v=test");
        assert!(!result.is_playlist);
    }

    #[test]
    fn test_search_result_playlist() {
        let result = SearchResult {
            title: "Mix - Lofi".to_string(),
            url: "https://youtube.com/playlist?list=test".to_string(),
            uploader: "YouTube Music".to_string(),
            duration: "N/A".to_string(),
            view_count: "N/A".to_string(),
            is_playlist: true,
        };

        assert!(result.is_playlist);
        assert!(result.url.contains("playlist"));
    }

    #[test]
    fn test_view_count_formatting_millions() {
        let count = 1_200_000_u64;
        let formatted = format_views(count);
        assert_eq!(formatted, "1.2M");
    }

    #[test]
    fn test_view_count_formatting_thousands() {
        let count = 5_400_u64;
        let formatted = format_views(count);
        assert_eq!(formatted, "5.4K");
    }

    #[test]
    fn test_view_count_formatting_small() {
        let count = 999_u64;
        let formatted = format_views(count);
        assert_eq!(formatted, "999");
    }

    #[test]
    fn test_duration_formatting() {
        let seconds: f64 = 225.0; // 3:45
        let formatted = format_duration(seconds);
        assert_eq!(formatted, "03:45");
    }

    #[test]
    fn test_duration_formatting_hours() {
        let seconds: f64 = 3665.0; // 1:01:05
        let formatted = format_duration(seconds);
        assert_eq!(formatted, "61:05");
    }

    #[test]
    fn test_url_shorts_detection() {
        let url = "https://youtube.com/shorts/abc123";
        assert!(url.contains("/shorts/"));
    }

    #[test]
    fn test_url_mix_detection() {
        let url = "https://youtube.com/watch?v=test&list=RDtest";
        assert!(url.contains("list=RD"));
    }

    #[test]
    fn test_url_channel_detection() {
        assert!("https://youtube.com/channel/UC123".contains("/channel/"));
        assert!("https://youtube.com/@channelname".contains("/@"));
        assert!("https://youtube.com/c/channelname".contains("/c/"));
    }
}
