/*

plurk.rs
Copyright (C) 2022 dephilia <me@dephilia.moe>
Distributed under terms of the MIT license.
Plurk API doc: https://www.plurk.com/API

*/

use crate::error::PlurkError;
use crate::utils::*;
use chrono::{self, DateTime, FixedOffset};
use reqwest;
use reqwest_oauth1::{OAuthClientProvider, Secrets, TokenReaderFuture};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

const REQUEST_TOKEN_URL: &str = "/OAuth/request_token";
const AUTHORIZE_URL: &str = "/OAuth/authorize";
const ACCESS_TOKEN_URL: &str = "/OAuth/access_token";
const BASE_URL: &str = "https://www.plurk.com";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Plurk {
    consumer: PlurkKeys,
    oauth_token: Option<PlurkKeys>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlurkKeys {
    key: String,
    secret: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct PlurkUser {
    pub id: u64,
    nick_name: String,
    pub display_name: String,
    full_name: Option<String>,
    avatar: u64,
    date_of_birth: Option<WrappedDT>,
    dateformat: u8,
    default_lang: String,
    friend_list_privacy: String,
    gender: u8,
    has_profile_image: u8,
    karma: f32,
    name_color: Option<String>,
    premium: bool,
    status: String,
    timeline_privacy: u8,
    uid: Option<u64>,
    verified_account: bool,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct PlurkData {
    pub plurk_id: u64,
    #[serde(deserialize_with = "from_rfc2822")]
    pub posted: DateTime<FixedOffset>,
    pub content: String,
    pub content_raw: String,
    pub owner_id: u64,
    pub user_id: u64,
    anonymous: bool,
    coins: u64,
    favorers: Vec<u64>,
    favorite: bool,
    favorite_count: u64,
    has_gift: bool,
    id: Option<u64>,
    is_unread: u64,
    lang: String,
    mentioned: u64,
    no_comments: u64,
    plurk_type: u8,
    porn: bool,
    publish_to_followers: bool,
    qualifier: String,
    replurkable: bool,
    replurked: bool,
    replurkers: Vec<u64>,
    replurkers_count: u64,
    responded: u64,
    response_count: u64,
    responses_seen: u64,
    with_poll: bool,
    replurker_id: Option<u64>,
    excluded: Option<String>,
    limited_to: Option<String>,
    last_edited: Option<WrappedDT>,
}

impl Plurk {
    pub fn from_toml(path: &str) -> Result<Self, PlurkError> {
        let path = Path::new(path);
        let display = path.display();

        let mut file =
            File::open(&path).map_err(|_| PlurkError::IOError(format!("{}", display)))?;

        let mut s = String::new();
        file.read_to_string(&mut s)
            .map_err(|_| PlurkError::IOError(format!("{}", display)))?;

        toml::from_str(s.as_str()).map_err(|_| PlurkError::IOError(format!("{}", display)))
    }

    pub fn to_toml(&self, path: &str) -> Result<(), PlurkError> {
        let path = Path::new(path);
        let display = path.display().to_string();

        let mut file =
            File::create(&path).map_err(|_| PlurkError::IOError(format!("{}", display)))?;

        let s = toml::to_string(&self).map_err(|_| PlurkError::IOError(format!("{}", display)))?;

        file.write_all(s.as_bytes())
            .map_err(|_| PlurkError::IOError(format!("{}", display)))?;

        Ok(())
    }

    pub fn has_token(&self) -> bool {
        match &self.oauth_token {
            Some(ot) if (ot.key.is_empty() || ot.secret.is_empty()) => false,
            Some(_) => true,
            None => false,
        }
    }

    fn to_secret(&self) -> Secrets {
        match &self.oauth_token {
            Some(ot) if (ot.key.is_empty() || ot.secret.is_empty()) => {
                Secrets::new(self.consumer.key.clone(), self.consumer.secret.clone())
            }
            Some(ot) => Secrets::new(self.consumer.key.clone(), self.consumer.secret.clone())
                .token(ot.key.clone(), ot.secret.clone()),
            None => Secrets::new(self.consumer.key.clone(), self.consumer.secret.clone()),
        }
    }

    pub async fn request(&self, api: &str) -> Result<reqwest::Response, PlurkError> {
        let secrets = self.to_secret().clone();
        Ok(reqwest::Client::new()
            .oauth1(secrets)
            .post(Plurk::cmd(api))
            .send()
            .await
            .map_err(|e| PlurkError::OauthError(e))?)
    }

    pub async fn request_query<T>(
        &self,
        api: &str,
        query: &T,
    ) -> Result<reqwest::Response, PlurkError>
    where
        T: Serialize + ?Sized + Clone,
    {
        let secrets = self.to_secret().clone();
        Ok(reqwest::Client::new()
            .oauth1(secrets)
            .post(Plurk::cmd(api))
            .form(query)
            .send()
            .await
            .map_err(|e| PlurkError::OauthError(e))?)
    }

    pub async fn acquire_plurk_key(&mut self) -> Result<(), PlurkError> {
        let secrets = self.to_secret();

        let endpoint_reqtoken = format!("{}{}", BASE_URL, REQUEST_TOKEN_URL);

        let client = reqwest::Client::new();
        let resp = client
            .oauth1(secrets)
            .post(endpoint_reqtoken)
            .query(&[("oauth_callback", "oob")])
            .send()
            .parse_oauth_token()
            .await
            .map_err(|e| PlurkError::OauthError(e))?;

        // step 2. acquire user pin
        let endpoint_authorize = format!(
            "{}{}?oauth_token={}",
            BASE_URL, AUTHORIZE_URL, resp.oauth_token
        );
        println!("Please access to: {}", endpoint_authorize);

        print!("Input pin: ");
        let mut user_input = String::new();
        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read the user input");
        let pin = user_input.trim();

        // step 3. acquire access token
        let secrets = self
            .to_secret()
            .token(resp.oauth_token, resp.oauth_token_secret);
        let endpoint_acctoken = format!("{}{}", BASE_URL, ACCESS_TOKEN_URL);

        let client = reqwest::Client::new();
        let resp = client
            .oauth1(secrets)
            .post(endpoint_acctoken)
            .query(&[("oauth_verifier", pin)])
            .send()
            .parse_oauth_token()
            .await
            .map_err(|e| PlurkError::OauthError(e))?;
        let oauth_token = PlurkKeys {
            key: resp.oauth_token,
            secret: resp.oauth_token_secret,
        };
        self.oauth_token = Some(oauth_token);

        Ok(())
    }

    fn cmd(api: &str) -> String {
        format!("{}{}", BASE_URL, api)
    }
}
