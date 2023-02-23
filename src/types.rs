use crate::strify_duration;
use chrono::prelude::*;
#[cfg(feature = "http")]
use pulldown_cmark::{html, Parser};
use serde::Serialize;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;

/// Identify an unique clocking entity
#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct EntryId<'a> {
    pub title: Cow<'a, str>,
    pub start: DateTime<Utc>,
}

/// Represent an unfinished clocking entity.
#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct UnfinishedEntry<'a> {
    pub id: EntryId<'a>,
    pub notes: Cow<'a, str>,
}

const TIME_FORMAT: &str = "%Y-%m-%d %a %H:%M";
impl<'a> UnfinishedEntry<'a> {
    pub fn started_minutes(&self) -> i64 {
        (Utc::now() - self.id.start).num_minutes()
    }
}

impl<'a> fmt::Display for UnfinishedEntry<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r = writeln!(f, "{}:", &self.id.title).and(writeln!(
            f,
            "\tStarted at: {}",
            self.id.start.with_timezone(&Local).format(TIME_FORMAT)
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

/// Represent a finished clocking entry.
#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct FinishedEntry<'a> {
    pub id: EntryId<'a>,
    pub end: DateTime<Utc>,
    pub notes: Cow<'a, str>,
}

#[cfg(feature = "http")]
impl<'a> FinishedEntry<'a> {
    pub fn html_segment(&self) -> String {
        let text = format!(
            "## {}\n **{}** ~ **{}** \n\n {}",
            &self.id.title,
            self.id.start.with_timezone(&Local).format(TIME_FORMAT),
            self.end.with_timezone(&Local).format(TIME_FORMAT),
            &self.notes
        );

        let parser = Parser::new(&text);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        html_output
    }
}

impl<'a> fmt::Display for FinishedEntry<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r = writeln!(f, "{}:", &self.id.title).and(writeln!(
            f,
            "\t{} ~ {}",
            self.id.start.with_timezone(&Local).format(TIME_FORMAT),
            self.end.with_timezone(&Local).format(TIME_FORMAT),
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

/// Represent the time span of a finished clocking entry.
#[derive(Serialize, Debug, Eq, PartialEq, Clone)]
pub(crate) struct TimeSpan {
    start: DateTime<Local>,
    end: DateTime<Local>,
}

impl TimeSpan {
    pub fn build(start: DateTime<Local>, end: DateTime<Local>) -> Result<Self, &'static str> {
        if end > start {
            Ok(TimeSpan { start, end })
        } else {
            Err("Invalid TimeSpan: end must after start.")
        }
    }

    pub fn duration(&self) -> chrono::Duration {
        self.end - self.start
    }

    pub fn start(&self) -> DateTime<Local> {
        self.start
    }

    pub fn end(&self) -> DateTime<Local> {
        self.end
    }
}

impl Ord for TimeSpan {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}

impl PartialOrd for TimeSpan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.start.cmp(&other.start))
    }
}

const LOCAL_FORMAT: &str = "%Y-%m-%d %a %H:%M";
const LOCAL_NO_DATE_FORMAT: &str = "%H:%M";
impl fmt::Display for TimeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_format = if f.alternate() {
            LOCAL_NO_DATE_FORMAT
        } else {
            LOCAL_FORMAT
        };

        let dur_string = strify_duration(&self.duration());
        write!(
            f,
            "{} ~ {}, {}",
            self.start.format(time_format),
            self.end.format(time_format),
            dur_string
        )
    }
}
