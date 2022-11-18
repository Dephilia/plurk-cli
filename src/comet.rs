// comet.rs
// Copyright (C) 2022 dephilia <me@dephilia.moe>
// Distributed under terms of the MIT license.

use crate::error::PlurkError;
use crate::plurk::{Plurk, PlurkData, PlurkUser};
use crate::utils::*;
use chrono::{self, DateTime, FixedOffset};
use colored::Colorize;
use regex::Regex;
use reqwest::Url;
use serde::Deserialize;
use serde_qs as qs;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

const COMET_KNOCK: &str = "https://www.plurk.com/_comet/generic";

#[derive(Clone, Debug)]
pub struct PlurkComet {
    base_url: String,
    channel: String,
    offset: i64,
}

#[derive(Deserialize, Debug)]
pub struct UserChannel {
    comet_server: String,
    #[allow(dead_code)]
    channel_name: String,
}

#[derive(Debug, PartialEq, Deserialize)]
struct CometDatas {
    channel: String,
    offset: i64,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum CometContentUnit {
    #[serde(rename = "new_response")]
    Response {
        plurk_id: u64,
        #[serde(rename = "plurk")]
        plurk_data: PlurkData,
        response: CometResponse,
        response_count: u64,
        user: HashMap<String, PlurkUser>,
    },
    #[serde(rename = "new_plurk")]
    Plurk(PlurkData),
    #[serde(rename = "update_notification")]
    Notification { counts: CometNotiCount },
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct CometResponse {
    content: String,
    content_raw: String,
    editability: u8,
    id: u64,
    lang: String,
    last_edited: Option<WrappedDT>,
    plurk_id: u64,
    #[serde(deserialize_with = "from_rfc2822")]
    pub posted: DateTime<FixedOffset>,
    qualifier: String,
    user_id: u64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct CometNotiCount {
    noti: u32,
    req: u32,
}

#[derive(Deserialize, Debug)]
struct CometContent {
    new_offset: i64,
    data: Option<Vec<CometContentUnit>>,
}

impl PlurkComet {
    pub async fn from_plurk(plurk: Plurk) -> Result<Self, PlurkError> {
        let resp = plurk.request("/APP/Realtime/getUserChannel").await?;

        let body = resp
            .json::<UserChannel>()
            .await
            .map_err(|e| PlurkError::ParseError(e.to_string()))?;

        PlurkComet::new(body.comet_server.as_str())
    }
    pub fn new(comet_url: &str) -> Result<Self, PlurkError> {
        let url = Url::parse(comet_url).map_err(|e| PlurkError::ParseError(e.to_string()))?;
        let query = match url.query() {
            Some(q) => q,
            None => return Err(PlurkError::InvalidUrl(comet_url.to_string()).into()),
        };

        let comet_datas: CometDatas =
            qs::from_str(query).map_err(|e| PlurkError::ParseError(e.to_string()))?;

        let url = url
            .join("comet")
            .map_err(|e| PlurkError::ParseError(e.to_string()))?;

        Ok(PlurkComet {
            base_url: Url::to_string(&url),
            channel: comet_datas.channel,
            offset: comet_datas.offset,
        })
    }
    pub async fn poll_once_mut(&mut self) -> Result<Option<Vec<CometContentUnit>>, PlurkError> {
        let url = Url::parse_with_params(
            &self.base_url,
            &[
                ("channel", &self.channel),
                ("offset", &self.offset.to_string()),
            ],
        )
        .map_err(|_| PlurkError::UrlError)?;

        let client = reqwest::Client::new();

        let res = client
            .get(url)
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| PlurkError::ReqwestError(e))?;

        let text = res.text().await.map_err(|e| PlurkError::ReqwestError(e))?;

        let res = PlurkComet::query(text.as_str())?;
        self.offset = res.new_offset;

        Ok(res.data)
    }

    fn query(comet_callback: &str) -> Result<CometContent, PlurkError> {
        let re = Regex::new(r"CometChannel.scriptCallback\((.*)\);")
            .map_err(|e| PlurkError::InvalidCometData(e.to_string()))?;
        let mat = match re.captures(comet_callback) {
            Some(m) => m,
            None => return Err(PlurkError::InvalidCometData(comet_callback.to_string()).into()),
        };
        serde_json::from_str(&mat[1])
            .map_err(|e| PlurkError::InvalidCometData(format!("{}\n{}", e, comet_callback)))
    }

    pub async fn knock(&self) -> Result<(), PlurkError> {
        let url = Url::parse_with_params(&COMET_KNOCK, &[("channel", &self.channel)])
            .map_err(|_| PlurkError::UrlError)?;

        let client = reqwest::Client::new();

        let _res = client
            .get(url)
            .send()
            .await
            .map_err(|e| PlurkError::ReqwestError(e))?;
        Ok(())
    }

    pub async fn print_comet(plurk: &Plurk, comet: CometContentUnit) -> Result<(), PlurkError> {
        #[derive(Deserialize)]
        struct ObjGetPublicProfile {
            user_info: PlurkUser,
        }
        match comet {
            CometContentUnit::Response {
                plurk_id,
                plurk_data,
                response,
                response_count: _,
                user,
            } => {
                let resp = plurk
                    .request_query(
                        "/APP/Profile/getPublicProfile",
                        &[("user_id", plurk_data.owner_id)],
                    )
                    .await?;

                let plurk_owner = resp
                    .json::<ObjGetPublicProfile>()
                    .await
                    .map_err(|e| PlurkError::ParseError(e.to_string()))?;

                let display_name = plurk_owner.user_info.display_name;

                let response_display_name = &user
                    .get(&response.user_id.to_string())
                    .unwrap()
                    .display_name;
                println!(
                    "New response ==> https://www.plurk.com/p/{}",
                    base36_encode(plurk_id)
                );
                println!(
                    "{} {} {}",
                    plurk_data
                        .posted
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .yellow(),
                    display_name.bold().bright_blue(),
                    plurk_data.qualifier.black().on_bright_white()
                );
                println!("{}", plurk_data.content_raw);
                println!(" -------");
                println!(
                    "{} {} {} {}",
                    response
                        .posted
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .yellow(),
                    response_display_name.bold().bright_magenta(),
                    response.qualifier.black().on_bright_white(),
                    response.content_raw
                );
            }
            CometContentUnit::Plurk(p) => {
                let resp = plurk
                    .request_query("/APP/Profile/getPublicProfile", &[("user_id", p.owner_id)])
                    .await?;

                let plurk_owner = resp
                    .json::<ObjGetPublicProfile>()
                    .await
                    .map_err(|e| PlurkError::ParseError(e.to_string()))?;

                let display_name = plurk_owner.user_info.display_name;
                println!(
                    "New plurk ==> https://www.plurk.com/p/{}",
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
            }
            CometContentUnit::Notification { counts } => {
                println!("Notification: {:?}", counts);
            }
        };
        Ok(())
    }
}
impl fmt::Display for PlurkComet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<Comet>:\n\tBase Url: {}\n\tChannel: {}\n\tOffset: {}",
            self.base_url, self.channel, self.offset
        )
    }
}
