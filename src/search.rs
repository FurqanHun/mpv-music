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
        .args(&args)
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
                let m = (seconds / 60.0).floor();
                let s = (seconds % 60.0).floor();
                format!("{:02}:{:02}", m, s)
            } else {
                "LIVE/???".to_string()
            };

            // Views: 1200000 -> 1.2M
            let views = if let Some(count) = v["view_count"].as_u64() {
                if count >= 1_000_000 {
                    format!("{:.1}M", count as f64 / 1_000_000.0)
                } else if count >= 1_000 {
                    format!("{:.1}K", count as f64 / 1_000.0)
                } else {
                    count.to_string()
                }
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
        "Search finished. Found: {}, Ignored Channels: {}, Bad URLs: {}",
        results.len(),
        stats_channels,
        stats_bad_url
    );

    Ok(results)
}
