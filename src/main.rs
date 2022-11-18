// plurk.rs
// Copyright (C) 2022 dephilia <me@dephilia.moe>
// Distributed under terms of the MIT license.

mod app;
mod comet;
mod error;
mod plurk;
mod utils;

use app::*;
use clap::CommandFactory;
use clap::Parser;
use error::PlurkError;
use plurk::Plurk;
use tokio;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("key.toml"))]
    key_file: String,

    #[arg(short, long, default_value_t = false)]
    comet: bool,

    #[arg(short, long, default_value_t = false)]
    me: bool,

    #[arg(short, long, default_value_t = false)]
    timeline: bool,

    #[arg(short, long, default_value_t = false, requires = "timeline")]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), PlurkError> {
    let args = Args::parse();

    let mut plurk = Plurk::from_toml(&args.key_file)?;

    if !plurk.has_token() {
        plurk.acquire_plurk_key().await?;
        plurk.to_toml(&args.key_file)?;
    }

    if args.me {
        print_me(plurk.clone()).await?;
        return Ok(());
    }

    if args.comet {
        poll_comet(plurk.clone()).await?;
        return Ok(());
    }

    if args.timeline {
        print_timeline(plurk.clone(), args.verbose).await?;
        return Ok(());
    }

    let mut cmd = Args::command();
    cmd.print_help().unwrap();

    Ok(())
}
