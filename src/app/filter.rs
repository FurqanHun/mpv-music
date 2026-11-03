use crate::cli::Cli;
use crate::config::Config;
use crate::indexer::Track;
use crate::player;
use crate::tui;
use crate::ui;
use anyhow::Result;
use std::collections::HashSet;

pub fn has_filter_flags(args: &Cli) -> bool {
    args.genre.is_some() || args.artist.is_some() || args.album.is_some() || args.title.is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagField {
    Artist,
    Genre,
    Album,
    Title,
}

impl TagField {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Artist => "artist",
            Self::Genre => "genre",
            Self::Album => "album",
            Self::Title => "title",
        }
    }

    pub fn extract<'a>(&self, track: &'a Track) -> &'a str {
        match self {
            Self::Artist => &track.artist,
            Self::Genre => &track.genre,
            Self::Album => &track.album,
            Self::Title => &track.title,
        }
    }
}

pub fn active_filter_field(args: &Cli) -> Option<TagField> {
    if args.artist.is_some() {
        Some(TagField::Artist)
    } else if args.genre.is_some() {
        Some(TagField::Genre)
    } else if args.album.is_some() {
        Some(TagField::Album)
    } else if args.title.is_some() {
        Some(TagField::Title)
    } else {
        None
    }
}

pub fn resolve_cli_candidates(tracks: &[Track], args: &Cli) -> Vec<Track> {
    let mut filtered = tui::apply_cli_filters(tracks, args, false);

    if let Some(field) = active_filter_field(args) {
        let query_str = match field {
            TagField::Artist => args.artist.as_ref().and_then(|o| o.as_ref()),
            TagField::Genre => args.genre.as_ref().and_then(|o| o.as_ref()),
            TagField::Album => args.album.as_ref().and_then(|o| o.as_ref()),
            TagField::Title => args.title.as_ref().and_then(|o| o.as_ref()),
        };

        if let Some(query) = query_str {
            crate::search::norm::sort_by_relevance(&mut filtered, query, |t| field.extract(t));

            let has_strong_match = filtered.iter().any(|t| {
                crate::search::norm::score_match(query, field.extract(t)).unwrap_or(i64::MIN)
                    >= 9_000
            });

            if has_strong_match {
                filtered.retain(|t| {
                    crate::search::norm::score_match(query, field.extract(t)).unwrap_or(i64::MIN)
                        >= 9_000
                });
            }
        }
    }

    filtered
}

pub fn handle_cli_filters(
    tracks: &[Track],
    cfg: &Config,
    args: &Cli,
    extra_mpv_args: &[String],
) -> Result<()> {
    let mut filtered = resolve_cli_candidates(tracks, args);

    if filtered.is_empty() {
        ui::warning("No match.");
        return Ok(());
    }

    if let Some(field) = active_filter_field(args) {
        if field == TagField::Title {
            if filtered.len() == 1 {
                log::info!("Single match found. Playing directly.");
                player::play(&filtered[0].path, cfg, extra_mpv_args)?;
                return Ok(());
            }

            ui::info(format!("Found {} matching tracks.", filtered.len()));
            if args.play_all {
                let paths: Vec<String> = filtered.iter().map(|t| t.path.clone()).collect();
                player::play_files(&paths, cfg, extra_mpv_args)?;
            } else {
                tui::run_track_mode(&filtered, cfg, extra_mpv_args)?;
            }
            return Ok(());
        }

        let mut seen = HashSet::new();
        let mut unique_options: Vec<String> = Vec::new();
        for t in &filtered {
            let val = field.extract(t).to_string();
            if seen.insert(val.clone()) {
                unique_options.push(val);
            }
        }

        if !args.play_all && unique_options.len() > 1 {
            if let Some(selected_vals) = tui::run_skim_multi_selection(
                unique_options,
                &format!("Which {}s? (TAB select, Ctrl+A all) > ", field.as_str()),
            ) {
                let selected_set: HashSet<String> = selected_vals.into_iter().collect();
                filtered.retain(|t| selected_set.contains(field.extract(t)));
            } else {
                return Ok(());
            }
        }
    }

    if filtered.is_empty() {
        ui::warning("No match.");
        return Ok(());
    }

    if filtered.len() == 1 {
        log::info!("Single match found. Playing directly.");
        player::play(&filtered[0].path, cfg, extra_mpv_args)?;
        return Ok(());
    }

    ui::info(format!("Found {} matching tracks.", filtered.len()));
    if args.play_all {
        let paths: Vec<String> = filtered.iter().map(|t| t.path.clone()).collect();
        player::play_files(&paths, cfg, extra_mpv_args)?;
    } else {
        tui::run_post_filter_action(&filtered, cfg, extra_mpv_args)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn sample_track(title: &str, artist: &str, album: &str, genre: &str) -> Track {
        Track {
            path: format!("/music/{}.mp3", title),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            genre: genre.to_string(),
            mtime: 0,
            size: 0,
            media_type: crate::indexer::MediaType::Audio,
        }
    }

    #[test]
    fn test_resolve_cli_candidates_prevents_exact_shadowing() {
        let tracks = vec![
            sample_track("And July", "Heize", "And July", "R&B"),
            sample_track("July", "DEAN", "130 mood", "R&B"),
            sample_track("Something Else", "Artist", "Album", "Pop"),
        ];

        let args = Cli::parse_from(["mpv-music", "-t", "july"]);
        let candidates = resolve_cli_candidates(&tracks, &args);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].title, "July");
        assert_eq!(candidates[1].title, "And July");
    }

    #[test]
    fn test_resolve_cli_candidates_single_match() {
        let tracks = vec![
            sample_track("July", "DEAN", "130 mood", "R&B"),
            sample_track("Radiohead - Creep", "Radiohead", "Pablo Honey", "Rock"),
        ];

        let args = Cli::parse_from(["mpv-music", "-t", "creep"]);
        let candidates = resolve_cli_candidates(&tracks, &args);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title, "Radiohead - Creep");
    }

    #[test]
    fn test_resolve_cli_candidates_prunes_fuzzy_noise_when_strong_match_exists() {
        let tracks = vec![
            sample_track("July", "DEAN", "130 mood", "R&B"),
            sample_track("Juicy", "Doja Cat", "Hot Pink", "Pop"),
            sample_track("Something Else", "Artist", "Album", "Pop"),
        ];

        let args = Cli::parse_from(["mpv-music", "-t", "july"]);
        let candidates = resolve_cli_candidates(&tracks, &args);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title, "July");
    }

    #[test]
    fn test_resolve_cli_candidates_keeps_fuzzy_matches_when_no_strong_match() {
        let tracks = vec![
            sample_track("Juli", "Artist A", "Album A", "Pop"),
            sample_track("Jule", "Artist B", "Album B", "Pop"),
            sample_track("Radiohead - Creep", "Radiohead", "Pablo Honey", "Rock"),
        ];

        let args = Cli::parse_from(["mpv-music", "-t", "july"]);
        let candidates = resolve_cli_candidates(&tracks, &args);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].title, "Juli");
        assert_eq!(candidates[1].title, "Jule");
    }

    #[test]
    fn test_resolve_cli_candidates_insertion_typo() {
        let tracks = vec![
            sample_track("I'm a Controversy", "Ado", "Album", "Pop"),
            sample_track("Radiohead - Creep", "Radiohead", "Pablo Honey", "Rock"),
        ];

        let args = Cli::parse_from(["mpv-music", "-t", "controeversy"]);
        let candidates = resolve_cli_candidates(&tracks, &args);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title, "I'm a Controversy");
    }
}
