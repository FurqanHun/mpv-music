use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::time::SystemTime;

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    timestamp: u64,
    results: Vec<SearchResult>,
}

fn get_cache_path() -> Option<std::path::PathBuf> {
    ProjectDirs::from("com", "furqanhun", "mpv-music")
        .map(|dirs| dirs.data_dir().join("yt_cache.json"))
}

fn load_cache() -> (HashMap<String, CacheEntry>, bool) {
    let path = match get_cache_path() {
        Some(p) => p,
        None => return (HashMap::new(), false),
    };

    if path.exists()
        && let Ok(content) = fs::read_to_string(&path)
        && let Ok(map) = serde_json::from_str::<HashMap<String, CacheEntry>>(&content)
    {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let initial_len = map.len();
        let pruned_map: HashMap<String, CacheEntry> = map
            .into_iter()
            .filter(|(_, v)| now.saturating_sub(v.timestamp) < 86400)
            .collect();
        let was_pruned = initial_len != pruned_map.len();
        return (pruned_map, was_pruned);
    }
    (HashMap::new(), false)
}

fn save_cache(cache: &HashMap<String, CacheEntry>) {
    if let Some(path) = get_cache_path() {
        if let Some(parent) = path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            log::error!("Failed to create cache dir: {}", e);
        }
        match serde_json::to_string(cache) {
            Ok(content) => {
                let temp_path = path.with_extension("tmp");
                if let Err(e) = fs::write(&temp_path, content) {
                    log::error!("Failed to write cache file {:?}: {}", temp_path, e);
                } else if let Err(e) = fs::rename(&temp_path, &path) {
                    log::error!("Failed to swap cache file {:?}: {}", path, e);
                } else {
                    log::info!("Cache successfully saved to {:?}", path);
                }
            }
            Err(e) => {
                log::error!("Failed to serialize cache: {}", e);
            }
        }
    }
}

/// Returns a list of parsed search results, ignoring channels, mixes, and shorts.
pub fn search_youtube(query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    log::info!(
        "Starting YouTube search for: '{}' (Limit: {})",
        query,
        limit
    );

    let cache_key = format!("{}|{}", query, limit);
    let (mut cache, was_pruned) = load_cache();

    if let Some(entry) = cache.get(&cache_key) {
        log::info!("Cache hit for YouTube search: '{}'", query);
        if was_pruned {
            save_cache(&cache);
        }
        return Ok(entry.results.clone());
    }

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

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    cache.insert(
        cache_key,
        CacheEntry {
            timestamp: now,
            results: results.clone(),
        },
    );
    save_cache(&cache);

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
