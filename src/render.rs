use crate::client::Client;
use crate::comet::Event;
use crate::model::{Plurk, Profile, Response, User};
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
    fn base36_urls() {
        assert_eq!(base36(0), "0");
        assert_eq!(base36(35), "z");
        assert_eq!(base36(36), "10");
        assert_eq!(plurk_url(241403675), "https://www.plurk.com/p/3zq49n");
    }
}
