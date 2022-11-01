// plurk.rs
// Copyright (C) 2022 dephilia <me@dephilia.moe>
// Distributed under terms of the MIT license.

mod comet;
mod error;
mod plurk;
mod utils;

use comet::PlurkComet;
use error::PlurkError;
use plurk::Plurk;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use terminal_size::{terminal_size, Width};
use tokio;

#[tokio::main]
async fn main() -> Result<(), PlurkError> {
    let mut plurk = Plurk::from_toml("key.toml")?;

    if !plurk.has_token() {
        plurk.acquire_plurk_key().await?;
        plurk.to_toml("key.toml")?;
    }

    let mut comet = PlurkComet::from_plurk(plurk.clone()).await?;
    comet.print();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc_async::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    println!("Polling Comet...ctrl+c to exit");
    while running.load(Ordering::SeqCst) {
        let cdata = comet.poll_once_mut().await?;
        if let Some(datas) = cdata {
            for data in datas {
                PlurkComet::print_comet(&plurk, data).await?;
                if let Some((Width(w), _)) = terminal_size() {
                    println!("{}", "=".repeat(w.into()));
                }
            }
        };
    }
    println!("Goodbye!");

    Ok(())
}
