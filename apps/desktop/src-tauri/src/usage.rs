//! «مصرف روزانه»: VPN traffic of this computer, per local day, kept 45 days
//! (the Android app's `DailyUsage`). The server's per-service history counts
//! every device on a service; this is the one number only the app can know.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDate};
use serde::Serialize;
use tauri::State;

use crate::servers::ServersState;

const KEEP_DAYS: i64 = 45;
const SAVE_EVERY: Duration = Duration::from_secs(30);

#[derive(Default)]
struct Book {
    /// `YYYY-MM-DD` → bytes up and down together.
    days: BTreeMap<String, u64>,
    /// The engine's counter at the last sample.
    last: Option<i64>,
    saved: Option<Instant>,
    dirty: bool,
}

pub struct Usage {
    path: PathBuf,
    book: Mutex<Book>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Day {
    day: String,
    bytes: u64,
}

impl Usage {
    pub fn load(path: PathBuf) -> Self {
        let days = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            path,
            book: Mutex::new(Book {
                days,
                ..Default::default()
            }),
        }
    }

    /// A new engine counts from zero.
    pub fn begin(&self) {
        if let Ok(mut b) = self.book.lock() {
            b.last = Some(0);
        }
    }

    /// Books what moved since the last sample to today. A counter that went
    /// backwards (the engine restarted for a failover) books nothing.
    pub fn sample(&self, total: i64) {
        self.sample_on(total, Local::now().date_naive());
    }

    fn sample_on(&self, total: i64, today: NaiveDate) {
        let Ok(mut b) = self.book.lock() else { return };
        let moved = b.last.map(|l| total - l).filter(|d| *d > 0);
        b.last = Some(total);
        if let Some(d) = moved {
            *b.days.entry(today.to_string()).or_default() += d as u64;
            let oldest = (today - chrono::Duration::days(KEEP_DAYS)).to_string();
            b.days.retain(|day, _| *day > oldest);
            b.dirty = true;
        }
        if b.dirty && b.saved.is_none_or(|s| s.elapsed() > SAVE_EVERY) {
            self.save(&mut b);
        }
    }

    pub fn flush(&self) {
        if let Ok(mut b) = self.book.lock() {
            b.last = None;
            if b.dirty {
                self.save(&mut b);
            }
        }
    }

    fn save(&self, b: &mut Book) {
        if let Ok(json) = serde_json::to_vec(&b.days) {
            let tmp = self.path.with_extension("tmp");
            if let Ok(mut f) = std::fs::File::create(&tmp) {
                use std::io::Write;
                if f.write_all(&json).is_ok() && f.sync_all().is_ok() && std::fs::rename(&tmp, &self.path).is_ok() {
                    b.dirty = false;
                }
            }
        }
        b.saved = Some(Instant::now());
    }

    /// The last `n` days up to today, oldest first, zeros included.
    fn last_days(&self, n: i64, today: NaiveDate) -> Vec<Day> {
        let b = self.book.lock().map(|b| b.days.clone()).unwrap_or_default();
        (0..n)
            .rev()
            .map(|back| {
                let day = (today - chrono::Duration::days(back)).to_string();
                Day {
                    bytes: b.get(&day).copied().unwrap_or(0),
                    day,
                }
            })
            .collect()
    }
}

#[tauri::command]
pub fn usage_local(state: State<'_, ServersState>, days: u32) -> Vec<Day> {
    state.tunnel.usage.last_days(
        i64::from(days.clamp(1, KEEP_DAYS as u32)),
        Local::now().date_naive(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn books_deltas_per_day_and_survives_a_restart() {
        let dir = std::env::temp_dir().join(format!("geek-usage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("usage.json");
        let _ = std::fs::remove_file(&path);

        let u = Usage::load(path.clone());
        u.begin();
        u.sample_on(1000, day("2026-09-29"));
        u.sample_on(1500, day("2026-09-30"));
        // The engine restarted for a failover: its counter starts over.
        u.sample_on(200, day("2026-09-30"));
        u.sample_on(700, day("2026-09-30"));
        u.flush();

        let again = Usage::load(path);
        let days = again.last_days(3, day("2026-09-30"));
        assert_eq!(
            days,
            vec![
                Day {
                    day: "2026-09-28".into(),
                    bytes: 0
                },
                Day {
                    day: "2026-09-29".into(),
                    bytes: 1000
                },
                Day {
                    day: "2026-09-30".into(),
                    bytes: 1000
                },
            ]
        );
    }

    #[test]
    fn forgets_days_older_than_45() {
        let u = Usage::load(std::env::temp_dir().join("geek-usage-none/usage.json"));
        u.begin();
        u.sample_on(10, day("2026-01-01"));
        u.sample_on(20, day("2026-03-01"));
        assert_eq!(u.book.lock().unwrap().days.len(), 1);
    }
}
