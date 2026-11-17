mod client;
mod comet;
mod config;
mod error;
mod model;
mod render;

use chrono::{DateTime, Utc};
use clap::{CommandFactory, Parser, Subcommand};
use client::Client;
use comet::Comet;
use config::{Credentials, Keys};
use error::{Error, Result};
use model::{PlurkList, User};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value_os_t = config::default_path())]
    key_file: PathBuf,
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

pub fn format_offset(t: &DateTime<Utc>) -> String {
    t.format("%Y-%-m-%-dT%H:%M:%S").to_string()
}

async fn timeline(client: &Client, verbose: bool, limit: u64) -> Result<()> {
    let since = Utc::now() - chrono::Duration::days(1);
    let list: PlurkList = client
        .call(
            "/Polling/getPlurks",
            &[
                ("offset", format_offset(&since)),
                ("limit", limit.to_string()),
            ],
        )
        .await?;
    for p in &list.plurks {
        render::print_plurk_line(client, &list.plurk_users, p, verbose).await;
    }
    Ok(())
}

async fn comet_loop(client: &Client) -> Result<()> {
    let mut comet = Comet::connect(client).await?;
    let mut delay = Duration::from_secs(1);
    loop {
        match comet.poll().await {
            Ok(events) => {
                delay = Duration::from_secs(1);
                for e in &events {
                    render::print_event(client, e).await;
                }
            }
            Err(Error::Resync) => {
                comet = Comet::connect(client).await?;
            }
            Err(e) => {
                eprintln!("{e}; retrying in {}s", delay.as_secs());
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(60));
            }
        }
    }
}

async fn run(cli: Cli) -> Result<()> {
    if let Some(Commands::GenKey {
        consumer_key,
        consumer_secret,
        token_key,
        token_secret,
    }) = cli.command
    {
        let token = match (token_key, token_secret) {
            (Some(key), Some(secret)) => Some(Keys { key, secret }),
            _ => None,
        };
        let creds = Credentials::new(
            Keys {
                key: consumer_key,
                secret: consumer_secret,
            },
            token,
        );
        return creds.save(&cli.key_file);
    }

    let mut client = Client::new(Credentials::load(&cli.key_file)?);
    if client.creds.token().is_none() {
        client.acquire_token().await?;
        client.creds.save(&cli.key_file)?;
    }

    match cli.command {
        Some(Commands::Me) => {
            let me: User = client.call("/Users/me", &[]).await?;
            render::print_me(&me);
        }
        Some(Commands::Comet) => {
            println!("Polling Comet...ctrl+c to exit");
            tokio::select! {
                r = comet_loop(&client) => r?,
                _ = tokio::signal::ctrl_c() => {}
            }
        }
        Some(Commands::Timeline { verbose, limit }) => timeline(&client, verbose, limit).await?,
        _ => {
            let _ = Cli::command().print_help();
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn offset_format_per_spec() {
        let t = Utc.with_ymd_and_hms(2009, 6, 20, 21, 55, 34).unwrap();
        assert_eq!(format_offset(&t), "2009-6-20T21:55:34");
        let t = Utc.with_ymd_and_hms(2024, 12, 1, 1, 2, 3).unwrap();
        assert_eq!(format_offset(&t), "2024-12-1T01:02:03");
    }

    #[test]
    fn gen_key_honors_key_file() {
        let dir = std::env::temp_dir().join(format!("plurk-cli-genkey-{}", std::process::id()));
        let path = dir.join("sub").join("k.toml");
        let cli = Cli::parse_from(["plurk", "-k", path.to_str().unwrap(), "gen-key", "ck", "cs"]);
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(run(cli))
            .unwrap();
        assert_eq!(Credentials::load(&path).unwrap().consumer.key, "ck");
        let _ = std::fs::remove_dir_all(dir);
    }
}
