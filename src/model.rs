// Spec-shaped deserialization targets: some fields are not rendered yet.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

pub fn from_rfc2822<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    DateTime::parse_from_rfc2822(&s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    #[serde(alias = "uid")]
    pub id: u64,
    #[serde(default)]
    pub nick_name: String,
    pub display_name: Option<String>,
    pub full_name: Option<String>,
    pub avatar: Option<u64>,
    pub gender: Option<u8>,
    pub has_profile_image: Option<u8>,
    pub karma: Option<f64>,
    pub location: Option<String>,
}

impl User {
    pub fn name(&self) -> &str {
        match self.display_name.as_deref() {
            Some(d) if !d.is_empty() => d,
            _ => &self.nick_name,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Plurk {
    pub plurk_id: u64,
    pub owner_id: u64,
    pub user_id: u64,
    #[serde(deserialize_with = "from_rfc2822")]
    pub posted: DateTime<Utc>,
    pub qualifier: String,
    pub qualifier_translated: Option<String>,
    pub content: String,
    pub content_raw: Option<String>,
    #[serde(default)]
    pub response_count: u64,
    pub limited_to: Option<serde_json::Value>,
}

impl Plurk {
    pub fn text(&self) -> &str {
        self.content_raw.as_deref().unwrap_or(&self.content)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Response {
    pub id: u64,
    pub plurk_id: u64,
    pub user_id: u64,
    #[serde(deserialize_with = "from_rfc2822")]
    pub posted: DateTime<Utc>,
    pub qualifier: String,
    pub content: String,
    pub content_raw: Option<String>,
    pub handle: Option<String>,
}

impl Response {
    pub fn text(&self) -> &str {
        self.content_raw.as_deref().unwrap_or(&self.content)
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct PlurkList {
    #[serde(default)]
    pub plurks: Vec<Plurk>,
    #[serde(default)]
    pub plurk_users: HashMap<String, User>,
}

#[derive(Deserialize, Debug)]
pub struct Profile {
    pub user_info: User,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const PLURK: &str = r#"{"responses_seen": 0, "qualifier": "thinks", "plurk_id": 90812, "response_count": 0, "limited_to": null, "no_comments": 0, "is_unread": 1, "lang": "en", "content_raw": "test me out", "user_id": 1, "plurk_type": 0, "content": "test me out", "qualifier_translated": "thinks", "posted": "Fri, 05 Jun 2009 23:07:13 GMT", "owner_id": 1, "favorite": false, "favorite_count": 1, "favorers": [3196376], "replurkable": true, "replurked": true, "replurker_id": null, "replurkers": [1], "replurkers_count": 1}"#;
    const MINIMAL: &str = r#"{"lang": "en", "posted": "Fri, 05 Jun 2009 23:07:13 GMT", "qualifier": "thinks", "plurk_id": 90812, "owner_id": 1, "content": "test me out", "user_id": 1, "is_unread": 1, "no_comments": 0, "plurk_type": 0}"#;
    const USER_MIN: &str = r#"{"display_name": "amix3", "gender": 0, "nick_name": "amix", "has_profile_image": 1, "id": 1, "avatar": null}"#;
    const USER_FULL: &str = r#"{"display_name": "Alexey", "is_channel": 0, "nick_name": "Scoundrel", "has_profile_image": 1, "location": "Canada", "date_of_birth": "Sat, 19 Mar 1983 00:00:00 GMT", "relationship": "not_saying", "avatar": 3, "full_name": "Alexey Kovyrin", "gender": 1, "recruited": 6, "id": 5, "karma": 33.5}"#;

    #[test]
    fn full_plurk() {
        let p: Plurk = serde_json::from_str(PLURK).unwrap();
        assert_eq!(p.plurk_id, 90812);
        assert_eq!(p.text(), "test me out");
        assert_eq!(p.qualifier_translated.as_deref(), Some("thinks"));
        assert_eq!(
            p.posted,
            Utc.with_ymd_and_hms(2009, 6, 5, 23, 7, 13).unwrap()
        );
    }

    #[test]
    fn minimal_plurk() {
        let p: Plurk = serde_json::from_str(MINIMAL).unwrap();
        assert!(p.content_raw.is_none());
        assert_eq!(p.text(), "test me out");
        assert_eq!(p.response_count, 0);
    }

    #[test]
    fn users() {
        let a: User = serde_json::from_str(USER_MIN).unwrap();
        assert_eq!(a.name(), "amix3");
        let b: User = serde_json::from_str(USER_FULL).unwrap();
        assert_eq!(b.id, 5);
        assert_eq!(b.karma, Some(33.5));
        let empty: User =
            serde_json::from_str(r#"{"id":2,"nick_name":"n","display_name":""}"#).unwrap();
        assert_eq!(empty.name(), "n");
    }

    #[test]
    fn bad_date_is_error_not_panic() {
        let r = serde_json::from_str::<Plurk>(
            &MINIMAL.replace("Fri, 05 Jun 2009 23:07:13 GMT", "garbage"),
        );
        assert!(r.is_err());
    }

    #[test]
    fn polling_shape() {
        let body = format!(
            r#"{{"plurks": [{PLURK}, {MINIMAL}], "plurk_users": {{"1": {USER_MIN}, "5": {USER_FULL}}}}}"#
        );
        let l: PlurkList = serde_json::from_str(&body).unwrap();
        assert_eq!(l.plurks.len(), 2);
        assert_eq!(l.plurk_users["5"].name(), "Alexey");
        let empty: PlurkList = serde_json::from_str("{}").unwrap();
        assert!(empty.plurks.is_empty());
    }
}
