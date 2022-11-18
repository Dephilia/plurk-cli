// plurk.rs
// Copyright (C) 2022 dephilia <me@dephilia.moe>
// Distributed under terms of the MIT license.
// Plurk API doc: https://www.plurk.com/API

use crate::error::PlurkError;
use crate::utils::*;
use chrono::{self, DateTime, FixedOffset};
use reqwest;
use reqwest_oauth1::{OAuthClientProvider, Secrets, TokenReaderFuture};
use serde::{Deserialize, Serialize};
use std::fmt;
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
    pub nick_name: String,
    pub display_name: String,
    pub full_name: Option<String>,
    pub avatar: Option<u64>,
    pub date_of_birth: Option<WrappedDT>,
    pub dateformat: u8,
    pub default_lang: String,
    pub friend_list_privacy: String,
    pub gender: u8,
    pub has_profile_image: u8,
    pub karma: f32,
    pub name_color: Option<String>,
    pub premium: bool,
    pub status: String,
    pub timeline_privacy: u8,
    pub uid: Option<u64>,
    pub verified_account: bool,
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
    pub anonymous: bool,
    pub coins: u64,
    pub favorers: Vec<u64>,
    pub favorite: bool,
    pub favorite_count: u64,
    pub has_gift: bool,
    pub id: Option<u64>,
    pub is_unread: u64,
    pub lang: String,
    pub mentioned: u64,
    pub no_comments: u64,
    pub plurk_type: u8,
    pub porn: bool,
    pub publish_to_followers: bool,
    pub qualifier: String,
    pub replurkable: bool,
    pub replurked: bool,
    pub replurkers: Vec<u64>,
    pub replurkers_count: u64,
    pub responded: u64,
    pub response_count: u64,
    pub responses_seen: u64,
    pub with_poll: bool,
    pub replurker_id: Option<u64>,
    pub excluded: Option<String>,
    pub limited_to: Option<String>,
    pub last_edited: Option<WrappedDT>,
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

impl fmt::Display for PlurkUser {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "PlurkUser {}({}) aka. {}",
            self.nick_name, self.id, self.display_name
        )
    }
}

impl fmt::Display for PlurkData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Plurk ==> https://www.plurk.com/p/{}\n\
            {} {}\n\
            {}
            ",
            base36_encode(self.plurk_id),
            self.user_id,
            self.qualifier,
            self.content_raw
        )
    }
}
