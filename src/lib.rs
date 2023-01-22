use chrono::prelude::*;

pub mod sqlite_store;

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

type Result<T> = std::result::Result<T, String>;

pub trait ClockingStore {
    fn start_clocking(&mut self, title: &str) -> Result<ClockingItemId>;
    fn start_clocking_item(&mut self, item: &ClockingItem) -> bool;
    fn finish_clocking(&mut self, id: &ClockingItemId, notes: &str) -> bool;
    fn finish_clocking_item(&mut self, id: &ClockingItemId, end: &DateTime<Utc>, notes: &str) -> bool;
    fn query_clocking(&self, start: &DateTime<Utc>, end: Option<DateTime<Utc>>) -> Vec<ClockingItem>;
}
