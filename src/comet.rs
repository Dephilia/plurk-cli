use crate::plurk::{Plurk, PlurkData, PlurkUser};
use crate::plurkerr::PlurkError;
use regex::Regex;
use reqwest::Url;
use serde::Deserialize;
use serde_qs as qs;
use std::time::Duration;

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
pub enum CometContentType {
    #[serde(rename = "new_response")]
    Response,
    #[serde(rename = "new_plurk")]
    Plurk,
}

#[derive(Deserialize, Debug)]
pub struct CometContentUnit {
    #[serde(rename = "type")]
    pub comet_type: CometContentType,
    pub plurk_id: u64,
    // pub plurk: Option<PlurkData>,
    // pub user: Option<Vec<HashMap<u64, PlurkUser>>>,
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
            .map_err(|_| PlurkError::ParseError)?;

        PlurkComet::new(body.comet_server.as_str())
    }
    pub fn new(comet_url: &str) -> Result<Self, PlurkError> {
        let url = match Url::parse(comet_url) {
            Ok(u) => u,
            Err(_) => return Err(PlurkError::ParseError),
        };
        let query = match url.query() {
            Some(q) => q,
            None => return Err(PlurkError::InvalidUrl(comet_url.to_string()).into()),
        };

        let comet_datas: CometDatas = match qs::from_str(query) {
            Ok(data) => data,
            Err(_) => return Err(PlurkError::ParseError),
        };

        let url = match url.join("comet") {
            Ok(data) => data,
            Err(_) => return Err(PlurkError::ParseError),
        };

        Ok(PlurkComet {
            base_url: Url::to_string(&url),
            channel: comet_datas.channel,
            offset: comet_datas.offset,
        })
    }
    pub fn print(&self) {
        println!(
            "<Comet>:\n\tBase Url: {}\n\tChannel: {}\n\tOffset: {}",
            self.base_url, self.channel, self.offset
        );
    }

    pub async fn call_once_mut(&mut self) -> Result<Option<Vec<CometContentUnit>>, PlurkError> {
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
        let re =
            Regex::new(r"CometChannel.scriptCallback\((.*)\);").or(Err(PlurkError::ParseError))?;
        let mat = match re.captures(comet_callback) {
            Some(m) => m,
            None => return Err(PlurkError::InvalidCometData(comet_callback.to_string()).into()),
        };
        match serde_json::from_str(&mat[1]) {
            Ok(t) => Ok(t),
            Err(_) => return Err(PlurkError::InvalidCometData(mat[1].to_string()).into()),
        }
    }
}
