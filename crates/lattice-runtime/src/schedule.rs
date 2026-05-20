//! Cron expression parser and in-memory scheduler.
//!
//! Parses 5-field cron expressions (`min hour day month weekday`) and runs
//! registered jobs in a background thread.  For distributed scheduling
//! (leader election, persistent job store), implement [`Scheduler`] directly.

use crate::ports::Scheduler;
use crate::query::WorkItem;
use lattice_core::Error;
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Cron expression parser
// ---------------------------------------------------------------------------

/// A parsed 5-field cron expression.
///
/// Each field is either a wildcard (matches any value) or a set of allowed
/// integers.  Fields: minute (0–59), hour (0–23), day (1–31), month (1–12),
/// weekday (0–6, 0 = Sunday).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronSpec {
    pub(crate) minute: Field,
    pub(crate) hour: Field,
    pub(crate) day: Field,
    pub(crate) month: Field,
    pub(crate) weekday: Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Field {
    Any,
    Values(BTreeSet<u32>),
}

impl Field {
    fn matches(&self, v: u32) -> bool {
        match self {
            Self::Any => true,
            Self::Values(s) => s.contains(&v),
        }
    }
}

impl CronSpec {
    /// Parse a 5-field cron expression.
    ///
    /// Supported syntax: `*`, comma lists (`1,2,3`), ranges (`1-5`),
    /// step (`*/5`, `0-30/5`).
    pub fn parse(expr: &str) -> Result<Self, Error> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(Error::validation(format!(
                "cron: expected 5 fields, got {} in {:?}",
                parts.len(),
                expr
            )));
        }
        Ok(Self {
            minute: parse_field(parts[0], 0, 59)?,
            hour: parse_field(parts[1], 0, 23)?,
            day: parse_field(parts[2], 1, 31)?,
            month: parse_field(parts[3], 1, 12)?,
            weekday: parse_field(parts[4], 0, 6)?,
        })
    }

    /// Return `true` when the expression fires at the given UTC time.
    pub fn matches_utc(&self, t: &chrono::DateTime<chrono::Utc>) -> bool {
        use chrono::Datelike;
        use chrono::Timelike;
        self.minute.matches(t.minute())
            && self.hour.matches(t.hour())
            && self.day.matches(t.day())
            && self.month.matches(t.month())
            && self.weekday.matches(t.weekday().num_days_from_sunday())
    }
}

fn parse_field(s: &str, min: u32, max: u32) -> Result<Field, Error> {
    if s == "*" {
        return Ok(Field::Any);
    }
    let mut values = BTreeSet::new();
    for part in s.split(',') {
        if let Some((range, step)) = part.split_once('/') {
            let step: u32 = step
                .parse()
                .map_err(|_| Error::validation(format!("cron: invalid step {:?}", step)))?;
            if step == 0 {
                return Err(Error::validation("cron: step must be > 0"));
            }
            let (lo, hi) = if range == "*" {
                (min, max)
            } else if let Some((a, b)) = range.split_once('-') {
                (parse_int(a)?, parse_int(b)?)
            } else {
                let v = parse_int(range)?;
                (v, max)
            };
            let mut v = lo;
            while v <= hi {
                values.insert(v);
                v = v.saturating_add(step);
            }
        } else if let Some((a, b)) = part.split_once('-') {
            let lo = parse_int(a)?;
            let hi = parse_int(b)?;
            for v in lo..=hi {
                values.insert(v);
            }
        } else {
            values.insert(parse_int(part)?);
        }
    }
    for &v in &values {
        if v < min || v > max {
            return Err(Error::validation(format!(
                "cron: value {} out of range [{}, {}]",
                v, min, max
            )));
        }
    }
    Ok(Field::Values(values))
}

fn parse_int(s: &str) -> Result<u32, Error> {
    s.trim()
        .parse()
        .map_err(|_| Error::validation(format!("cron: {:?} is not an integer", s)))
}

// ---------------------------------------------------------------------------
// MemoryCronScheduler
// ---------------------------------------------------------------------------

struct Job {
    spec: CronSpec,
    task: WorkItem,
}

/// In-memory cron scheduler that fires registered jobs in a background thread.
///
/// The scheduler polls at 1-second resolution.  Jobs whose cron spec matches
/// the current UTC minute are dispatched to `handler` — a caller-supplied
/// closure — so the scheduler itself stays free of any execution logic.
///
/// For distributed or persistent scheduling, implement [`Scheduler`] directly.
pub struct MemoryCronScheduler {
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    counter: Mutex<u64>,
}

impl MemoryCronScheduler {
    /// Create a new scheduler and start the background tick thread.
    ///
    /// `handler` receives a [`WorkItem`] whenever a job fires.  It runs
    /// on the scheduler's background thread — keep it non-blocking or spawn
    /// additional threads from it.
    pub fn new<F>(handler: F) -> Arc<Self>
    where
        F: Fn(WorkItem) + Send + 'static,
    {
        let jobs: Arc<Mutex<HashMap<String, Job>>> = Arc::new(Mutex::new(HashMap::new()));
        let jobs_clone = jobs.clone();

        thread::Builder::new()
            .name("lattice-cron".to_string())
            .spawn(move || {
                let mut last_minute: Option<u32> = None;
                loop {
                    thread::sleep(Duration::from_secs(1));
                    let now = chrono::Utc::now();
                    use chrono::Timelike;
                    let minute = now.minute();
                    if Some(minute) == last_minute {
                        continue;
                    }
                    last_minute = Some(minute);
                    let map = jobs_clone.lock().unwrap_or_else(|e| e.into_inner());
                    for job in map.values() {
                        if job.spec.matches_utc(&now) {
                            handler(job.task.clone());
                        }
                    }
                }
            })
            .expect("failed to spawn cron thread");

        Arc::new(Self {
            jobs,
            counter: Mutex::new(0),
        })
    }

    fn next_id(&self) -> String {
        let mut c = self.counter.lock().unwrap_or_else(|e| e.into_inner());
        *c += 1;
        format!("cron_{}", *c)
    }
}

impl Scheduler for MemoryCronScheduler {
    fn schedule(&self, cron: &str, task: WorkItem) -> Result<String, Error> {
        let spec = CronSpec::parse(cron)?;
        let id = self.next_id();
        self.jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.clone(), Job { spec, task });
        Ok(id)
    }

    fn cancel(&self, job_id: &str) -> Result<(), Error> {
        self.jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(job_id)
            .map(|_| ())
            .ok_or_else(|| Error::not_found("cron job", job_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_wildcard() {
        let s = CronSpec::parse("* * * * *").unwrap();
        assert_eq!(s.minute, Field::Any);
        assert_eq!(s.hour, Field::Any);
    }

    #[test]
    fn parse_specific_values() {
        let s = CronSpec::parse("0,30 8-17 * * 1-5").unwrap();
        assert!(matches!(&s.minute, Field::Values(v) if v.contains(&0) && v.contains(&30)));
        assert!(matches!(&s.hour, Field::Values(v) if (8..=17).all(|h| v.contains(&h))));
        assert!(matches!(&s.weekday, Field::Values(v) if (1..=5).all(|d| v.contains(&d))));
    }

    #[test]
    fn parse_step() {
        let s = CronSpec::parse("*/15 * * * *").unwrap();
        if let Field::Values(v) = s.minute {
            assert!(v.contains(&0));
            assert!(v.contains(&15));
            assert!(v.contains(&30));
            assert!(v.contains(&45));
            assert!(!v.contains(&1));
        } else {
            panic!("expected Values");
        }
    }

    #[test]
    fn matches_utc_fires_on_correct_time() {
        use chrono::TimeZone;
        let s = CronSpec::parse("0 12 * * *").unwrap(); // noon daily
        let noon = chrono::Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        let not_noon = chrono::Utc.with_ymd_and_hms(2024, 6, 1, 12, 1, 0).unwrap();
        assert!(s.matches_utc(&noon));
        assert!(!s.matches_utc(&not_noon));
    }

    #[test]
    fn wrong_field_count_errors() {
        assert!(CronSpec::parse("* * * *").is_err());
        assert!(CronSpec::parse("* * * * * *").is_err());
    }
}
