use crate::client::Client;
use crate::comet::Event;
use crate::model::{Alert, GetPlurk, Plurk, Profile, Response, ResponseList, UnreadCount, User};
use chrono::{DateTime, Local, Utc};
use colored::Colorize;
use std::collections::HashMap;
use terminal_size::{Width, terminal_size};

pub fn base36(mut v: u64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    loop {
        out.push(DIGITS[(v % 36) as usize]);
        v /= 36;
        if v == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

pub fn base36_decode(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    s.chars().try_fold(0u64, |acc, c| {
        let d = c.to_digit(36)?;
        acc.checked_mul(36)?.checked_add(u64::from(d))
    })
}

pub fn plurk_url(id: u64) -> String {
    format!("https://www.plurk.com/p/{}", base36(id))
}

pub fn local_time(t: &DateTime<Utc>) -> String {
    t.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn separator() {
    if let Some((Width(w), _)) = terminal_size() {
        println!("{}", "=".repeat(w.into()));
    }
}

async fn display_name(client: &Client, users: &HashMap<String, User>, id: u64) -> String {
    if let Some(u) = users.get(&id.to_string()) {
        return u.name().to_string();
    }
    let profile: crate::error::Result<Profile> = client
        .call("/Profile/getPublicProfile", &[("user_id", id.to_string())])
        .await;
    match profile {
        Ok(p) => p.user_info.name().to_string(),
        Err(_) => id.to_string(),
    }
}

pub fn print_me(user: &User) {
    println!(
        "PlurkUser {}({}) aka. {}",
        user.nick_name,
        user.id,
        user.name()
    );
}

pub async fn print_plurk_line(
    client: &Client,
    users: &HashMap<String, User>,
    p: &Plurk,
    verbose: bool,
) {
    let name = display_name(client, users, p.owner_id).await;
    let head = format!(
        "{} {} {}",
        local_time(&p.posted).bright_yellow(),
        name.bold().bright_blue(),
        p.qualifier.black().on_bright_white()
    );
    if verbose {
        println!("Plurk ==> {}", plurk_url(p.plurk_id));
        println!("{head}");
        println!("{}", p.text());
        separator();
    } else {
        println!("{head} {}", p.text().replace('\n', "   "));
    }
}

fn print_response(r: &Response, name: &str) {
    println!(
        "{} {} {} {}",
        local_time(&r.posted).yellow(),
        name.bold().bright_magenta(),
        r.qualifier.black().on_bright_white(),
        r.text()
    );
}

pub const ANONYMOUS_ID: u64 = 99999;

pub fn response_name(r: &Response, friends: &HashMap<String, User>) -> String {
    if let Some(h) = r.handle.as_deref().filter(|h| !h.is_empty()) {
        return h.to_string();
    }
    friends
        .get(&r.user_id.to_string())
        .map(|u| u.name().to_string())
        .unwrap_or_else(|| {
            if r.user_id == ANONYMOUS_ID {
                "anonymous".to_string()
            } else {
                r.user_id.to_string()
            }
        })
}

pub fn print_show(g: &GetPlurk, list: &ResponseList) {
    let p = &g.plurk;
    println!("Plurk ==> {}", plurk_url(p.plurk_id));
    println!(
        "{} {} {}",
        local_time(&p.posted).bright_yellow(),
        g.user.name().bold().bright_blue(),
        p.qualifier.black().on_bright_white()
    );
    println!("{}", p.text());
    separator();
    for r in &list.responses {
        print_response(r, &response_name(r, &list.friends));
    }
}

pub fn print_unread(u: &UnreadCount) {
    println!(
        "all: {}  my: {}  private: {}  responded: {}",
        u.all, u.my, u.private, u.responded
    );
}

pub fn alert_line(a: &Alert) -> String {
    let when = a
        .posted
        .as_ref()
        .map(local_time)
        .unwrap_or_else(|| "-".to_string());
    let mut line = format!("{when} {}", a.kind);
    if let Some(u) = a.actor() {
        line.push_str(&format!(" {}", u.nick_name));
    }
    if let Some(id) = a.plurk_id {
        line.push_str(&format!(" {}", plurk_url(id)));
    }
    line
}

pub async fn print_event(client: &Client, event: &Event) {
    match event {
        Event::NewPlurk(p) => {
            println!("New plurk ==> {}", plurk_url(p.plurk_id));
            print_plurk_line(client, &HashMap::new(), p, true).await;
        }
        Event::NewResponse {
            plurk_id,
            plurk,
            response,
            user,
        } => {
            println!("New response ==> {}", plurk_url(*plurk_id));
            if let Some(p) = plurk {
                let name = display_name(client, user, p.owner_id).await;
                println!(
                    "{} {} {}",
                    local_time(&p.posted).yellow(),
                    name.bold().bright_blue(),
                    p.qualifier.black().on_bright_white()
                );
                println!("{}", p.text());
                println!(" -------");
            }
            let name = display_name(client, user, response.user_id).await;
            print_response(response, &name);
            separator();
        }
        Event::UpdateNotification { counts } => {
            println!("Notification: noti={} req={}", counts.noti, counts.req);
        }
        Event::Unknown => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_names() {
        let list: ResponseList = serde_json::from_str(
            r#"{"friends": {"3": {"nick_name": "alvin", "id": 3}}, "responses": [
            {"id":1,"plurk_id":1,"user_id":3,"qualifier":":","content":"a","posted":"Fri, 05 Jun 2009 23:08:13 GMT"},
            {"id":2,"plurk_id":1,"user_id":99999,"qualifier":":","content":"b","handle":"Fox","posted":"Fri, 05 Jun 2009 23:08:13 GMT"},
            {"id":3,"plurk_id":1,"user_id":99999,"qualifier":":","content":"b","posted":"Fri, 05 Jun 2009 23:08:13 GMT"},
            {"id":4,"plurk_id":1,"user_id":8,"qualifier":":","content":"b","posted":"Fri, 05 Jun 2009 23:08:13 GMT"}]}"#,
        )
        .unwrap();
        let names: Vec<_> = list
            .responses
            .iter()
            .map(|r| response_name(r, &list.friends))
            .collect();
        assert_eq!(names, ["alvin", "Fox", "anonymous", "8"]);
    }

    #[test]
    fn alert_lines() {
        let a: Alert = serde_json::from_str(
            r#"{"type":"plurk_liked","from_user":{"id":1,"nick_name":"amix"},"posted":"Fri, 05 Jun 2009 23:07:13 GMT","plurk_id":241403675}"#,
        )
        .unwrap();
        assert!(alert_line(&a).ends_with("plurk_liked amix https://www.plurk.com/p/3zq49n"));
        let a: Alert = serde_json::from_str(r#"{"type":"weird"}"#).unwrap();
        assert_eq!(alert_line(&a), "- weird");
    }

    #[test]
    fn base36_urls() {
        assert_eq!(base36(0), "0");
        assert_eq!(base36(35), "z");
        assert_eq!(base36(36), "10");
        assert_eq!(plurk_url(241403675), "https://www.plurk.com/p/3zq49n");
    }

    #[test]
    fn base36_round_trip() {
        for v in [0, 1, 35, 36, 1295, 241403675, u64::MAX] {
            assert_eq!(base36_decode(&base36(v)), Some(v));
        }
        assert_eq!(base36_decode("3ZQ49N"), Some(241403675));
        assert_eq!(base36_decode(""), None);
        assert_eq!(base36_decode("a-b"), None);
        assert_eq!(base36_decode("zzzzzzzzzzzzzzzz"), None);
    }
}
