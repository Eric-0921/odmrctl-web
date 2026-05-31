use std::time::{Instant, SystemTime, UNIX_EPOCH};

use odmr_logging::{EventLevel, RunEvent, RunEventType};

use std::sync::atomic::{AtomicU64, Ordering};

static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_event_id() -> u64 {
    EVENT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub fn utc_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn wall_time_utc_iso() -> String {
    let now = SystemTime::now();
    let dur = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let nanos = dur.subsec_nanos();
    format!("{}.{:09}Z", format_unix_secs_rfc3339(secs), nanos)
}

fn format_unix_secs_rfc3339(secs: u64) -> String {
    const DAYS_IN_MONTH: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut days = secs / 86400;
    let mut year = 1970u64;
    loop {
        let is_leap =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let year_days = if is_leap { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }
    let is_leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let mut month = 0usize;
    while month < 12 {
        let dim = if month == 1 && is_leap {
            29
        } else {
            DAYS_IN_MONTH[month]
        };
        if days < dim {
            break;
        }
        days -= dim;
        month += 1;
    }
    let day = days + 1;
    let rem = secs % 86400;
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year,
        month + 1,
        day,
        hour,
        minute,
        second
    )
}

pub fn make_event(
    run_id: &str,
    event_type: RunEventType,
    level: EventLevel,
    message: &str,
    device_id: &str,
    data: Option<serde_json::Value>,
    mono_ns: Option<u64>,
) -> RunEvent {
    RunEvent {
        schema_version: "0.2.0".into(),
        kind: "run_event".into(),
        run_id: run_id.into(),
        event_id: format!("evt_{:010}", next_event_id()),
        timestamp_unix_ms: utc_now_ms(),
        timestamp_monotonic_ns: mono_ns,
        level,
        event_type,
        step_id: None,
        device_id: Some(device_id.into()),
        message: message.into(),
        data,
    }
}

pub struct TimelineTracker {
    run_start_instant: Instant,
    pub events: Vec<crate::types::TimelineEvent>,
}

impl TimelineTracker {
    pub fn new() -> Self {
        Self {
            run_start_instant: Instant::now(),
            events: Vec::new(),
        }
    }

    pub fn record(
        &mut self,
        event_type: &str,
        device_id: &str,
        data: Option<serde_json::Value>,
    ) -> u64 {
        let now = Instant::now();
        let mono_ns = now.duration_since(self.run_start_instant).as_nanos() as u64;
        self.events.push(crate::types::TimelineEvent {
            event_type: event_type.into(),
            wall_time_utc: wall_time_utc_iso(),
            monotonic_ns: mono_ns,
            monotonic_ns_since_run_start: mono_ns,
            device_id: device_id.into(),
            data,
        });
        mono_ns
    }
}

impl Default for TimelineTracker {
    fn default() -> Self {
        Self::new()
    }
}
