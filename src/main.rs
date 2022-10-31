mod comet;
mod plurk;
mod plurkerr;

use comet::PlurkComet;
use plurk::{Plurk, PlurkData, PlurkUser};
use plurkerr::PlurkError;
use serde::Deserialize;
use std::cmp;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio;

#[derive(Deserialize)]
struct PublicPlurks {
    plurks: Vec<PlurkData>,
}

fn base36_encode(value: u64) -> String {
    let base36 = "0123456789abcdefghijklmnopqrstuvwxyz".as_bytes();
    let mut v = value;
    let mut result = String::new();

    loop {
        let i: usize = v as usize % 36;
        let b: char = base36[i] as char;
        result.push(b);
        v = v / 36;
        if v == 0 {
            break;
        }
    }

    result.chars().rev().collect::<String>()
}

fn limit_str(text: &str, limit: usize) -> String {
    let text_size = text.chars().count() as usize;
    let text = match text.find('\n') {
        Some(size) => match text.get(0..size) {
            Some(r) => r,
            None => text,
        },
        None => text,
    };

    let str_size = text.chars().count() as usize;
    let limit = cmp::min(limit, str_size);

    let mut ret: String = text.chars().take(limit).skip(0).collect();

    if ret.chars().count() < text_size {
        ret.push_str(" ...<read more> ");
    }
    ret
}

#[tokio::main]
async fn main() -> Result<(), PlurkError> {
    let mut plurk = Plurk::from_toml("key.toml");

    if !plurk.has_token() {
        plurk.acquire_plurk_key().await?;
        plurk.to_toml("key.toml");
    }

    let plurk = plurk;
    let resp = plurk.request("/APP/Users/me").await?;

    let body = resp
        .json::<PlurkUser>()
        .await
        .map_err(|_| PlurkError::ParseError)?;
    println!("{:?}", body);
    let user_id = body.id;

    // let body = resp.json::<serde_json::Value>().await?;
    // let date = chrono::offset::Utc::now();
    // let stamp = date.format("%Y-%m-%dT%H:%M:%S");
    // println!("{}", stamp);

    let resp = plurk
        .request_query("/APP/Timeline/getPublicPlurks", &[("user_id", user_id)])
        .await?;

    // let body = resp.json::<serde_json::Value>().await?;
    // println!("{:?}", body["plurks"]);

    let body = resp
        .json::<PublicPlurks>()
        .await
        .map_err(|_| PlurkError::ParseError)?;
    for data in body.plurks {
        println!(
            "{} https://www.plurk.com/p/{} | {}",
            data.posted.format("%Y%m%d %H:%M"),
            base36_encode(data.plurk_id),
            limit_str(&data.content_raw, 40),
        );
    }

    let mut comet = PlurkComet::from_plurk(plurk.clone()).await?;
    comet.print();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    println!("Polling Comet...ctrl+c to exit");
    while running.load(Ordering::SeqCst) {
        let cdata = comet.call_once_mut().await?;
        if let Some(datas) = cdata {
            for data in datas {
                println!(
                    "{:?} https://www.plurk.com/p/{}",
                    data.comet_type,
                    base36_encode(data.plurk_id),
                );
            }
        };
    }
    println!("Goodbye!");

    Ok(())
}
