//
// error.rs
// Copyright (C) 2022 dephilia <dephilia@MacBook-Pro.local>
// Distributed under terms of the MIT license.
//

use std::{error::Error, fmt};

#[derive(Debug)]
pub enum PlurkError {
    InvalidUrl(String),
    InvalidCometData(String),
    ParseError,
    OauthError(reqwest_oauth1::Error),
    ReqwestError(reqwest::Error),
    UrlError,
    StdError(Box<dyn Error>),
}

impl fmt::Display for PlurkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidUrl(url) => write!(f, "Invalid url: {}", url),
            Self::InvalidCometData(url) => write!(f, "Invalid comet data: {}", url),
            Self::ParseError => write!(f, "Parse comet data error"),
            Self::OauthError(e) => write!(f, "oauth1 error: {}", e),
            Self::ReqwestError(e) => write!(f, "reqwest error: {}", e),
            Self::UrlError => write!(f, "url error"),
            Self::StdError(e) => write!(f, "std error: {}", e),
        }
    }
}

impl Error for PlurkError {}
