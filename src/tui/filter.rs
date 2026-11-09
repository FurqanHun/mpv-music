use crate::cli::Cli;
use crate::indexer;

pub fn parse_filter_terms(val: &str) -> Vec<String> {
    let or_normalized = val
        .replace("||", "|")
        .replace(" OR ", "|")
        .replace(',', "|");

    or_normalized
        .split('|')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(crate::search::norm::normalize_search_text)
        .collect()
}

pub fn apply_cli_filters(
    tracks: &[indexer::Track],
    args: &Cli,
    exact: bool,
) -> Vec<indexer::Track> {
    let prepare_terms = |arg: &Option<Option<String>>| -> Option<Vec<String>> {
        arg.as_ref()
            .and_then(|opt| opt.as_ref())
            .map(|val| parse_filter_terms(val))
    };

    let genre_terms = prepare_terms(&args.genre);
    let artist_terms = prepare_terms(&args.artist);
    let album_terms = prepare_terms(&args.album);
    let title_terms = prepare_terms(&args.title);

    let has_any_filter = genre_terms.is_some()
        || artist_terms.is_some()
        || album_terms.is_some()
        || title_terms.is_some();

    if !has_any_filter {
        return tracks.to_vec();
    }

    tracks
        .iter()
        .filter(|t| {
            let matches_field = |field: &str, terms: &Option<Vec<String>>| {
                if let Some(search_vals) = terms {
                    if search_vals.is_empty() {
                        return false;
                    }
                    let field_norm = crate::search::norm::normalize_search_text(field);
                    let field_clean_norm = crate::search::norm::normalize_search_text(
                        &crate::search::norm::clean_noise(field),
                    );

                    if exact {
                        search_vals.iter().any(|term| {
                            field_norm
                                .split([';', ','])
                                .map(|s| s.trim())
                                .any(|tag| tag == term)
                                || field_clean_norm
                                    .split([';', ','])
                                    .map(|s| s.trim())
                                    .any(|tag| tag == term)
                        })
                    } else {
                        search_vals.iter().any(|term| {
                            field_norm.contains(term)
                                || field_clean_norm.contains(term)
                                || crate::search::norm::match_all_tokens_normalized(term, field)
                        })
                    }
                } else {
                    true
                }
            };

            if args.or {
                genre_terms
                    .as_ref()
                    .is_some_and(|_| matches_field(&t.genre, &genre_terms))
                    || artist_terms
                        .as_ref()
                        .is_some_and(|_| matches_field(&t.artist, &artist_terms))
                    || album_terms
                        .as_ref()
                        .is_some_and(|_| matches_field(&t.album, &album_terms))
                    || title_terms
                        .as_ref()
                        .is_some_and(|_| matches_field(&t.title, &title_terms))
            } else {
                matches_field(&t.genre, &genre_terms)
                    && matches_field(&t.artist, &artist_terms)
                    && matches_field(&t.album, &album_terms)
                    && matches_field(&t.title, &title_terms)
            }
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn sample_track(title: &str, artist: &str, album: &str, genre: &str) -> indexer::Track {
        indexer::Track {
            path: format!("/music/{}.mp3", title),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            genre: genre.to_string(),
            mtime: 0,
            size: 0,
            media_type: indexer::MediaType::Audio,
        }
    }

    #[test]
    fn test_apply_cli_filters_accents_and_fullwidth() {
        let tracks = vec![
            sample_track("Café Song", "Beyoncé", "Renaissance", "Pop"),
            sample_track("Ｊｕｌｙ", "DEAN", "130 mood", "R&B"),
            sample_track("Don’t Speak", "No Doubt", "Tragic Kingdom", "Rock"),
        ];

        let args = Cli::parse_from(["mpv-music", "-t", "cafe"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "Café Song");

        let args = Cli::parse_from(["mpv-music", "-t", "july"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "Ｊｕｌｙ");

        let args = Cli::parse_from(["mpv-music", "-t", "don't"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "Don’t Speak");

        let args = Cli::parse_from(["mpv-music", "-t", "dont"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "Don’t Speak");
    }

    #[test]
    fn test_apply_cli_filters_clean_noise() {
        let tracks = vec![
            sample_track("July (feat. DEAN)", "Heize", "And July", "R&B"),
            sample_track(
                "Creep [Remastered 2021]",
                "Radiohead",
                "Pablo Honey",
                "Rock",
            ),
        ];

        let args = Cli::parse_from(["mpv-music", "-t", "july"]);
        let res = apply_cli_filters(&tracks, &args, true);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "July (feat. DEAN)");

        let args = Cli::parse_from(["mpv-music", "-t", "creep"]);
        let res = apply_cli_filters(&tracks, &args, true);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "Creep [Remastered 2021]");
    }

    #[test]
    fn test_apply_cli_filters_multi_word() {
        let tracks = vec![
            sample_track("July (feat. DEAN)", "Heize", "And July", "R&B"),
            sample_track("And July", "Heize", "And July", "R&B"),
        ];

        let args = Cli::parse_from(["mpv-music", "-t", "dean july"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "July (feat. DEAN)");

        let args = Cli::parse_from(["mpv-music", "-t", "july dean"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "July (feat. DEAN)");
    }

    #[test]
    fn test_apply_cli_filters_typo_fuzzy() {
        let tracks = vec![
            sample_track("July", "DEAN", "130 mood", "R&B"),
            sample_track("And July", "Heize", "And July", "R&B"),
            sample_track("Different Song", "Artist", "Album", "Pop"),
        ];

        let args = Cli::parse_from(["mpv-music", "-t", "jlu"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 2);
        assert!(res.iter().any(|t| t.title == "July"));
        assert!(res.iter().any(|t| t.title == "And July"));

        let args = Cli::parse_from(["mpv-music", "-t", "ujuly and"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "And July");

        let args = Cli::parse_from(["mpv-music", "-t", "and ujuly"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "And July");

        let args = Cli::parse_from(["mpv-music", "-t", "ujuly adn"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].title, "And July");

        let args = Cli::parse_from(["mpv-music", "-t", "unjuly"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 2);
        assert!(res.iter().any(|t| t.title == "July"));
        assert!(res.iter().any(|t| t.title == "And July"));
    }

    #[test]
    fn test_apply_cli_filters_same_flag_or_aliases() {
        let tracks = vec![
            sample_track("Show", "Ado", "Kyogen", "J-Pop"),
            sample_track("Overdose", "Natori", "Theater", "Classical"),
            sample_track("Creep", "Radiohead", "Pablo Honey", "Rock"),
        ];

        let args = Cli::parse_from(["mpv-music", "-a", "Ado || Natori"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 2);
        assert!(res.iter().any(|t| t.artist == "Ado"));
        assert!(res.iter().any(|t| t.artist == "Natori"));

        let args = Cli::parse_from(["mpv-music", "-a", "Ado | Natori"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 2);

        let args = Cli::parse_from(["mpv-music", "-a", "Ado, Natori"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 2);

        let args = Cli::parse_from(["mpv-music", "-a", "Ado OR Natori"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 2);

        let args = Cli::parse_from(["mpv-music", "-g", "J-Pop | Rock"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 2);
        assert!(res.iter().any(|t| t.genre == "J-Pop"));
        assert!(res.iter().any(|t| t.genre == "Rock"));

        let args = Cli::parse_from(["mpv-music", "-t", "Show OR Creep"]);
        let res = apply_cli_filters(&tracks, &args, false);
        assert_eq!(res.len(), 2);
        assert!(res.iter().any(|t| t.title == "Show"));
        assert!(res.iter().any(|t| t.title == "Creep"));
    }

    #[test]
    fn test_apply_cli_filters_multi_flag_and_vs_or() {
        let tracks = vec![
            sample_track("Show", "Ado", "Kyogen", "J-Pop"),
            sample_track("Creep", "Radiohead", "Pablo Honey", "Rock"),
            sample_track("Karma", "Radiohead", "OK Computer", "Alternative"),
        ];

        let args_and = Cli::parse_from(["mpv-music", "-a", "Radiohead", "-g", "Rock"]);
        let res_and = apply_cli_filters(&tracks, &args_and, false);
        assert_eq!(res_and.len(), 1);
        assert_eq!(res_and[0].title, "Creep");

        let args_or = Cli::parse_from(["mpv-music", "-a", "Ado", "-g", "Rock", "--or"]);
        let res_or = apply_cli_filters(&tracks, &args_or, false);
        assert_eq!(res_or.len(), 2);
        assert!(res_or.iter().any(|t| t.title == "Show"));
        assert!(res_or.iter().any(|t| t.title == "Creep"));
    }
}
