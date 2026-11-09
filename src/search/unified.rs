use super::norm::{normalize_search_text, score_match_normalized};
use crate::cli::Cli;
use crate::config::Config;
use crate::indexer::Track;
use crate::player;
use crate::tui;
use crate::ui;
use anyhow::Result;

pub fn parse_query_branches(query: &str) -> Vec<Vec<String>> {
    let or_normalized = query
        .replace("||", "|")
        .replace(" OR ", "|")
        .replace(',', "|");

    or_normalized
        .split('|')
        .map(|branch| branch.trim())
        .filter(|branch| !branch.is_empty())
        .map(|branch| {
            let and_normalized = branch
                .replace("&&", " ")
                .replace(" AND ", " ")
                .replace(" & ", " ")
                .replace(';', " ");

            and_normalized
                .split_whitespace()
                .map(|token| token.to_string())
                .collect()
        })
        .filter(|tokens: &Vec<String>| !tokens.is_empty())
        .collect()
}

#[derive(Debug, Clone)]
struct NormalizedBranch {
    tokens: Vec<String>,
    branch_len: usize,
}

impl NormalizedBranch {
    fn from_raw(branch: &[String]) -> Self {
        let mut tokens = Vec::with_capacity(branch.len());
        let mut branch_len = 0;
        for token in branch {
            let norm = normalize_search_text(token);
            if !norm.is_empty() {
                branch_len += norm.len();
                tokens.push(norm);
            }
        }
        Self { tokens, branch_len }
    }
}

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

fn score_normalized_branch(branch: &NormalizedBranch, track: &Track) -> Option<i64> {
    let mut branch_score = 0;

    for norm_token in &branch.tokens {
        let title_score = score_match_normalized(norm_token, &track.title).map(|s| s + 20_000);
        let artist_score = score_match_normalized(norm_token, &track.artist).map(|s| s + 10_000);
        let album_score = score_match_normalized(norm_token, &track.album);

        let best_token_score = match (title_score, artist_score, album_score) {
            (None, None, None) => return None,
            (s1, s2, s3) => s1.into_iter().chain(s2).chain(s3).max().unwrap(),
        };

        branch_score += best_token_score;
    }

    let total_len = (track.title.len() + track.artist.len() + track.album.len()) as i64;
    branch_score -= total_len.saturating_sub(branch.branch_len as i64).min(100);

    Some(branch_score)
}

pub fn score_branch(branch: &[String], track: &Track) -> Option<i64> {
    let norm_branch = NormalizedBranch::from_raw(branch);
    score_normalized_branch(&norm_branch, track)
}

pub fn score_track_unified(query: &str, track: &Track) -> Option<i64> {
    let raw_branches = parse_query_branches(query);
    if raw_branches.is_empty() {
        return Some(0);
    }
    let branches: Vec<NormalizedBranch> = raw_branches
        .iter()
        .map(|b| NormalizedBranch::from_raw(b))
        .collect();

    branches
        .iter()
        .filter_map(|branch| score_normalized_branch(branch, track))
        .max()
}

pub fn resolve_unified_candidates(tracks: &[Track], query: &str) -> Vec<Track> {
    let raw_branches = parse_query_branches(query);
    if raw_branches.is_empty() {
        return Vec::new();
    }
    let branches: Vec<NormalizedBranch> = raw_branches
        .iter()
        .map(|b| NormalizedBranch::from_raw(b))
        .collect();

    let branch_has_strong: Vec<bool> = branches
        .iter()
        .map(|branch| {
            tracks
                .iter()
                .any(|t| score_normalized_branch(branch, t).unwrap_or(i64::MIN) >= 30_000)
        })
        .collect();

    let mut scored: Vec<(i64, Track)> = tracks
        .iter()
        .filter_map(|t| {
            let mut best_score: Option<i64> = None;
            let mut keep = false;

            for (i, branch) in branches.iter().enumerate() {
                if let Some(s) = score_normalized_branch(branch, t) {
                    best_score = Some(best_score.map_or(s, |curr| curr.max(s)));
                    if branch_has_strong[i] {
                        if s >= 30_000 {
                            keep = true;
                        }
                    } else {
                        keep = true;
                    }
                }
            }

            if keep {
                best_score.map(|s| (s, t.clone()))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by_key(|a| std::cmp::Reverse(a.0));
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

    #[test]
    fn test_parse_query_branches_operators() {
        let parsed_pipe = parse_query_branches("natori | ado");
        assert_eq!(parsed_pipe, vec![vec!["natori"], vec!["ado"]]);

        let parsed_double_pipe = parse_query_branches("natori || ado");
        assert_eq!(parsed_double_pipe, vec![vec!["natori"], vec!["ado"]]);

        let parsed_or = parse_query_branches("natori OR ado");
        assert_eq!(parsed_or, vec![vec!["natori"], vec!["ado"]]);

        let parsed_comma = parse_query_branches("natori, ado");
        assert_eq!(parsed_comma, vec![vec!["natori"], vec!["ado"]]);

        let parsed_and_words = parse_query_branches("dean AND july");
        assert_eq!(parsed_and_words, vec![vec!["dean", "july"]]);

        let parsed_and_double = parse_query_branches("dean && july");
        assert_eq!(parsed_and_double, vec![vec!["dean", "july"]]);

        let parsed_and_amp = parse_query_branches("simon & garfunkel");
        assert_eq!(parsed_and_amp, vec![vec!["simon", "garfunkel"]]);

        let parsed_semicolon = parse_query_branches("vocella; natori");
        assert_eq!(parsed_semicolon, vec![vec!["vocella", "natori"]]);

        let parsed_embedded_amp = parse_query_branches("R&B | J-Pop");
        assert_eq!(parsed_embedded_amp, vec![vec!["R&B"], vec!["J-Pop"]]);
    }

    #[test]
    fn test_unified_search_or_matching() {
        let track_natori = sample_track("Overdose", "Natori", "Theater");
        let track_ado = sample_track("Show", "Ado", "Ado Collection");
        let track_mj = sample_track("Smooth Criminal", "Michael Jackson", "Bad");

        let tracks = vec![track_natori.clone(), track_ado.clone(), track_mj.clone()];

        for q in &[
            "natori | ado",
            "natori || ado",
            "natori OR ado",
            "natori, ado",
        ] {
            let candidates = resolve_unified_candidates(&tracks, q);
            assert_eq!(candidates.len(), 2, "Failed for query: {}", q);
            let titles: Vec<&str> = candidates.iter().map(|t| t.title.as_str()).collect();
            assert!(titles.contains(&"Overdose"));
            assert!(titles.contains(&"Show"));
            assert!(!titles.contains(&"Smooth Criminal"));
        }
    }

    #[test]
    fn test_unified_search_and_matching() {
        let track_dean = sample_track("July", "DEAN", "130 mood");
        let track_noah = sample_track("July", "Noah Cyrus", "NC-17");

        let tracks = vec![track_dean.clone(), track_noah.clone()];

        for q in &["dean july", "dean AND july", "dean && july"] {
            let candidates = resolve_unified_candidates(&tracks, q);
            assert_eq!(candidates.len(), 1, "Failed for query: {}", q);
            assert_eq!(candidates[0].artist, "DEAN");
        }

        let track_duo = sample_track("Overdose", "Vocella; Natori", "Collab");
        assert!(score_track_unified("vocella; natori", &track_duo).is_some());
    }

    #[test]
    fn test_unified_search_per_branch_pruning() {
        let track_natori = sample_track("Overdose", "Vocella; Natori", "Album");
        let track_gentle = sample_track("Ghosts", "The Gentle Men", "Album");
        let track_noise = sample_track("Other", "Gntle", "Album");

        let tracks = vec![track_natori, track_gentle, track_noise];
        let candidates = resolve_unified_candidates(&tracks, "notori || gentle");

        assert_eq!(candidates.len(), 2);
        let artists: Vec<&str> = candidates.iter().map(|t| t.artist.as_str()).collect();
        assert!(artists.contains(&"Vocella; Natori"));
        assert!(artists.contains(&"The Gentle Men"));
    }
}
