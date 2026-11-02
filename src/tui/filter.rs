use crate::cli::Cli;
use crate::indexer;

pub fn apply_cli_filters(
    tracks: &[indexer::Track],
    args: &Cli,
    exact: bool,
) -> Vec<indexer::Track> {
    let prepare_terms = |arg: &Option<Option<String>>| -> Option<Vec<String>> {
        arg.as_ref().and_then(|opt| opt.as_ref()).map(|val| {
            val.split(',')
                .map(|s| crate::search_norm::normalize_search_text(s))
                .filter(|s| !s.is_empty())
                .collect()
        })
    };

    let genre_terms = prepare_terms(&args.genre);
    let artist_terms = prepare_terms(&args.artist);
    let album_terms = prepare_terms(&args.album);
    let title_terms = prepare_terms(&args.title);

    tracks
        .iter()
        .filter(|t| {
            let matches = |field: &str, terms: &Option<Vec<String>>| {
                if let Some(search_vals) = terms {
                    let field_norm = crate::search_norm::normalize_search_text(field);
                    let field_clean_norm =
                        crate::search_norm::normalize_search_text(&crate::search_norm::clean_noise(field));

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
                                || crate::search_norm::match_all_tokens(term, field)
                        })
                    }
                } else {
                    true
                }
            };

            matches(&t.genre, &genre_terms)
                && matches(&t.artist, &artist_terms)
                && matches(&t.album, &album_terms)
                && matches(&t.title, &title_terms)
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
            sample_track("Creep [Remastered 2021]", "Radiohead", "Pablo Honey", "Rock"),
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
    }
}
