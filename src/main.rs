// plurk.rs
// Copyright (C) 2022 dephilia <me@dephilia.moe>
// Distributed under terms of the MIT license.

mod app;
mod comet;
mod error;
mod plurk;
mod utils;

use app::*;
use clap::{CommandFactory, Parser, Subcommand};
use error::PlurkError;
use plurk::Plurk;
use tokio;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value_t = get_key_file_string())]
    key_file: String,
}

#[derive(Subcommand)]
enum Commands {
    GenKey {
        consumer_key: String,
        consumer_secret: String,
        token_key: Option<String>,
        token_secret: Option<String>,
    },
    Comet,

    Me,

    Timeline {
        #[arg(short, long)]
        verbose: bool,
        #[arg(short, long, default_value_t = 20)]
        limit: u64,
    },
}

#[tokio::main]
async fn main() -> Result<(), PlurkError> {
    let cli = Cli::parse();

    /* Gen Key need to put here */
    if let Some(Commands::GenKey {
        consumer_key,
        consumer_secret,
        token_key,
        token_secret,
    }) = &cli.command
    {
        gen_key_file(
            consumer_key.clone(),
            consumer_secret.clone(),
            token_key.clone(),
            token_secret.clone(),
        )?;
        return Ok(());
    }

    let mut plurk = Plurk::from_toml(&cli.key_file)?;

    if !plurk.has_token() {
        plurk.acquire_plurk_key().await?;
        plurk.to_toml(&cli.key_file)?;
    }

    match &cli.command {
        Some(Commands::GenKey { .. }) => {
            // Bypass here
        }
        Some(Commands::Me) => {
            print_me(plurk.clone()).await?;
        }
        Some(Commands::Comet) => {
            poll_comet(plurk.clone()).await?;
        }
        Some(Commands::Timeline { verbose, limit }) => {
            print_timeline(plurk.clone(), verbose.clone(), limit.clone()).await?;
        }
        None => {
            let mut cmd = Cli::command();
            cmd.print_help().unwrap();
        }
    }

    Ok(())
}
