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

use crate::crawler::batch::{spawn_batch, BatchJob, BatchStatus};
use crate::settings::config::{BatchFailureMode, CrawlConfig};
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
    /// URLs to crawl. Exactly one spawns a plain crawl; two or more
    /// spawn a batch, so the batch queue is reachable from a schedule.
    pub urls: Vec<String>,
    pub config: CrawlConfig,
    /// Batch on-failure override, only meaningful with two or more URLs.
    /// `None` falls back to the app-settings default at run time — same
    /// rule the `start_batch` command applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<BatchFailureMode>,
    /// Id of the template this schedule's config was seeded from, kept
    /// for provenance in the UI. The config itself is a snapshot: later
    /// edits to the template do not retroactively change the schedule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
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
    /// Id of the most recent job this schedule spawned (single-URL runs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_job_id: Option<String>,
    /// Id of the most recent batch this schedule spawned (multi-URL runs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_batch_id: Option<String>,
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
        map.values().filter(|s| is_due(s, now)).cloned().collect()
    };

    for mut schedule in due {
        run_once(&mut schedule, state, app).await;
        advance(&mut schedule, now);
        let _ = state.schedules.insert(schedule).await;
    }
}

/// A schedule fires when it is enabled and its `next_run` has passed.
/// Pure, so the catch-up semantics are testable without an `AppHandle`.
fn is_due(schedule: &Schedule, now: DateTime<Utc>) -> bool {
    schedule.enabled && is_past(&schedule.next_run, now)
}

/// Stamp `last_run` and move `next_run` to the following occurrence.
/// Always called after a run attempt, successful or not.
fn advance(schedule: &mut Schedule, now: DateTime<Utc>) {
    schedule.last_run = Some(now.to_rfc3339());
    schedule.next_run = compute_next_run(now, schedule).to_rfc3339();
}

/// Spawn one schedule's work: a single URL goes through `spawn_crawl`,
/// two or more through the batch runner, so a schedule can drive the
/// batch queue. Records the spawned id on the schedule; every failure
/// path is deliberately silent because the caller reschedules either way.
async fn run_once(schedule: &mut Schedule, state: &Arc<AppState>, app: &AppHandle) {
    let urls: Vec<String> = schedule
        .urls
        .iter()
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
        .collect();

    // Revalidate against the current rules before spawning — a stored
    // schedule may predate a settings change (SSRF, robots, …).
    if urls.is_empty()
        || urls
            .iter()
            .any(|u| crate::commands::validate_crawl_input(u, &schedule.config).is_err())
    {
        return;
    }

    if urls.len() == 1 {
        if let Ok(job_id) = crate::commands::spawn_crawl(
            urls[0].clone(),
            schedule.config.clone(),
            state.clone(),
            app.clone(),
            None,
        )
        .await
        {
            schedule.last_job_id = Some(job_id);
        }
        return;
    }

    let on_failure = match schedule.on_failure {
        Some(mode) => mode,
        None => crate::commands::get_settings(app.clone())
            .await
            .map(|s| s.batch_on_failure)
            .unwrap_or_default(),
    };

    let batch = BatchJob {
        id: uuid::Uuid::new_v4().to_string(),
        name: Some(schedule.name.clone()),
        urls,
        config: schedule.config.clone(),
        on_failure,
        child_job_ids: Vec::new(),
        status: BatchStatus::Queued,
        current_index: 0,
        created_at: Utc::now().to_rfc3339(),
        error: None,
        start_time: None,
        end_time: None,
    };

    if let Ok(batch_id) = spawn_batch(batch, state.clone(), app.clone()).await {
        schedule.last_batch_id = Some(batch_id);
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
            urls: vec!["https://example.com".into()],
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
            on_failure: None,
            template_id: None,
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
            last_batch_id: None,
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
    fn elapsed_schedule_is_due_and_disabled_one_is_not() {
        let now = utc(2026, 7, 25, 12, 0);
        let mut s = schedule(Cadence::Daily, 9, 0);

        // Fired while the app was closed → due at the next tick, which
        // is the startup catch-up tick.
        s.next_run = utc(2026, 7, 24, 9, 0).to_rfc3339();
        assert!(is_due(&s, now));

        // Same schedule, paused: never fires.
        s.enabled = false;
        assert!(!is_due(&s, now));

        // Enabled but not yet reached.
        s.enabled = true;
        s.next_run = utc(2026, 7, 25, 18, 0).to_rfc3339();
        assert!(!is_due(&s, now));
    }

    #[test]
    fn advance_stamps_last_run_and_moves_next_run_ahead() {
        let now = utc(2026, 7, 25, 12, 0);
        let mut s = schedule(Cadence::Daily, 9, 0);
        s.next_run = utc(2026, 7, 24, 9, 0).to_rfc3339();

        advance(&mut s, now);

        assert_eq!(s.last_run.as_deref(), Some(now.to_rfc3339()).as_deref());
        // Advanced past a missed run rather than replaying it: the next
        // fire is the upcoming 09:00, not yesterday's.
        assert_eq!(s.next_run, utc(2026, 7, 26, 9, 0).to_rfc3339());
        assert!(!is_due(&s, now));
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
