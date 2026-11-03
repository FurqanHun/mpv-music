use crate::cli::Cli;
use crate::config::Config;
use crate::indexer::Track;
use crate::player;
use super::norm::{normalize_search_text, score_match};
use crate::tui;
use crate::ui;
use anyhow::Result;

pub fn is_direct_playback_target(target: &str) -> bool {
    let lower = target.to_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ftp://")
        || lower.contains("youtube.com")
        || lower.contains("youtu.be")
    {
        return true;
    }

    let path = std::path::Path::new(target);
    path.exists()
}

pub fn score_track_unified(query: &str, track: &Track) -> Option<i64> {
    let norm_query = normalize_search_text(query);
    let tokens: Vec<&str> = norm_query.split_whitespace().collect();
    if tokens.is_empty() {
        return Some(0);
    }

    let mut total_score = 0;
    for token in &tokens {
        let title_score = score_match(token, &track.title).map(|s| s + 20_000);
        let artist_score = score_match(token, &track.artist).map(|s| s + 10_000);
        let album_score = score_match(token, &track.album);

        let best_token_score = match (title_score, artist_score, album_score) {
            (None, None, None) => return None,
            (s1, s2, s3) => s1.into_iter().chain(s2).chain(s3).max().unwrap(),
        };

        total_score += best_token_score;
    }

    let total_len = (track.title.len() + track.artist.len() + track.album.len()) as i64;
    total_score -= total_len.saturating_sub(norm_query.len() as i64).min(100);

    Some(total_score)
}

pub fn resolve_unified_candidates(tracks: &[Track], query: &str) -> Vec<Track> {
    let mut scored: Vec<(i64, Track)> = tracks
        .iter()
        .filter_map(|t| score_track_unified(query, t).map(|s| (s, t.clone())))
        .collect();

    scored.sort_by_key(|a| std::cmp::Reverse(a.0));

    let has_strong_match = scored.iter().any(|(s, _)| *s >= 30_000);
    if has_strong_match {
        scored.retain(|(s, _)| *s >= 30_000);
    }

    scored.into_iter().map(|(_, t)| t).collect()
}

pub fn handle_unified_search(
    tracks: &[Track],
    query: &str,
    cfg: &Config,
    args: &Cli,
    extra_mpv_args: &[String],
) -> Result<()> {
    let candidates = resolve_unified_candidates(tracks, query);

    if candidates.is_empty() {
        ui::warning("No match.");
        return Ok(());
    }

    if candidates.len() == 1 {
        log::info!("Single match found. Playing directly.");
        player::play(&candidates[0].path, cfg, extra_mpv_args)?;
        return Ok(());
    }

    ui::info(format!("Found {} matching tracks.", candidates.len()));
    if args.play_all {
        let paths: Vec<String> = candidates.iter().map(|t| t.path.clone()).collect();
        player::play_files(&paths, cfg, extra_mpv_args)?;
    } else {
        tui::run_track_mode(&candidates, cfg, extra_mpv_args)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_track(title: &str, artist: &str, album: &str) -> Track {
        Track {
            path: format!("/music/{}.mp3", title),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            genre: "Genre".to_string(),
            mtime: 0,
            size: 0,
            media_type: crate::indexer::MediaType::Audio,
        }
    }

    #[test]
    fn test_is_direct_playback_target() {
        assert!(is_direct_playback_target("https://example.com/song.mp3"));
        assert!(is_direct_playback_target("http://radio.stream"));
        assert!(is_direct_playback_target("https://youtu.be/abc"));
        assert!(!is_direct_playback_target("dean july"));
        assert!(!is_direct_playback_target("controversy"));
    }

    #[test]
    fn test_score_track_unified_multi_field() {
        let track1 = sample_track("July", "DEAN", "130 mood");
        let track2 = sample_track("July", "Noah Cyrus", "NC-17");

        assert!(score_track_unified("dean july", &track1).is_some());
        assert!(score_track_unified("july dean", &track1).is_some());
        assert!(score_track_unified("dean july", &track2).is_none());
    }

    #[test]
    fn test_resolve_unified_candidates_ranking() {
        let tracks = vec![
            sample_track("And July", "Heize", "And July (feat. DEAN)"),
            sample_track("July", "DEAN", "130 mood"),
            sample_track("Juicy", "Doja Cat", "Hot Pink"),
        ];

        let candidates = resolve_unified_candidates(&tracks, "july");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].title, "July");
        assert_eq!(candidates[1].title, "And July");
    }
}
