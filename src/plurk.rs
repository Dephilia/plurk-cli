/*

plurk.rs
Copyright (C) 2022 dephilia <me@dephilia.moe>
Distributed under terms of the MIT license.
Plurk API doc: https://www.plurk.com/API

*/

use crate::plurkerr::PlurkError;
use chrono::{self, DateTime, FixedOffset};
use reqwest;
use reqwest_oauth1::{OAuthClientProvider, Secrets, TokenReaderFuture};
use serde::{Deserialize, Deserializer, Serialize};
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
pub struct PlurkUser {
    pub id: u64,
    pub nick_name: String,
    pub display_name: String,
    pub full_name: String,
}

#[derive(Deserialize, Debug)]
pub struct PlurkData {
    pub plurk_id: u64,
    #[serde(deserialize_with = "from_rfc2822")]
    pub posted: DateTime<FixedOffset>,
    pub content: String,
    pub content_raw: String,
    pub owner_id: u64,
    pub user_id: u64,
}

impl Plurk {
    pub fn from_toml(path: &str) -> Self {
        let path = Path::new(path);
        let display = path.display();

        let mut file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {:?}", display, why),
            Ok(file) => file,
        };

        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(why) => panic!("couldn't read {}: {:?}", display, why),
            Ok(_) => (),
        }

        match toml::from_str(s.as_str()) {
            Err(why) => panic!("parse error {}: {:?}", display, why),
            Ok(parsed) => parsed,
        }
    }

    pub fn to_toml(&self, path: &str) {
        let path = Path::new(path);
        let display = path.display();

        let mut file = match File::create(&path) {
            Err(why) => panic!("couldn't create {}: {:?}", display, why),
            Ok(file) => file,
        };

        let s = match toml::to_string(&self) {
            Err(why) => panic!("parse error {}: {:?}", display, why),
            Ok(parsed) => parsed,
        };

        match file.write_all(s.as_bytes()) {
            Err(why) => panic!("couldn't write {}: {:?}", display, why),
            Ok(_) => (),
        }
    }

    pub fn has_token(&self) -> bool {
        match &self.oauth_token {
            Some(ot) if (ot.key.is_empty() || ot.secret.is_empty()) => false,
            Some(_) => true,
            None => false,
        }
    }

    pub fn to_secret(&self) -> Secrets {
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

    pub fn cmd(api: &str) -> String {
        format!("{}{}", BASE_URL, api)
    }
}

fn from_rfc2822<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;

    Ok(DateTime::parse_from_rfc2822(&s).unwrap())
}
