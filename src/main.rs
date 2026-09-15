mod app;
mod cli;
mod config;
mod dep_check;
mod indexer;
mod player;
mod radio;
mod search;
mod tui;
mod update;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let args = Cli::parse();
    app::run(args)
}
