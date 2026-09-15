use crate::cli::Cli;
use crate::config::Config;
use crate::indexer::Track;
use crate::player;
use crate::tui;
use anyhow::Result;
use std::collections::HashSet;

pub fn is_multi_value_search(args: &Cli) -> bool {
    let check_comma = |arg: &Option<Option<String>>| -> bool {
        arg.as_ref()
            .and_then(|o| o.as_ref())
            .is_some_and(|s| s.contains(','))
    };

    check_comma(&args.artist)
        || check_comma(&args.genre)
        || check_comma(&args.album)
        || check_comma(&args.title)
}

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

pub fn handle_cli_filters(
    tracks: &[Track],
    cfg: &Config,
    args: &Cli,
    extra_mpv_args: &[String],
) -> Result<()> {
    let is_multi = is_multi_value_search(args);

    // stage 1: exact match
    let mut filtered = if !is_multi {
        tui::apply_cli_filters(tracks, args, true)
    } else {
        Vec::new()
    };

    // stage 2: partial match / ambiguity handling
    if filtered.is_empty() {
        log::debug!("Exact match skipped or failed, trying partial...");
        let partials = tui::apply_cli_filters(tracks, args, false);

        if partials.is_empty() {
            eprintln!("\x1b[33;1m[Warning]\x1b[0m No match.");
            return Ok(());
        }

        // identify active tag
        let mut unique_options: HashSet<String> = HashSet::new();
        let field_opt = active_filter_field(args);

        if let Some(field) = field_opt {
            for t in &partials {
                unique_options.insert(field.extract(t).to_string());
            }

            if !args.play_all && unique_options.len() > 1 {
                let mut options_vec: Vec<String> = unique_options.into_iter().collect();
                options_vec.sort();

                if let Some(selected_vals) = tui::run_skim_multi_selection(
                    options_vec,
                    &format!("Which {}s? (TAB to select multiple) > ", field.as_str()),
                ) {
                    let selected_set: HashSet<String> = selected_vals.into_iter().collect();

                    filtered = partials
                        .into_iter()
                        .filter(|t| selected_set.contains(field.extract(t)))
                        .collect();
                } else {
                    return Ok(());
                }
            } else {
                filtered = partials;
            }
        } else {
            filtered = partials;
        }
    }

    if filtered.is_empty() {
        eprintln!("\x1b[33;1m[Warning]\x1b[0m No match.");
        return Ok(());
    }

    if filtered.len() == 1 {
        log::info!("Single match found. Playing directly.");
        player::play(&filtered[0].path, cfg, extra_mpv_args)?;
        return Ok(());
    }

    println!("Found {} matching tracks.", filtered.len());
    if args.play_all || args.title.is_some() {
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

    #[test]
    fn test_is_multi_value_search() {
        let args_single = Cli::parse_from(["mpv-music", "--artist", "queen"]);
        assert!(!is_multi_value_search(&args_single));

        let args_multi = Cli::parse_from(["mpv-music", "--artist", "queen,bowie"]);
        assert!(is_multi_value_search(&args_multi));
    }

    #[test]
    fn test_active_filter_field() {
        let args_artist = Cli::parse_from(["mpv-music", "--artist", "queen"]);
        assert_eq!(active_filter_field(&args_artist), Some(TagField::Artist));

        let args_genre = Cli::parse_from(["mpv-music", "--genre", "rock"]);
        assert_eq!(active_filter_field(&args_genre), Some(TagField::Genre));

        let args_album = Cli::parse_from(["mpv-music", "--album", "innuendo"]);
        assert_eq!(active_filter_field(&args_album), Some(TagField::Album));

        let args_title = Cli::parse_from(["mpv-music", "--title", "bohemian"]);
        assert_eq!(active_filter_field(&args_title), Some(TagField::Title));

        let args_none = Cli::parse_from(["mpv-music"]);
        assert_eq!(active_filter_field(&args_none), None);
    }
}
