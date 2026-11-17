use crate::client::Client;
use crate::error::{Error, Result};
use crate::model::{Alert, GetPlurk, ReplurkResult, ResponseList, SuccessText, UnreadCount};
use crate::render::{self, base36_decode, plurk_url};
use serde::Deserialize;

pub const PLURK_HELP: &str = "Plurk id: numeric id, base36 id (plurk.com/p/<id>), or a full plurk.com/p/<id> URL. An all-digit value is numeric, unless it comes from a URL, where it is base36";

pub type Params = Vec<(&'static str, String)>;

pub fn parse_plurk_ref(input: &str) -> std::result::Result<u64, String> {
    let s = input.trim();
    let invalid = || format!("invalid plurk id: {input}");
    if s.starts_with("http://") || s.starts_with("https://") {
        let (_, rest) = s.split_once("/p/").ok_or_else(invalid)?;
        let id = rest.split(['/', '?', '#']).next().unwrap_or("");
        return base36_decode(id).ok_or_else(invalid);
    }
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        return s.parse().map_err(|_| invalid());
    }
    base36_decode(s).ok_or_else(invalid)
}

pub fn post_params(
    content: &str,
    qualifier: &str,
    private: bool,
    no_comments: Option<u8>,
    lang: Option<&str>,
) -> Params {
    let mut p: Params = vec![
        ("content", content.to_string()),
        ("qualifier", qualifier.to_string()),
    ];
    if private {
        p.push(("limited_to", "[0]".to_string()));
    }
    if let Some(n) = no_comments {
        p.push(("no_comments", n.to_string()));
    }
    if let Some(l) = lang {
        p.push(("lang", l.to_string()));
    }
    p
}

pub fn respond_params(plurk_id: u64, content: &str, qualifier: &str) -> Params {
    vec![
        ("plurk_id", plurk_id.to_string()),
        ("content", content.to_string()),
        ("qualifier", qualifier.to_string()),
    ]
}

#[derive(Deserialize)]
struct PlurkAdded {
    plurk_id: u64,
}

#[derive(Deserialize)]
struct ResponseAdded {
    id: u64,
}

pub async fn post(
    client: &Client,
    content: &str,
    qualifier: &str,
    private: bool,
    no_comments: Option<u8>,
    lang: Option<&str>,
) -> Result<()> {
    let params = post_params(content, qualifier, private, no_comments, lang);
    let added: PlurkAdded = client.call("/Timeline/plurkAdd", &params).await?;
    println!("{}", plurk_url(added.plurk_id));
    Ok(())
}

pub async fn respond(client: &Client, plurk_id: u64, content: &str, qualifier: &str) -> Result<()> {
    let params = respond_params(plurk_id, content, qualifier);
    let added: ResponseAdded = client.call("/Responses/responseAdd", &params).await?;
    println!("{} (response {})", plurk_url(plurk_id), added.id);
    Ok(())
}

pub fn get_plurk_params(plurk_id: u64) -> Params {
    vec![("plurk_id", plurk_id.to_string())]
}

pub fn responses_params(plurk_id: u64, count: Option<u64>) -> Params {
    let mut p = get_plurk_params(plurk_id);
    if let Some(c) = count {
        p.push(("count", c.to_string()));
    }
    p
}

pub async fn show(client: &Client, plurk_id: u64, count: Option<u64>) -> Result<()> {
    let plurk: GetPlurk = client
        .call("/Timeline/getPlurk", &get_plurk_params(plurk_id))
        .await?;
    let list: ResponseList = client
        .call("/Responses/get", &responses_params(plurk_id, count))
        .await?;
    render::print_show(&plurk, &list);
    Ok(())
}

pub async fn unread(client: &Client) -> Result<()> {
    let u: UnreadCount = client.call("/Polling/getUnreadCount", &[]).await?;
    render::print_unread(&u);
    Ok(())
}

pub fn alerts_path(history: bool) -> &'static str {
    if history {
        "/Alerts/getHistory"
    } else {
        "/Alerts/getActive"
    }
}

pub async fn alerts(client: &Client, history: bool) -> Result<()> {
    let list: Vec<Alert> = client.call(alerts_path(history), &[]).await?;
    for a in &list {
        println!("{}", render::alert_line(a));
    }
    Ok(())
}

pub fn ids_params(ids: &[u64]) -> Params {
    let list = ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
    vec![("ids", format!("[{list}]"))]
}

pub async fn set_flag(client: &Client, path: &str, ids: &[u64]) -> Result<()> {
    let r: SuccessText = client.call(path, &ids_params(ids)).await?;
    println!("{}", r.success_text);
    Ok(())
}

pub fn replurk_failures(r: &ReplurkResult) -> Vec<String> {
    let mut failed: Vec<_> = r
        .results
        .iter()
        .filter(|(_, i)| !i.success)
        .map(|(id, i)| format!("{id}: {}", i.error))
        .collect();
    failed.sort();
    failed
}

pub async fn replurk(client: &Client, path: &str, ids: &[u64]) -> Result<()> {
    let r: ReplurkResult = client.call(path, &ids_params(ids)).await?;
    let failed = replurk_failures(&r);
    for line in &failed {
        eprintln!("failed {line}");
    }
    println!("{}/{} ok", r.results.len() - failed.len(), r.results.len());
    if r.success && failed.is_empty() {
        Ok(())
    } else {
        Err(Error::Api {
            status: 200,
            message: format!("{} plurk(s) failed", failed.len().max(1)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plurk_ref_forms() {
        assert_eq!(parse_plurk_ref("241403675"), Ok(241403675));
        assert_eq!(parse_plurk_ref("3zq49n"), Ok(241403675));
        assert_eq!(
            parse_plurk_ref("https://www.plurk.com/p/3zq49n"),
            Ok(241403675)
        );
        assert_eq!(
            parse_plurk_ref("https://www.plurk.com/p/3zq49n?x=1#y"),
            Ok(241403675)
        );
    }

    #[test]
    fn all_digit_ambiguity() {
        assert_eq!(parse_plurk_ref("100"), Ok(100));
        assert_eq!(parse_plurk_ref("https://www.plurk.com/p/100"), Ok(36 * 36));
    }

    #[test]
    fn plurk_ref_rejects_garbage() {
        for s in ["", "-1", "https://www.plurk.com/", "https://x/p/", "a b"] {
            assert!(parse_plurk_ref(s).is_err(), "{s}");
        }
    }

    #[test]
    fn private_post_sends_limited_to() {
        let p = post_params("hi", ":", true, None, None);
        assert!(p.contains(&("limited_to", "[0]".to_string())));
        assert!(p.contains(&("qualifier", ":".to_string())));
        let p = post_params("hi", "says", false, Some(2), Some("en"));
        assert!(!p.iter().any(|(k, _)| *k == "limited_to"));
        assert!(p.contains(&("no_comments", "2".to_string())));
        assert!(p.contains(&("lang", "en".to_string())));
    }

    #[test]
    fn show_params() {
        assert_eq!(
            responses_params(7, None),
            vec![("plurk_id", "7".to_string())]
        );
        assert!(responses_params(7, Some(5)).contains(&("count", "5".to_string())));
    }

    #[test]
    fn alerts_endpoint() {
        assert_eq!(alerts_path(false), "/Alerts/getActive");
        assert_eq!(alerts_path(true), "/Alerts/getHistory");
    }

    #[test]
    fn ids_are_json_list() {
        assert_eq!(
            ids_params(&[342, 23242]),
            vec![("ids", "[342,23242]".to_string())]
        );
    }

    #[test]
    fn replurk_failures_reported() {
        let r: ReplurkResult = serde_json::from_str(
            r#"{"success": false, "results": {"1": {"success": true, "error": ""}, "2": {"success": false, "error": "no"}}}"#,
        )
        .unwrap();
        assert_eq!(replurk_failures(&r), ["2: no"]);
    }

    #[test]
    fn respond_param_vector() {
        assert_eq!(
            respond_params(7, "yo", ":"),
            vec![
                ("plurk_id", "7".to_string()),
                ("content", "yo".to_string()),
                ("qualifier", ":".to_string())
            ]
        );
    }
}
