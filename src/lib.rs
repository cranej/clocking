pub mod server;
pub mod sqlite_store;
pub mod types;
pub mod views;

use chrono::prelude::*;
use types::*;

type Result<T> = std::result::Result<T, String>;

const NAIVE_DATE_FORMAT: &str = "%Y-%m-%d";
pub trait ClockingStore {
    /// Start a clocking entry at now.
    fn start(&mut self, title: &str) -> Result<EntryId> {
        let entry = UnfinishedEntry {
            id: EntryId {
                title: title.to_string(),
                start: Utc::now(),
            },
            notes: String::new(),
        };

        if self.try_start_entry(&entry) {
            Ok(entry.id.clone())
        } else {
            Err("Falied to start clocking.".to_string())
        }
    }

    /// Start a clocking entry.
    ///
    /// Return false if entry already started.
    fn try_start_entry(&mut self, entry: &UnfinishedEntry) -> bool;

    /// Try to finish the latest-started unfinished entry of given title.
    ///
    /// Returns Ok(false) if no such unfinished entry found.
    fn try_finish_title(&mut self, title: &str, notes: &str) -> Result<bool>;

    /// Try to finish an unfinished clocking entry, set end datetime to now.
    ///
    /// Returns false if give entry is already finished or not found.
    fn try_finish_entry_now(&mut self, id: &EntryId, notes: &str) -> bool {
        let end = Utc::now();
        self.try_finish_entry(id, &end, notes)
    }

    /// Try to finish an unfinished clocking entry, set end datetime to `end`.
    ///
    /// Returns false if give entry is already finished or not found.
    fn try_finish_entry(&mut self, id: &EntryId, end: &DateTime<Utc>, notes: &str) -> bool;

    /// Query finished clocking entries with start in `[query_start, query_end]`.
    ///
    /// `query_end` default to now if None specified.
    fn finished(
        &self,
        query_start: &DateTime<Utc>,
        query_end: Option<DateTime<Utc>>,
    ) -> Vec<FinishedEntry>;

    /// Query finished clocking entries from date range:
    ///   start: (@today - `days_offset`) 0:00:00
    ///   to: (@today - `days_offset` + days) 0:00:00 if days is not None, otherwise to now()
    fn finished_by_offset(&self, days_offset: u64, days: Option<u64>) -> Vec<FinishedEntry> {
        let (start, end) = store_helper::query_start_end(days_offset, days);
        self.finished(&start, end)
    }

    /// Query finished clocking entries, accepts 'yyyy-mm-dd' local dates as query range.
    ///
    /// Note: `day_end` is included in the query range.
    fn finished_by_date_str(
        &self,
        day_start: &str,
        day_end: &str,
    ) -> std::result::Result<Vec<FinishedEntry>, &'static str> {
        let start_date = NaiveDate::parse_from_str(day_start, NAIVE_DATE_FORMAT)
            .map_err(|_| "Invalid format of day_start")?;
        let end_date = NaiveDate::parse_from_str(day_end, NAIVE_DATE_FORMAT)
            .map_err(|_| "Invalid format of day_end")?;

        if end_date < start_date {
            Err("Invalid date range: day_end must not before day_start")
        } else {
            let today_naive = Local::now().date_naive();
            let local_fixed_offset = Local.offset_from_local_date(&today_naive).unwrap();
            let start_dt = DateTime::<FixedOffset>::from_local(
                start_date.and_hms_opt(0, 0, 0).unwrap(),
                local_fixed_offset,
            )
            .with_timezone(&Utc);
            let end_dt = DateTime::<FixedOffset>::from_local(
                end_date.and_hms_opt(23, 59, 59).unwrap(),
                local_fixed_offset,
            )
            .with_timezone(&Utc);

            Ok(self.finished(&start_dt, Some(end_dt)))
        }
    }

    /// Fetch latest-started finished clocking entries by title.
    fn latest_finished(&self, title: &str) -> Option<FinishedEntry>;

    /// Fetch at most `limit` latest-started finished clocking entries.
    fn recent_titles(&self, limit: usize) -> Vec<String>;

    /// Fetch at most `limit` latest-started unfinished clocking entries.
    fn unfinished(&self, limit: usize) -> Vec<UnfinishedEntry>;
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
