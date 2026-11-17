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

#[derive(Deserialize, Debug, Default)]
pub struct ResponseList {
    #[serde(default)]
    pub responses: Vec<Response>,
    #[serde(default)]
    pub friends: HashMap<String, User>,
    #[serde(default)]
    pub responses_seen: u64,
}

#[derive(Deserialize, Debug)]
pub struct GetPlurk {
    #[serde(alias = "plurks")]
    pub plurk: Plurk,
    pub user: User,
}

#[derive(Deserialize, Debug)]
pub struct SuccessText {
    pub success_text: String,
}

#[derive(Deserialize, Debug)]
pub struct ReplurkItem {
    pub success: bool,
    #[serde(default)]
    pub error: String,
}

#[derive(Deserialize, Debug)]
pub struct ReplurkResult {
    pub success: bool,
    #[serde(default)]
    pub results: HashMap<String, ReplurkItem>,
}

#[derive(Deserialize, Debug, Default)]
pub struct UnreadCount {
    #[serde(default)]
    pub all: u64,
    #[serde(default)]
    pub my: u64,
    #[serde(default)]
    pub private: u64,
    #[serde(default)]
    pub responded: u64,
}

fn opt_rfc2822<'de, D>(d: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = Option::<String>::deserialize(d)?;
    Ok(s.and_then(|s| {
        DateTime::parse_from_rfc2822(&s)
            .ok()
            .map(|t| t.with_timezone(&Utc))
    }))
}

#[derive(Deserialize, Debug, Clone)]
pub struct Alert {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default, deserialize_with = "opt_rfc2822")]
    pub posted: Option<DateTime<Utc>>,
    pub plurk_id: Option<u64>,
    pub from_user: Option<User>,
    pub to_user: Option<User>,
    pub new_fan: Option<User>,
    pub new_friend: Option<User>,
    pub friend_info: Option<User>,
    pub owner: Option<User>,
}

impl Alert {
    pub fn actor(&self) -> Option<&User> {
        [
            &self.from_user,
            &self.new_fan,
            &self.new_friend,
            &self.friend_info,
            &self.owner,
            &self.to_user,
        ]
        .into_iter()
        .find_map(|u| u.as_ref())
    }
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

    const RESPONSES: &str = r#"{"friends": {"3": {"display_name": "", "nick_name": "alvin", "id": 3}}, "responses_seen": 2, "responses": [{"lang": "en", "content_raw": "Reforms", "content": "Reforms", "user_id": 3, "plurk_id": 90812, "id": 5, "qualifier": "says", "posted": "Fri, 05 Jun 2009 23:08:13 GMT"}, {"content": "anon", "user_id": 99999, "plurk_id": 90812, "id": 6, "qualifier": ":", "handle": "Blue Fox", "my_anonymous": false, "posted": "Fri, 05 Jun 2009 23:09:13 GMT"}]}"#;

    #[test]
    fn responses_shape() {
        let l: ResponseList = serde_json::from_str(RESPONSES).unwrap();
        assert_eq!(l.responses.len(), 2);
        assert_eq!(l.responses_seen, 2);
        assert_eq!(l.friends["3"].name(), "alvin");
        assert_eq!(l.responses[1].handle.as_deref(), Some("Blue Fox"));
        let empty: ResponseList = serde_json::from_str("{}").unwrap();
        assert!(empty.responses.is_empty());
    }

    #[test]
    fn get_plurk_accepts_both_keys() {
        for key in ["plurk", "plurks"] {
            let body = format!(r#"{{"{key}": {MINIMAL}, "user": {USER_MIN}}}"#);
            let g: GetPlurk = serde_json::from_str(&body).unwrap();
            assert_eq!(g.plurk.plurk_id, 90812);
            assert_eq!(g.user.name(), "amix3");
        }
    }

    #[test]
    fn ok_replies() {
        let s: SuccessText = serde_json::from_str(r#"{"success_text": "ok"}"#).unwrap();
        assert_eq!(s.success_text, "ok");
        let r: ReplurkResult = serde_json::from_str(
            r#"{"success": true, "results": {"342": {"success": true, "error":""}}}"#,
        )
        .unwrap();
        assert!(r.success && r.results["342"].success);
        let r: ReplurkResult = serde_json::from_str(
            r#"{"success": false, "results": {"1": {"success": false, "error": "x"}}}"#,
        )
        .unwrap();
        assert_eq!(r.results["1"].error, "x");
    }

    #[test]
    fn unread_count() {
        let u: UnreadCount =
            serde_json::from_str(r#"{"all": 2, "my": 1, "private": 1, "responded": 0}"#).unwrap();
        assert_eq!((u.all, u.my, u.private, u.responded), (2, 1, 1, 0));
    }

    #[test]
    fn alert_samples() {
        let u = r#"{"id": 9, "nick_name": "frodo_b"}"#;
        let d = "Fri, 05 Jun 2009 23:07:13 GMT";
        let cases = [
            ("friendship_request", "from_user", false),
            ("friendship_pending", "to_user", false),
            ("new_fan", "new_fan", false),
            ("friendship_accepted", "friend_info", false),
            ("new_friend", "new_friend", false),
            ("private_plurk", "owner", true),
            ("plurk_liked", "from_user", true),
            ("plurk_replurked", "from_user", true),
            ("mentioned", "from_user", true),
            ("my_responded", "from_user", true),
        ];
        for (kind, field, has_plurk) in cases {
            let extra = if has_plurk {
                r#", "plurk_id": 77, "num_others": 1, "response_id": null"#
            } else {
                ""
            };
            let body = format!(r#"{{"type": "{kind}", "{field}": {u}, "posted": "{d}"{extra}}}"#);
            let a: Alert = serde_json::from_str(&body).unwrap();
            assert_eq!(a.kind, kind);
            assert_eq!(a.actor().unwrap().nick_name, "frodo_b");
            assert_eq!(a.plurk_id.is_some(), has_plurk);
            assert!(a.posted.is_some());
        }
        let a: Alert = serde_json::from_str(r#"{"type": "future_kind"}"#).unwrap();
        assert!(a.actor().is_none() && a.posted.is_none());
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
