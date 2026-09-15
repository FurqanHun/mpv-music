use crate::config;
use crate::player;
use crate::radio::RADIO_STATIONS;
use crate::tui::Icons;
use crate::tui::runner::run_skim_simple;
use anyhow::Result;

pub fn run_radio_mode(
    cfg: &config::Config,
    extra_args: &[String],
    filter: Option<&str>,
) -> Result<()> {
    let options: Vec<&str> = if let Some(f) = filter {
        let f_lower = f.to_lowercase().replace("-", "").replace(" ", "");
        RADIO_STATIONS
            .iter()
            .filter(|(n, url, _)| {
                !url.is_empty()
                    && n.to_lowercase()
                        .replace("-", "")
                        .replace(" ", "")
                        .contains(&f_lower)
            })
            .map(|(name, _, _)| *name)
            .collect()
    } else {
        RADIO_STATIONS.iter().map(|(name, _, _)| *name).collect()
    };

    if options.is_empty() {
        log::error!("No radio stations found matching filter: {:?}", filter);
        eprintln!(
            "\x1b[33;1m[Warning]\x1b[0m Radio station not found. Please use the interactive menu."
        );
        return Ok(());
    }

    if options.len() == 1 && filter.is_some() {
        let s = options[0];
        if let Some((name, url, _)) = RADIO_STATIONS.iter().find(|(n, _, _)| *n == s) {
            return player::play_radio(name, url, cfg, extra_args);
        }
    }

    let icons = Icons::new(cfg.nerd_fonts);
    let radio_prompt = format!(
        "{}Choose Station (Please consider donating!) > ",
        icons.pad(icons.radio())
    );
    let selected = run_skim_simple(options, &radio_prompt);

    if let Some(s) = selected.as_deref()
        && let Some((name, url, _is_listen_moe)) = RADIO_STATIONS.iter().find(|(n, _, _)| *n == s)
        && !url.is_empty()
    {
        player::play_radio(name, url, cfg, extra_args)?;
    }

    Ok(())
}
