use crate::cli::Cli;
use crate::indexer;

pub fn apply_cli_filters(
    tracks: &[indexer::Track],
    args: &Cli,
    exact: bool,
) -> Vec<indexer::Track> {
    let prepare_terms = |arg: &Option<Option<String>>| -> Option<Vec<String>> {
        arg.as_ref().and_then(|opt| opt.as_ref()).map(|val| {
            val.to_lowercase()
                .split(',')
                .map(|s| s.trim().to_string())
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
                    let field_lower = field.to_lowercase();

                    if exact {
                        search_vals.iter().any(|term| {
                            field_lower
                                .split([';', ','])
                                .map(|s| s.trim())
                                .any(|tag| tag == term)
                        })
                    } else {
                        search_vals.iter().any(|term| field_lower.contains(term))
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
