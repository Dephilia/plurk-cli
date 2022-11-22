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
    #[arg(short, long, default_value_t = get_key_file_string())]
    key_file: String,

    #[arg(short, long, default_value_t = false)]
    gen_key: bool,

    #[arg(long)]
    consumer_key: Option<String>,
    #[arg(long)]
    consumer_secret: Option<String>,
    #[arg(long)]
    token_key: Option<String>,
    #[arg(long)]
    token_secret: Option<String>,

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

    if args.gen_key {
        if let (Some(ck), Some(cs)) = (args.consumer_key, args.consumer_secret) {
            gen_key_file(ck, cs, args.token_key, args.token_secret)?;
        } else {
            println!("Missing argument: --consumer-key, --consumer-secret");
        }
        return Ok(());
    }

    if !get_key_file()?.as_path().exists() {
        println!("Config file not exist.");
        return Ok(());
    }

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
