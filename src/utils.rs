//
// util.rs
// Copyright (C) 2022 dephilia <dephilia@MacBook-Pro.local>
// Distributed under terms of the MIT license.
//
//
use chrono::{self, DateTime, FixedOffset};
use serde::{Deserialize, Deserializer};
use std::cmp;

pub fn base36_encode(value: u64) -> String {
    let base36 = "0123456789abcdefghijklmnopqrstuvwxyz".as_bytes();
    let mut v = value;
    let mut result = String::new();

    loop {
        let i: usize = v as usize % 36;
        let b: char = base36[i] as char;
        result.push(b);
        v = v / 36;
        if v == 0 {
            break;
        }
    }

    result.chars().rev().collect::<String>()
}

pub fn limit_str(text: &str, limit: usize) -> String {
    let text_size = text.chars().count() as usize;
    let text = match text.find('\n') {
        Some(size) => match text.get(0..size) {
            Some(r) => r,
            None => text,
        },
        None => text,
    };

    let str_size = text.chars().count() as usize;
    let limit = cmp::min(limit, str_size);

    let mut ret: String = text.chars().take(limit).skip(0).collect();

    if ret.chars().count() < text_size {
        ret.push_str(" ...<read more> ");
    }
    ret
}

pub fn from_rfc2822<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;

    Ok(DateTime::parse_from_rfc2822(&s).unwrap())
}

#[derive(Debug, Deserialize)]
pub struct WrappedDT(#[serde(deserialize_with = "from_rfc2822")] DateTime<FixedOffset>);
