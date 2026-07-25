//! Scheduled / recurring crawls.
//!
//! A [`Schedule`] fires a crawl on a daily, weekly, or monthly cadence
//! at a fixed UTC time. Schedules are persisted like every other domain
//! object (a [`JsonStore`](crate::state::JsonStore)); an in-process
//! ticker (see [`start_scheduler`]) wakes once a minute and runs any
//! schedule whose `next_run` has passed.
//!
//! **Startup catch-up:** the ticker's first tick fires immediately on
//! launch, so a schedule whose `next_run` elapsed while the app was
//! closed runs once at startup and is then rescheduled forward. This is
//! the in-process design chosen over an OS-level scheduler: no external
//! moving parts, and it fits the app's offline-first, single-user scope.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::settings::config::CrawlConfig;
use crate::state::{AppState, HasId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Cadence {
    Daily,
    Weekly,
    Monthly,
}

/// One recurring-crawl definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub id: String,
    pub name: String,
    pub url: String,
    pub config: CrawlConfig,
    pub cadence: Cadence,
    /// Fire time, UTC. `hour` 0–23, `minute` 0–59 (clamped on use).
    pub hour: u32,
    pub minute: u32,
    /// Weekly cadence only: 0 = Sunday … 6 = Saturday. Defaults to
    /// Monday when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekday: Option<u32>,
    /// Monthly cadence only: day of month, clamped to 1–28 so every
    /// month has it. Defaults to the 1st when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_of_month: Option<u32>,
    pub enabled: bool,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<String>,
    /// Next fire time as an RFC 3339 UTC string. Recomputed after every
    /// run and whenever the schedule is saved.
    pub next_run: String,
    /// Id of the most recent job this schedule spawned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_job_id: Option<String>,
}

impl HasId for Schedule {
    fn id(&self) -> &str {
        &self.id
    }
}

fn at(day: NaiveDate, hour: u32, minute: u32) -> Option<DateTime<Utc>> {
    day.and_hms_opt(hour, minute, 0)
        .map(|naive| Utc.from_utc_datetime(&naive))
}

/// Earliest fire time strictly after `after` that matches the schedule's
/// cadence and time-of-day. Pure and total: the day-stepping loops are
/// bounded (≤2 days for daily, ≤7 for weekly, ≤31 for monthly since the
/// day-of-month is clamped to 1–28).
pub fn compute_next_run(after: DateTime<Utc>, schedule: &Schedule) -> DateTime<Utc> {
    let hour = schedule.hour.min(23);
    let minute = schedule.minute.min(59);
    let mut day = after.date_naive();

    match schedule.cadence {
        Cadence::Daily => loop {
            if let Some(dt) = at(day, hour, minute) {
                if dt > after {
                    return dt;
                }
            }
            day = day.succ_opt().expect("date overflow");
        },
        Cadence::Weekly => {
            let target = schedule.weekday.unwrap_or(1).min(6);
            loop {
                if day.weekday().num_days_from_sunday() == target {
                    if let Some(dt) = at(day, hour, minute) {
                        if dt > after {
                            return dt;
                        }
                    }
                }
                day = day.succ_opt().expect("date overflow");
            }
        }
        Cadence::Monthly => {
            let target = schedule.day_of_month.unwrap_or(1).clamp(1, 28);
            loop {
                if day.day() == target {
                    if let Some(dt) = at(day, hour, minute) {
                        if dt > after {
                            return dt;
                        }
                    }
                }
                day = day.succ_opt().expect("date overflow");
            }
        }
    }
}

/// Spawn the background ticker that runs due schedules. Fires an
/// immediate first tick (startup catch-up) and then every 60 s.
pub fn start_scheduler(state: Arc<AppState>, app: AppHandle) {
    // `tauri::async_runtime::spawn` (not bare `tokio::spawn`) so this is
    // safe to call from `setup`, before a Tokio runtime context is
    // otherwise established on this thread.
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            run_due(&state, &app).await;
        }
    });
}

/// Run every enabled schedule whose `next_run` is in the past, then
/// advance each one to its next occurrence. A crawl that fails to spawn
/// (e.g. its stored config no longer passes validation) is skipped but
/// still rescheduled so one bad run doesn't wedge the timer.
async fn run_due(state: &Arc<AppState>, app: &AppHandle) {
    let now = Utc::now();

    let due: Vec<Schedule> = {
        let map = state.schedules.read().await;
        map.values()
            .filter(|s| s.enabled && is_past(&s.next_run, now))
            .cloned()
            .collect()
    };

    for mut schedule in due {
        if crate::commands::validate_crawl_input(&schedule.url, &schedule.config).is_ok() {
            match crate::commands::spawn_crawl(
                schedule.url.clone(),
                schedule.config.clone(),
                state.clone(),
                app.clone(),
                None,
            )
            .await
            {
                Ok(job_id) => schedule.last_job_id = Some(job_id),
                Err(_) => { /* spawn failed; still reschedule below */ }
            }
        }
        schedule.last_run = Some(now.to_rfc3339());
        schedule.next_run = compute_next_run(now, &schedule).to_rfc3339();
        let _ = state.schedules.insert(schedule).await;
    }
}

/// True when `rfc3339` parses to a time at or before `now`. An
/// unparseable timestamp is treated as *not* due (conservative — a
/// corrupt field shouldn't trigger unexpected crawls).
fn is_past(rfc3339: &str, now: DateTime<Utc>) -> bool {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|dt| dt.with_timezone(&Utc) <= now)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedule(cadence: Cadence, hour: u32, minute: u32) -> Schedule {
        Schedule {
            id: "s1".into(),
            name: "test".into(),
            url: "https://example.com".into(),
            config: CrawlConfig {
                output_dir: String::new(),
                max_depth: 2,
                page_limit: 10,
                download_assets: false,
                headless_strategy: "never".into(),
                content_selectors: vec![],
                exclude_patterns: vec![],
                include_patterns: vec![],
                path_prefix: String::new(),
                respect_robots_txt: true,
                stay_within_domain: true,
                ssrf_protection: true,
                profile: None,
            },
            cadence,
            hour,
            minute,
            weekday: None,
            day_of_month: None,
            enabled: true,
            created_at: "2026-07-25T00:00:00Z".into(),
            last_run: None,
            next_run: String::new(),
            last_job_id: None,
        }
    }

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    #[test]
    fn daily_advances_to_today_when_time_still_ahead() {
        let s = schedule(Cadence::Daily, 14, 30);
        // 09:00 → same day 14:30
        let next = compute_next_run(utc(2026, 7, 25, 9, 0), &s);
        assert_eq!(next, utc(2026, 7, 25, 14, 30));
    }

    #[test]
    fn daily_rolls_to_tomorrow_when_time_passed() {
        let s = schedule(Cadence::Daily, 8, 0);
        // 09:00 → next day 08:00
        let next = compute_next_run(utc(2026, 7, 25, 9, 0), &s);
        assert_eq!(next, utc(2026, 7, 26, 8, 0));
    }

    #[test]
    fn daily_is_strictly_after_even_at_exact_time() {
        let s = schedule(Cadence::Daily, 9, 0);
        // exactly 09:00 → must move to tomorrow, never return `after`
        let next = compute_next_run(utc(2026, 7, 25, 9, 0), &s);
        assert_eq!(next, utc(2026, 7, 26, 9, 0));
    }

    #[test]
    fn weekly_lands_on_target_weekday() {
        let mut s = schedule(Cadence::Weekly, 6, 0);
        s.weekday = Some(1); // Monday
        // 2026-07-25 is a Saturday → next Monday is 2026-07-27
        let next = compute_next_run(utc(2026, 7, 25, 12, 0), &s);
        assert_eq!(next.weekday().num_days_from_sunday(), 1);
        assert_eq!(next, utc(2026, 7, 27, 6, 0));
    }

    #[test]
    fn monthly_lands_on_day_of_month() {
        let mut s = schedule(Cadence::Monthly, 0, 0);
        s.day_of_month = Some(1);
        // mid-July → 1st of August
        let next = compute_next_run(utc(2026, 7, 15, 0, 0), &s);
        assert_eq!(next, utc(2026, 8, 1, 0, 0));
    }

    #[test]
    fn monthly_day_is_clamped_to_28() {
        let mut s = schedule(Cadence::Monthly, 0, 0);
        s.day_of_month = Some(31); // clamps to 28, present every month
        let next = compute_next_run(utc(2026, 2, 1, 0, 0), &s);
        assert_eq!(next, utc(2026, 2, 28, 0, 0));
    }

    #[test]
    fn is_past_handles_parse_and_ordering() {
        let now = utc(2026, 7, 25, 12, 0);
        assert!(is_past("2026-07-25T11:00:00+00:00", now));
        assert!(!is_past("2026-07-25T13:00:00+00:00", now));
        assert!(!is_past("garbage", now));
    }

    #[test]
    fn schedule_roundtrips_camelcase() {
        let mut s = schedule(Cadence::Weekly, 6, 30);
        s.weekday = Some(3);
        s.next_run = "2026-07-27T06:30:00+00:00".into();
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"cadence\":\"weekly\""));
        assert!(json.contains("\"nextRun\""));
        assert!(json.contains("\"dayOfMonth\"") == false); // None skipped
        let back: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(back.weekday, Some(3));
        assert_eq!(back.cadence, Cadence::Weekly);
    }
}
