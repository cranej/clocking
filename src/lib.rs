use chrono::prelude::*;
use std::fmt;

pub mod sqlite_store;
pub mod views;

#[derive(PartialEq, Clone, Debug)]
pub struct ClockingItemId {
    title: String,
    start: DateTime<Utc>,
}

#[derive(PartialEq, Clone, Debug)]
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

type Result<T> = std::result::Result<T, String>;

pub trait ClockingStore {
    fn start_clocking(&mut self, title: &str) -> Result<ClockingItemId>;
    fn start_clocking_item(&mut self, item: &ClockingItem) -> bool;
    fn finish_clocking(&mut self, id: &ClockingItemId, notes: &str) -> bool;
    fn finish_latest_unfinished_by_title(&mut self, title: &str, notes: &str) -> Result<bool>;
    fn finish_clocking_item(
        &mut self,
        id: &ClockingItemId,
        end: &DateTime<Utc>,
        notes: &str,
    ) -> bool;
    fn query_clocking(
        &self,
        start: &DateTime<Utc>,
        end: Option<DateTime<Utc>>,
    ) -> Vec<ClockingItem>;
    fn latest(&self, title: &str) -> Option<ClockingItem>;
}
