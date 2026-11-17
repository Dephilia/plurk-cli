use crate::config::{Credentials, Keys};
use crate::error::{Error, Result};
use reqwest::Url;
use reqwest_oauth1::{OAuthClientProvider, Secrets};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::io::{self, Write};

pub const BASE_URL: &str = "https://www.plurk.com";

#[derive(Clone, Debug)]
pub struct Client {
    pub http: reqwest::Client,
    pub creds: Credentials,
}

#[derive(Deserialize)]
struct ErrorBody {
    error_text: Option<String>,
}

pub fn api_error(status: u16, body: &str) -> Error {
    let message = serde_json::from_str::<ErrorBody>(body)
        .ok()
        .and_then(|b| b.error_text)
        .unwrap_or_else(|| body.trim().to_string());
    Error::Api { status, message }
}

pub fn parse_response<T: DeserializeOwned>(status: u16, body: &str) -> Result<T> {
    if !(200..300).contains(&status) {
        return Err(api_error(status, body));
    }
    serde_json::from_str(body).map_err(|source| Error::Decode {
        source,
        body: body.chars().take(200).collect(),
    })
}

pub fn parse_token_body(body: &str) -> Result<Keys> {
    let url = Url::parse(&format!("http://x/?{}", body.trim()))
        .map_err(|_| Error::OAuthReply(body.chars().take(200).collect()))?;
    let mut key = None;
    let mut secret = None;
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "oauth_token" => key = Some(v.into_owned()),
            "oauth_token_secret" => secret = Some(v.into_owned()),
            _ => {}
        }
    }
    match (key, secret) {
        (Some(key), Some(secret)) => Ok(Keys { key, secret }),
        _ => Err(Error::OAuthReply(body.chars().take(200).collect())),
    }
}

impl Client {
    pub fn new(creds: Credentials) -> Self {
        Self {
            http: reqwest::Client::new(),
            creds,
        }
    }

    fn secrets(&self, token: Option<&Keys>) -> Secrets<'static> {
        let s = Secrets::new(
            self.creds.consumer.key.clone(),
            self.creds.consumer.secret.clone(),
        );
        match token {
            Some(t) => s.token(t.key.clone(), t.secret.clone()),
            None => s,
        }
    }

    pub async fn call<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let resp = self
            .http
            .clone()
            .oauth1(self.secrets(self.creds.token()))
            .post(format!("{BASE_URL}/APP{path}"))
            .form(&params.to_vec())
            .send()
            .await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        parse_response(status, &body)
    }

    async fn oauth_step(
        &self,
        path: &str,
        token: Option<&Keys>,
        query: &[(&str, &str)],
    ) -> Result<Keys> {
        let resp = self
            .http
            .clone()
            .oauth1(self.secrets(token))
            .post(format!("{BASE_URL}{path}"))
            .query(query)
            .send()
            .await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        if !(200..300).contains(&status) {
            return Err(api_error(status, &body));
        }
        parse_token_body(&body)
    }

    pub async fn acquire_token(&mut self) -> Result<()> {
        let temp = self
            .oauth_step("/OAuth/request_token", None, &[("oauth_callback", "oob")])
            .await?;
        println!(
            "Please access to: {BASE_URL}/OAuth/authorize?oauth_token={}",
            temp.key
        );
        print!("Input pin:");
        let io_err = |e| Error::Io {
            path: "stdin".into(),
            source: e,
        };
        io::stdout().flush().map_err(io_err)?;
        let mut pin = String::new();
        io::stdin().read_line(&mut pin).map_err(io_err)?;
        let access = self
            .oauth_step(
                "/OAuth/access_token",
                Some(&temp),
                &[("oauth_verifier", pin.trim())],
            )
            .await?;
        self.creds.oauth_token = Some(access);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_text_maps_to_api_error() {
        let r: Result<serde_json::Value> =
            parse_response(401, r#"{"error_text":"40101:unknown application key"}"#);
        match r {
            Err(Error::Api { status, message }) => {
                assert_eq!(status, 401);
                assert_eq!(message, "40101:unknown application key");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn non_json_error_falls_back_to_raw_body() {
        let e = api_error(502, "Bad Gateway\n");
        assert_eq!(e.to_string(), "Plurk API error (502): Bad Gateway");
    }

    #[test]
    fn display_matches_expected_cli_message() {
        let e = api_error(401, r#"{"error_text":"40101:unknown application key"}"#);
        assert_eq!(
            e.to_string(),
            "Plurk API error (401): 40101:unknown application key"
        );
    }

    #[test]
    fn success_decodes_and_bad_json_is_decode_error() {
        let v: serde_json::Value = parse_response(200, r#"{"a":1}"#).unwrap();
        assert_eq!(v["a"], 1);
        assert!(matches!(
            parse_response::<serde_json::Value>(200, "nope"),
            Err(Error::Decode { .. })
        ));
    }

    #[test]
    fn token_body_parses() {
        let k = parse_token_body(
            "oauth_token_secret=s%2B1&oauth_token=t&oauth_callback_confirmed=true",
        )
        .unwrap();
        assert_eq!(
            k,
            Keys {
                key: "t".into(),
                secret: "s+1".into()
            }
        );
        assert!(parse_token_body("oauth_token=t").is_err());
    }
}
