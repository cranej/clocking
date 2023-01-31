use chrono::prelude::*;
use serde::Serialize;
use std::cmp::Ordering;
use std::fmt;

pub mod server;
pub mod sqlite_store;
pub mod views;
mod types;

#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct ClockingItemId {
    title: String,
    start: DateTime<Utc>,
}

#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct ClockingItem {
    id: ClockingItemId,
    end: Option<DateTime<Utc>>,
    notes: String,
}

const TIME_FORMAT: &str = "%Y-%m-%d %a %H:%M";
impl fmt::Display for ClockingItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r = writeln!(f, "{}:", &self.id.title).and(writeln!(
            f,
            "\t{} ~ {}",
            self.id.start.with_timezone(&Local).format(TIME_FORMAT),
            self.end
                .map(|e| e.with_timezone(&Local).format(TIME_FORMAT).to_string())
                .unwrap_or_else(|| "Unfinished".to_string()),
        ));

        if !self.notes.is_empty() {
            r = r.and(writeln!(f, "\tNotes:"));
            for line in self.notes.lines() {
                r = r.and_then(|_| writeln!(f, "\t  {line}"));
            }
        }

        r
    }
}

#[derive(Serialize, Debug, Eq, PartialEq, Clone)]
pub(crate) struct Effort {
    start: DateTime<Local>,
    end: DateTime<Local>,
}

impl Ord for Effort {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}

impl PartialOrd for Effort {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.start.cmp(&other.start))
    }
}

const LOCAL_FORMAT: &str = "%Y-%m-%d %a %H:%M";
const LOCAL_NO_DATE_FORMAT: &str = "%H:%M";
impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_format = if f.alternate() {
            LOCAL_NO_DATE_FORMAT
        } else {
            LOCAL_FORMAT
        };

        let dur_string = strify_duration(&self.span());
        write!(
            f,
            "{} ~ {}, {}",
            self.start.format(time_format),
            self.end.format(time_format),
            dur_string
        )
    }
}

impl Effort {
    fn span(&self) -> chrono::Duration {
        self.end - self.start
    }
}

type Result<T> = std::result::Result<T, String>;

pub trait ClockingStore {
    /// Start a clocking entry at now.
    fn start(&mut self, title: &str) -> Result<ClockingItemId>;

    /// Start a clocking entry.
    ///
    /// Return false if entry already started.
    fn start_item(&mut self, item: &ClockingItem) -> bool;

    /// Try to finish an unfinished clocking entry, set end datetime to now.
    ///
    /// Returns false if give entry is already finished.
    fn finish(&mut self, id: &ClockingItemId, notes: &str) -> bool;

    /// Try to finish the latest started unfinished entry of given title.
    ///
    /// Returns Ok(false) if no such unfinished entry found.
    fn finish_latest_unfinished_by_title(&mut self, title: &str, notes: &str) -> Result<bool>;

    /// Try to finish an unfinished clocking entry, set end datetime to `end`.
    ///
    /// Returns false if give entry is already finished.
    fn finish_item(&mut self, id: &ClockingItemId, end: &DateTime<Utc>, notes: &str) -> bool;

    /// Query finished clocking entries with start in `[query_start, query_end]`.
    fn query(&self, start: &DateTime<Utc>, end: Option<DateTime<Utc>>) -> Vec<ClockingItem>;

    /// Query finished clocking items from date range:
    ///   start: (@today - `days_offset`) 0:00:00
    ///   to: (@today - `days_offset` + days) 0:00:00 if days is not None, otherwise to now()
    fn query_offset(&self, days_offset: u64, days: Option<u64>) -> Vec<ClockingItem> {
        let (start, end) = store_helper::query_start_end(days_offset, days);
        self.query(&start, end)
    }

    /// Fetch latest started finished clocking entries by title.
    fn latest(&self, title: &str) -> Option<ClockingItem>;

    /// Fetch at most `limit` latest started finished clocking entries.
    fn recent_titles(&self, limit: usize) -> Vec<String>;

    /// Fetch at most `limit` latest started unfinished clocking entries.
    fn unfinished(&self, limit: usize) -> Vec<ClockingItemId>;
}

pub(crate) mod store_helper {
    use chrono::naive::Days as NaiveDays;
    use chrono::prelude::*;

    pub(crate) fn query_start_end(
        days_offset: u64,
        days: Option<u64>,
    ) -> (DateTime<Utc>, Option<DateTime<Utc>>) {
        let today_naive = Local::now().date_naive();
        let local_fixed_offset = Local.offset_from_local_date(&today_naive).unwrap();
        let today_naive = today_naive.and_hms_opt(0, 0, 0).unwrap();

        let start_naive = today_naive
            .checked_sub_days(NaiveDays::new(days_offset))
            .unwrap();
        let start_in_utc = DateTime::<FixedOffset>::from_local(start_naive, local_fixed_offset)
            .with_timezone(&Utc);
        let end_in_utc = days.map(|d| {
            let end_naive = start_naive.checked_add_days(NaiveDays::new(d)).unwrap();
            DateTime::<FixedOffset>::from_local(end_naive, local_fixed_offset).with_timezone(&Utc)
        });

        (start_in_utc, end_in_utc)
    }
}

const HOUR_MINUTES: i64 = 60;
const DAY_MINUTES: i64 = HOUR_MINUTES * 24;
pub(crate) fn strify_duration(d: &chrono::Duration) -> String {
    let total_minutes = d.num_minutes();
    if total_minutes < HOUR_MINUTES {
        format!("0:{:0>2}", total_minutes)
    } else if total_minutes < DAY_MINUTES {
        let hours = total_minutes / HOUR_MINUTES;
        let minutes = total_minutes % HOUR_MINUTES;
        format!("{}:{:0>2}", hours, minutes)
    } else {
        let days = total_minutes / DAY_MINUTES;
        let remains = total_minutes % DAY_MINUTES;
        let hours = remains / HOUR_MINUTES;
        let minutes = remains % HOUR_MINUTES;

        format!("{}:{:0>2}:{:0>2}", days, hours, minutes)
    }
}
