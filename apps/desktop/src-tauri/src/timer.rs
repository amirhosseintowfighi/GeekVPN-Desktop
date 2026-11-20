//! «قطع خودکار»: the running connection ends by itself after a while (for
//! the night, or to save the quota). The deadline lives here, not in the
//! tunnel: a watcher looks every few seconds, disconnects when it passes,
//! and forgets it whenever the tunnel goes down by any other road.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::desktop::notify;
use crate::servers::ServersState;
use crate::tunnel::TunnelState;

pub const CHANGED: &str = "timer://changed";
const EVERY: Duration = Duration::from_secs(5);
const MAX_MINUTES: u32 = 24 * 60;

#[derive(Default)]
pub struct TimerState {
    /// Unix ms when the connection should end.
    deadline: Mutex<Option<u64>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerView {
    pub deadline_ms: Option<u64>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or_default()
}

/// What the watcher should do at `now` with `deadline`, the tunnel up or not.
#[derive(Debug, PartialEq, Eq)]
pub enum Tick {
    Wait,
    /// The tunnel went down some other way: drop the deadline.
    Forget,
    Disconnect,
}

pub fn tick(deadline: Option<u64>, up: bool, now: u64) -> Tick {
    match deadline {
        None => Tick::Wait,
        Some(_) if !up => Tick::Forget,
        Some(d) if now >= d => Tick::Disconnect,
        Some(_) => Tick::Wait,
    }
}

fn set(app: &AppHandle, deadline: Option<u64>) -> TimerView {
    let t = app.state::<TimerState>();
    *t.deadline.lock().unwrap_or_else(|e| e.into_inner()) = deadline;
    let v = TimerView { deadline_ms: deadline };
    let _ = app.emit(CHANGED, v.clone());
    v
}

/// The connection ended (any way): its deadline goes with it at once, so a
/// quick reconnect does not inherit it.
pub fn forget(app: &AppHandle) {
    let Some(t) = app.try_state::<TimerState>() else { return };
    let had = t.deadline.lock().unwrap_or_else(|e| e.into_inner()).take().is_some();
    if had {
        let _ = app.emit(CHANGED, TimerView { deadline_ms: None });
    }
}

pub fn watch(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(EVERY).await;
            let deadline = *app.state::<TimerState>().deadline.lock().unwrap_or_else(|e| e.into_inner());
            if deadline.is_none() {
                continue;
            }
            let servers = app.state::<ServersState>();
            let up = matches!(*servers.tunnel.state.lock().await, TunnelState::On { .. } | TunnelState::Connecting { .. });
            match tick(deadline, up, now_ms()) {
                Tick::Wait => {}
                Tick::Forget => {
                    set(&app, None);
                }
                Tick::Disconnect => {
                    set(&app, None);
                    servers.tunnel.disconnect(&app).await;
                    notify(&app, "قطع خودکار", "زمانی که برای اتصال گذاشته بودی تمام شد و VPN قطع شد.");
                }
            }
        }
    });
}

#[tauri::command]
pub fn timer_get(timer: State<'_, TimerState>) -> TimerView {
    TimerView { deadline_ms: *timer.deadline.lock().unwrap_or_else(|e| e.into_inner()) }
}

/// `minutes: None` cancels. Only while connected.
#[tauri::command]
pub async fn timer_set(app: AppHandle, minutes: Option<u32>) -> Result<TimerView, String> {
    let Some(m) = minutes else { return Ok(set(&app, None)) };
    if m == 0 || m > MAX_MINUTES {
        return Err("زمان باید بین ۱ دقیقه و ۲۴ ساعت باشد.".into());
    }
    let up = matches!(*app.state::<ServersState>().tunnel.state.lock().await, TunnelState::On { .. });
    if !up {
        return Err("اول وصل شو.".into());
    }
    Ok(set(&app, Some(now_ms() + u64::from(m) * 60_000)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ends_only_a_running_connection_on_time() {
        assert_eq!(tick(None, true, 10), Tick::Wait);
        assert_eq!(tick(Some(100), true, 99), Tick::Wait);
        assert_eq!(tick(Some(100), true, 100), Tick::Disconnect);
        assert_eq!(tick(Some(100), false, 50), Tick::Forget);
    }
}
