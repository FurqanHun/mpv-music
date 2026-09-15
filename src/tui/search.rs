use super::icons::Icons;
use super::items::SearchItem;
use super::runner::run_skim_input_prompt;
use crate::config;
use crate::player;
use crate::search;
use anyhow::Result;
use skim::prelude::*;

pub fn run_search_mode(
    cfg: &config::Config,
    initial_query: Option<String>,
    extra_args: &[String],
) -> Result<()> {
    if !cfg.ytdlp_available {
        let ytdlp_cmd = cfg.ytdlp_bin();
        eprintln!(
            "\n\x1b[33;1m[Warning]\x1b[0m Feature unavailable: '{}' not found.",
            ytdlp_cmd
        );
        eprintln!(
            "mpv-music requires '{}' to use Search and Streaming.",
            ytdlp_cmd
        );
        if ytdlp_cmd != "yt-dlp" {
            eprintln!("Check your 'ytdlp' setting in config.toml or the --ytdlp CLI option.");
        } else {
            eprintln!("Please install 'yt-dlp' to use Search and Streaming.");
        }
        return Ok(());
    }

    let icons = Icons::new(cfg.nerd_fonts);
    let query = if let Some(q) = initial_query {
        q
    } else {
        let prompt = format!("{}Search / URL > ", icons.pad(icons.search()));
        let header = "Search YouTube or paste URL (press ENTER on empty to go back).";
        match run_skim_input_prompt(&prompt, header) {
            Some(q) => q,
            None => return Ok(()),
        }
    };

    if query.is_empty() {
        return Ok(());
    }

    if query.starts_with("http") {
        log::info!("Direct URL detected, playing...");
        player::play(&query, cfg, extra_args)?;
        return Ok(());
    }

    println!("Fetching results for '{}'...", query);
    let results = search::search_youtube(&query, 25, cfg.ytdlp_bin())?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    let skim_items: Vec<SearchItem> = results
        .into_iter()
        .map(|r| SearchItem {
            result: r,
            playlist_icon: icons.playlist(),
            video_icon: icons.video_stream(),
        })
        .collect();

    let search_prompt = icons.prompt(icons.target_search(), "Search");
    let opts = SkimOptionsBuilder::default()
        .height("100%")
        .multi(true)
        .prompt(&search_prompt)
        .reverse(true)
        //.typos(2)
        .inline_info(true)
        .preview("")
        .build()
        .unwrap();

    if let Ok(output) = Skim::run_items(opts, skim_items) {
        if output.is_abort {
            return Ok(());
        }

        let selected_urls: Vec<String> = output
            .selected_items
            .iter()
            .map(|item| item.output().to_string())
            .collect();

        if !selected_urls.is_empty() {
            if selected_urls.len() == 1 {
                player::play(&selected_urls[0], cfg, extra_args)?;
            } else {
                log::info!("Playing queue of {} tracks", selected_urls.len());
                player::play_files(&selected_urls, cfg, extra_args)?;
            }
        }
    }
    Ok(())
}
