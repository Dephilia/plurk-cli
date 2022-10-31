mod comet;
mod error;
mod plurk;
mod utils;

use comet::PlurkComet;
use error::PlurkError;
use plurk::Plurk;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio;

/*
#[derive(Deserialize)]
struct PublicPlurks {
    plurks: Vec<PlurkData>,
}
*/

#[tokio::main]
async fn main() -> Result<(), PlurkError> {
    let mut plurk = Plurk::from_toml("key.toml")?;

    if !plurk.has_token() {
        plurk.acquire_plurk_key().await?;
        plurk.to_toml("key.toml")?;
    }

    /*
    let plurk = plurk;
    let resp = plurk.request("/APP/Users/me").await?;

    let body = resp
        .json::<PlurkUser>()
        .await
        .map_err(|e| PlurkError::ParseError(e.to_string()))?;
    // println!("{:?}", body);
    let user_id = body.id;

    let resp = plurk
        .request_query("/APP/Timeline/getPublicPlurks", &[("user_id", user_id)])
        .await?;

    let body = resp
        .json::<PublicPlurks>()
        .await
        .map_err(|e| PlurkError::ParseError(e.to_string()))?;
    for data in body.plurks {
        println!(
            "{} https://www.plurk.com/p/{} | {}",
            data.posted.format("%Y%m%d %H:%M"),
            base36_encode(data.plurk_id),
            limit_str(&data.content_raw, 40),
        );
    }
    */

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
                PlurkComet::print_comet(data);
            }
        };
    }
    println!("Goodbye!");

    Ok(())
}
