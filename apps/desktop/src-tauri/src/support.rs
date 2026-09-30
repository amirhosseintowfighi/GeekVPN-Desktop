//! «پشتیبانی»: the customer's tickets, their threads and replies, a new
//! ticket, «گزارش مشکل», and a watch that brings a support reply to the
//! sidebar badge and the notification center while the app sits by the clock.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};

use geek_api::{Ticket, TicketMessage};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::auth::{call, AuthState};
use crate::report::{compose, Facts};
use crate::servers::ServersState;
use crate::store::Mode;
use crate::tunnel::TunnelState;

pub const UNREAD_EVENT: &str = "support://unread";
const WATCH_EVERY: Duration = Duration::from_secs(90);
/// The bot's own floor, so what is accepted here is accepted there.
const MIN_MESSAGE: usize = 10;
const MAX_MESSAGE: usize = 4_000;
const TOPICS: &[&str] = &["connection", "payment", "account", "speed", "other"];

pub struct SupportState {
    /// Unread support messages over all tickets; -1 until first known.
    unread: AtomicI64,
    /// Ticket id → its newest message this computer has shown. The route
    /// that reads a thread does not mark it read on the server, so without
    /// this an answer the customer has read would stay «جدید» for good.
    seen: Mutex<HashMap<String, DateTime<Utc>>>,
    seen_file: PathBuf,
}

impl SupportState {
    pub fn new(seen_file: PathBuf) -> Self {
        let seen = std::fs::read(&seen_file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self { unread: AtomicI64::new(-1), seen: Mutex::new(seen), seen_file }
    }

    /// The server's unread counts, less what was read here.
    fn adjust(&self, mut tickets: Vec<Ticket>) -> Vec<Ticket> {
        let seen = self.seen.lock().unwrap();
        for t in &mut tickets {
            if let Some(at) = seen.get(&t.ticket_id) {
                // The ticket's time moves with the reply's own write, a moment
                // after the message is stamped.
                if t.last_reply_at.unwrap_or(t.created_at) <= *at + chrono::Duration::seconds(2) {
                    t.unread_count = 0;
                }
            }
        }
        tickets
    }

    fn mark_seen(&self, ticket_id: &str, newest: DateTime<Utc>) {
        let mut seen = self.seen.lock().unwrap();
        seen.insert(ticket_id.to_string(), newest);
        if let Ok(json) = serde_json::to_vec(&*seen) {
            let _ = std::fs::write(&self.seen_file, json);
        }
    }
}

/// The thread that moved last first, from either side.
fn sorted(mut tickets: Vec<Ticket>) -> Vec<Ticket> {
    tickets.sort_by_key(|t| std::cmp::Reverse(t.last_reply_at.unwrap_or(t.created_at)));
    tickets
}

/// Updates the badge; tells the customer when a new answer came in.
fn publish(app: &AppHandle, tickets: &[Ticket]) {
    let Some(state) = app.try_state::<SupportState>() else {
        return;
    };
    let count: i64 = tickets.iter().map(|t| i64::from(t.unread_count)).sum();
    let before = state.unread.swap(count, Ordering::SeqCst);
    if before >= 0 && count > before {
        let topic = tickets
            .iter()
            .find(|t| t.unread_count > 0)
            .map(|t| t.topic_fa.clone())
            .unwrap_or_default();
        crate::desktop::notify(
            app,
            "پشتیبانی جواب داد",
            &format!("{topic} · در «پشتیبانی» بخوان."),
        );
    }
    if before != count {
        let _ = app.emit(UNREAD_EVENT, json!({ "count": count }));
    }
}

#[tauri::command]
pub async fn tickets_list(
    app: AppHandle,
    auth: State<'_, AuthState>,
) -> Result<Vec<Ticket>, String> {
    let tickets = sorted(call(&app, &auth, |api, t| async move { api.tickets(&t).await }).await?);
    let tickets = match app.try_state::<SupportState>() {
        Some(s) => s.adjust(tickets),
        None => tickets,
    };
    publish(&app, &tickets);
    Ok(tickets)
}

#[tauri::command]
pub fn support_unread(state: State<'_, SupportState>) -> i64 {
    state.unread.load(Ordering::SeqCst).max(0)
}

#[tauri::command]
pub async fn ticket_thread(
    app: AppHandle,
    auth: State<'_, AuthState>,
    ticket_id: String,
) -> Result<Vec<TicketMessage>, String> {
    let messages = call(&app, &auth, |api, t| {
        let id = ticket_id.clone();
        async move { api.ticket_messages(&t, &id).await }
    })
    .await?;
    if let (Some(s), Some(newest)) = (app.try_state::<SupportState>(), messages.iter().map(|m| m.created_at).max()) {
        s.mark_seen(&ticket_id, newest);
    }
    Ok(messages)
}

fn check_message(message: &str) -> Result<String, String> {
    let m = message.trim();
    match m.chars().count() {
        n if n < MIN_MESSAGE => Err(format!("پیام حداقل {} حرف باشد.", fa(MIN_MESSAGE))),
        n if n > MAX_MESSAGE => Err(format!("پیام حداکثر {} حرف باشد.", fa(MAX_MESSAGE))),
        _ => Ok(m.to_string()),
    }
}

fn fa(n: usize) -> String {
    n.to_string()
        .chars()
        .map(|c| {
            c.to_digit(10)
                .and_then(|d| char::from_u32(0x06F0 + d))
                .unwrap_or(c)
        })
        .collect()
}

#[tauri::command]
pub async fn ticket_reply(
    app: AppHandle,
    auth: State<'_, AuthState>,
    ticket_id: String,
    message: String,
) -> Result<TicketMessage, String> {
    let message = check_message(&message)?;
    call(&app, &auth, |api, t| {
        let (id, m) = (ticket_id.clone(), message.clone());
        async move { api.ticket_reply(&t, &id, &m).await }
    })
    .await
}

#[tauri::command]
pub async fn ticket_open(
    app: AppHandle,
    auth: State<'_, AuthState>,
    topic: String,
    subject: String,
    message: String,
) -> Result<Ticket, String> {
    if !TOPICS.contains(&topic.as_str()) {
        return Err("موضوع تیکت را انتخاب کن.".into());
    }
    let subject: String = subject.trim().chars().take(128).collect();
    let message = check_message(&message)?;
    call(&app, &auth, |api, t| {
        let (topic, subject, m) = (topic.clone(), subject.clone(), message.clone());
        async move { api.open_ticket(&t, &topic, &subject, &m).await }
    })
    .await
}

/// What «گزارش مشکل» would send, for the customer to read first.
#[tauri::command]
pub async fn report_preview(app: AppHandle, description: String) -> Result<String, String> {
    Ok(build_report(&app, &description).await)
}

/// Opens a «مشکل اتصال» ticket with the report; answers its reference.
#[tauri::command]
pub async fn report_send(
    app: AppHandle,
    auth: State<'_, AuthState>,
    description: String,
) -> Result<String, String> {
    if description.trim().chars().count() < MIN_MESSAGE {
        return Err(format!(
            "چه اتفاقی افتاد؟ حداقل {} حرف بنویس.",
            fa(MIN_MESSAGE)
        ));
    }
    let report = build_report(&app, &description).await;
    let ticket = call(&app, &auth, |api, t| {
        let r = report.clone();
        async move {
            api.open_ticket(&t, "connection", "گزارش مشکل اتصال (دسکتاپ)", &r)
                .await
        }
    })
    .await?;
    Ok(ticket.reference)
}

async fn build_report(app: &AppHandle, description: &str) -> String {
    let servers = app.state::<ServersState>();
    let tunnel = &servers.tunnel;
    let (mode, kill_switch, route, auto_server, config) = {
        let store = servers.store.lock().await;
        let d = &store.data;
        let config = d
            .selected
            .as_deref()
            .and_then(|id| store.find_view(id))
            .map(|v| {
                let source = d
                    .sources
                    .iter()
                    .find(|s| s.id == v.source_id)
                    .map(|s| s.kind.label())
                    .unwrap_or("manual link");
                let s = &v.server;
                format!(
                    "{} {} {} :{} ({source})",
                    format!("{:?}", s.protocol).to_lowercase(),
                    s.network,
                    if s.security.is_empty() {
                        "none"
                    } else {
                        &s.security
                    },
                    s.port
                )
            });
        (d.mode, d.kill_switch, d.route, d.auto_select, config)
    };
    let connected = matches!(*tunnel.state.lock().await, TunnelState::On { .. });
    let mut core = String::new();
    let mut log = vec![];
    if let Ok(c) = tunnel.core().await {
        if let Ok(v) = c.call("version", json!({}), Duration::from_secs(3)).await {
            core = format!(
                "geekcore {}, xray {}",
                v["geekcore"].as_str().unwrap_or("?"),
                v["xray"].as_str().unwrap_or("?")
            );
        }
        log = c
            .log_tail()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_string)
            .collect();
        let keep = log.len().saturating_sub(80);
        log.drain(..keep);
    }
    if mode == Mode::Tun {
        if let Ok((_, hello)) = tunnel.helper().await {
            core += &format!(", sing-box {}, helper {}", hello.sing_box, hello.version);
        }
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let failure = tunnel.last_failure.lock().unwrap().clone();
    let facts = Facts {
        app_version: app.package_info().version.to_string(),
        os: format!(
            "{} ({})",
            sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.into()),
            std::env::consts::ARCH
        ),
        network: network_kind(),
        mode: match (mode, kill_switch) {
            (Mode::Tun, true) => "tun, kill switch".into(),
            (Mode::Tun, false) => "tun".into(),
            (Mode::Proxy, _) => "system proxy".into(),
        },
        route: serde_json::to_value(route)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default(),
        auto_server,
        connected,
        core: if core.is_empty() {
            "unavailable".into()
        } else {
            core
        },
        config,
        last_failure: failure.as_ref().map(|f| f.0.clone()),
        last_failure_minutes_ago: failure.map(|f| now.saturating_sub(f.1) / 60_000),
    };
    compose(description, &facts, &log)
}

/// The kind of the interface the default route leaves by.
fn network_kind() -> String {
    use netdev::interface::types::InterfaceType as T;
    match netdev::get_default_interface() {
        Ok(i) => match i.if_type {
            T::Wireless80211 => "wifi".into(),
            T::Ethernet | T::GigabitEthernet | T::FastEthernetT | T::FastEthernetFx => {
                "ethernet".into()
            }
            T::Ppp | T::Wwanpp | T::Wwanpp2 => "mobile / ppp".into(),
            other => format!("{other:?}").to_lowercase(),
        },
        Err(_) => "none".into(),
    }
}

/// Opens the bot's chat, for «ربات پشتیبانی».
#[tauri::command]
pub fn support_open_bot(app: AppHandle) -> Result<(), String> {
    let bot = crate::shop::bot_username().ok_or("این نسخه نام ربات را ندارد.")?;
    crate::auth::open_telegram(&app, &format!("https://t.me/{bot}"));
    Ok(())
}

/// Every 90 seconds while signed in: the unread count, so a reply shows on
/// the badge and as a notification without the page open.
pub fn watch(app: AppHandle, session: Arc<geek_api::Session>) {
    tauri::async_runtime::spawn(async move {
        loop {
            if session.user().await.is_some() {
                let api = session.api().clone();
                let got = session
                    .authorized(|t| {
                        let api = api.clone();
                        async move { api.tickets(&t).await }
                    })
                    .await;
                if let (Ok(tickets), Some(s)) = (got, app.try_state::<SupportState>()) {
                    publish(&app, &s.adjust(tickets));
                }
            } else if let Some(s) = app.try_state::<SupportState>() {
                s.unread.store(-1, Ordering::SeqCst);
            }
            tokio::time::sleep(WATCH_EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn messages_meet_the_bots_floor() {
        assert!(super::check_message("  کوتاه  ").is_err());
        assert_eq!(
            super::check_message("  از صبح وصل نمی‌شود  ").unwrap(),
            "از صبح وصل نمی‌شود"
        );
        assert!(super::check_message(&"x".repeat(4001)).is_err());
        assert_eq!(super::fa(10), "۱۰");
    }
}
