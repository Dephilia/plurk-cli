use crate::client::{Client, api_error};
use crate::error::{Error, Result};
use crate::model::{Plurk, Response, User};
use reqwest::Url;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Deserialize, Debug)]
pub struct Channel {
    pub comet_server: String,
    pub channel_name: String,
}

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub new_offset: i64,
    pub data: Option<Vec<Event>>,
}

#[derive(Deserialize, Debug)]
pub struct Counts {
    #[serde(default)]
    pub noti: u64,
    #[serde(default)]
    pub req: u64,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "new_plurk")]
    NewPlurk(Plurk),
    #[serde(rename = "new_response")]
    NewResponse {
        plurk_id: u64,
        plurk: Option<Plurk>,
        response: Response,
        #[serde(default)]
        user: HashMap<String, User>,
    },
    #[serde(rename = "update_notification")]
    UpdateNotification { counts: Counts },
    #[serde(other)]
    Unknown,
}

pub struct Comet {
    http: reqwest::Client,
    url: Url,
    channel: String,
    offset: Option<i64>,
}

pub fn strip_jsonp(text: &str) -> &str {
    match (text.find('{'), text.rfind('}')) {
        (Some(a), Some(b)) if a <= b => &text[a..=b],
        _ => text,
    }
}

impl Comet {
    pub async fn connect(client: &Client) -> Result<Self> {
        let ch: Channel = client.call("/Realtime/getUserChannel", &[]).await?;
        Self::new(client.http.clone(), &ch)
    }

    pub fn new(http: reqwest::Client, ch: &Channel) -> Result<Self> {
        let mut url = Url::parse(&ch.comet_server).map_err(|e| Error::Url(e.to_string()))?;
        let offset = url
            .query_pairs()
            .find(|(k, _)| k == "offset")
            .and_then(|(_, v)| v.parse().ok());
        url.set_query(None);
        Ok(Self {
            http,
            url,
            channel: ch.channel_name.clone(),
            offset,
        })
    }

    pub fn request_url(&self) -> Url {
        let mut url = self.url.clone();
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("channel", &self.channel);
            if let Some(o) = self.offset {
                q.append_pair("offset", &o.to_string());
            }
        }
        url
    }

    pub fn apply(&mut self, text: &str) -> Result<Vec<Event>> {
        let payload: Payload =
            serde_json::from_str(strip_jsonp(text)).map_err(|source| Error::Decode {
                source,
                body: text.chars().take(200).collect(),
            })?;
        match payload.new_offset {
            -1 => {}
            -3 => return Err(Error::Resync),
            o => self.offset = Some(o),
        }
        Ok(payload.data.unwrap_or_default())
    }

    pub async fn poll(&mut self) -> Result<Vec<Event>> {
        let resp = self
            .http
            .get(self.request_url())
            .timeout(Duration::from_secs(120))
            .send()
            .await?;
        let status = resp.status().as_u16();
        let text = resp.text().await?;
        if !(200..300).contains(&status) {
            return Err(api_error(status, &text));
        }
        self.apply(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHANNEL: &str = r#"{"comet_server": "https://comet03.plurk.com/comet/1235515351741/?channel=generic-4-f733d8522327edf87b4d1651e6395a6cca0807a0",   "channel_name": "generic-4-f733d8522327edf87b4d1651e6395a6cca0807a0"}"#;
    const NEW_PLURK: &str = r#"{"new_offset": 27, "data": [{"lang":"en", "content":"test another", "content_raw":"test another", "user_id":1, "plurk_type":1, "plurk_id":241403675, "type":"new_plurk", "response_count":0, "favorite":false, "qualifier":"says", "id":241403675, "is_unread":0, "responses_seen":0, "posted":"Tue, 02 Mar 2010 16:21:29 GMT", "limited_to":"|1||1|", "no_comments":0, "favorite_count":0, "owner_id":1, "_cid":27}]}"#;
    const NEW_RESPONSE: &str = r#"{"new_offset": 21, "data": [{"plurk_id": 241392217, "response_count": 12, "type": "new_response", "response": {"id": 7, "plurk_id": 241392217, "user_id": 5737406, "lang":"en", "content":"@Zap: (cozy)", "content_raw":"@Zap: (cozy)", "qualifier": ":", "posted": "Tue, 02 Mar 2010 16:21:29 GMT"},"user": {"5737406": {"display_name":"","uid":5737406, "nick_name": "zed"}}, "_cid": 21}]}"#;

    fn comet() -> Comet {
        let ch: Channel = serde_json::from_str(CHANNEL).unwrap();
        Comet::new(reqwest::Client::new(), &ch).unwrap()
    }

    #[test]
    fn channel_url_per_spec() {
        let c = comet();
        assert_eq!(
            c.request_url().as_str(),
            "https://comet03.plurk.com/comet/1235515351741/?channel=generic-4-f733d8522327edf87b4d1651e6395a6cca0807a0"
        );
    }

    #[test]
    fn waiting_keeps_offset() {
        let mut c = comet();
        c.offset = Some(5);
        assert!(c.apply(r#"{"new_offset": -1}"#).unwrap().is_empty());
        assert_eq!(c.offset, Some(5));
        assert!(c.request_url().as_str().ends_with("&offset=5"));
    }

    #[test]
    fn resync_on_minus_three() {
        let mut c = comet();
        assert!(matches!(
            c.apply(r#"{"new_offset": -3}"#),
            Err(Error::Resync)
        ));
    }

    #[test]
    fn plain_offset_without_data() {
        let mut c = comet();
        assert!(c.apply(r#"{"new_offset": 3}"#).unwrap().is_empty());
        assert_eq!(c.offset, Some(3));
    }

    #[test]
    fn new_plurk_with_string_limited_to() {
        let mut c = comet();
        let ev = c.apply(NEW_PLURK).unwrap();
        assert_eq!(c.offset, Some(27));
        match &ev[0] {
            Event::NewPlurk(p) => {
                assert_eq!(p.plurk_id, 241403675);
                assert_eq!(p.limited_to, Some(serde_json::json!("|1||1|")));
            }
            e => panic!("{e:?}"),
        }
    }

    #[test]
    fn new_response_carries_users() {
        let mut c = comet();
        let ev = c.apply(NEW_RESPONSE).unwrap();
        match &ev[0] {
            Event::NewResponse {
                plurk_id,
                plurk,
                response,
                user,
            } => {
                assert_eq!(*plurk_id, 241392217);
                assert!(plurk.is_none());
                assert_eq!(response.text(), "@Zap: (cozy)");
                assert_eq!(user["5737406"].name(), "zed");
            }
            e => panic!("{e:?}"),
        }
    }

    #[test]
    fn jsonp_wrapped_and_unknown_events() {
        let mut c = comet();
        let wrapped = format!(
            "CometChannel.scriptCallback({});\n",
            r#"{"new_offset": 9, "data": [{"type": "something_new", "x": 1}, {"type": "update_notification", "counts": {"noti": 1, "req": 2}}]}"#
        );
        let ev = c.apply(&wrapped).unwrap();
        assert_eq!(c.offset, Some(9));
        assert!(matches!(ev[0], Event::Unknown));
        assert!(matches!(ev[1], Event::UpdateNotification { .. }));
        assert!(
            c.apply(r#"CometChannel.scriptCallback({"new_offset": -1});"#)
                .is_ok()
        );
    }

    #[test]
    fn garbage_is_error() {
        assert!(matches!(comet().apply("<html>"), Err(Error::Decode { .. })));
    }
}
