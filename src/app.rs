// app.rs
// Copyright (C) 2022 dephilia <me@dephilia.moe>
// Distributed under terms of the MIT license.

use crate::comet::PlurkComet;
use crate::error::PlurkError;
use crate::plurk::{Plurk, PlurkData, PlurkUser};
use crate::utils::base36_encode;
use colored::Colorize;
use dirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use terminal_size::{terminal_size, Width};
use tokio::signal;

#[derive(Deserialize, Debug)]
struct ObjGetPlurks {
    plurks: Option<Vec<PlurkData>>,
    plurk_users: Option<HashMap<u64, PlurkUser>>,
}

pub async fn print_timeline(plurk: Plurk, verbose: bool) -> Result<(), PlurkError> {
    let now = chrono::offset::Utc::now();
    let time = now - chrono::Duration::days(1);
    let time = time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let resp = plurk
        .request_query("/APP/Polling/getPlurks", &[("offset", &time)])
        .await?;
    let body = resp
        .json::<ObjGetPlurks>()
        .await
        .map_err(|e| PlurkError::ParseError(e.to_string()))?;

    if let (Some(plurks), Some(plurk_users)) = (body.plurks, body.plurk_users) {
        for p in plurks {
            let display_name = &plurk_users
                .get(&p.owner_id)
                .ok_or(PlurkError::ParseError(p.owner_id.to_string()))?
                .display_name;
            if verbose {
                println!(
                    "Plurk ==> https://www.plurk.com/p/{}",
                    base36_encode(p.plurk_id)
                );
                println!(
                    "{} {} {}",
                    p.posted
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .bright_yellow(),
                    display_name.bold().bright_blue(),
                    p.qualifier.black().on_bright_white()
                );
                println!("{}", p.content_raw);
                if let Some((Width(w), _)) = terminal_size() {
                    println!("{}", "=".repeat(w.into()));
                }
            } else {
                println!(
                    "{} {} {} {}",
                    p.posted
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .bright_yellow(),
                    display_name.bold().bright_blue(),
                    p.qualifier.black().on_bright_white(),
                    p.content_raw.replace("\n", "   ")
                );
            }
        }
    }
    Ok(())
}

pub async fn print_me(plurk: Plurk) -> Result<(), PlurkError> {
    let resp = plurk.request("/APP/Users/me").await?;
    let body = resp
        .json::<PlurkUser>()
        .await
        .map_err(|e| PlurkError::ParseError(e.to_string()))?;
    println!("{}", body);
    Ok(())
}

#[allow(unreachable_code)]
pub async fn comet_loop(plurk: Plurk) -> Result<(), PlurkError> {
    let mut count: u8 = 0;
    let mut comet = PlurkComet::from_plurk(plurk.clone()).await?;
    loop {
        if count > 10 {
            comet.knock().await?;
            count = 0;
        } else {
            count += 1;
        }

        let cdata = match comet.poll_once_mut().await {
            Ok(d) => d,
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };

        if let Some(datas) = cdata {
            for data in datas {
                PlurkComet::print_comet(&plurk, data).await?;
                if let Some((Width(w), _)) = terminal_size() {
                    println!("{}", "=".repeat(w.into()));
                }
            }
        };
    }
    Ok(())
}

pub async fn poll_comet(plurk: Plurk) -> Result<(), PlurkError> {
    println!("Polling Comet...ctrl+c to exit");
    tokio::select! {
        output = comet_loop(plurk) => output,
        _ = signal::ctrl_c() => Ok(()),
    }
}

pub fn gen_key_file(
    consumer_key: String,
    consumer_secret: String,
    token_key: Option<String>,
    token_secret: Option<String>,
) -> Result<(), PlurkError> {
    let plurk = Plurk::new(consumer_key, consumer_secret, token_key, token_secret);
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir,
        None => PathBuf::from("."),
    };
    let config_dir = config_dir.join("plurk-cli");
    let display = config_dir.display();
    fs::create_dir_all(config_dir.clone())
        .map_err(|_| PlurkError::IOError(format!("Cannot create dir: {}", display)))?;

    let config_file = config_dir.join("key.toml");
    let display = config_file.display();
    plurk
        .to_toml(config_file.clone())
        .map_err(|_| PlurkError::IOError(format!("Cannot access file: {}", display)))?;
    Ok(())
}

pub fn get_key_file() -> Result<PathBuf, PlurkError> {
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir,
        None => PathBuf::from("."),
    };
    let config_dir = config_dir.join("plurk-cli");
    let display = config_dir.display();
    fs::create_dir_all(config_dir.clone())
        .map_err(|_| PlurkError::IOError(format!("Cannot create dir: {}", display)))?;

    let config_file = config_dir.join("key.toml");
    Ok(config_file)
}

pub fn get_key_file_string() -> String {
    get_key_file()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap()
}
