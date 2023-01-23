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

impl fmt::Display for ClockingItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r = writeln!(f, "{}:", &self.id.title).and(writeln!(
            f,
            "\tStart: {}",
            self.id.start.to_rfc3339()
        ));

        if !self.notes.is_empty() {
            r = r.and(writeln!(f, "\tNotes:"));
            for line in self.notes.lines() {
                r = r.and(writeln!(f, "\t\t{line}"));
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
}
