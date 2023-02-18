use crate::strify_duration;
use crate::types::*;
use chrono::prelude::*;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeMap as Map;
use std::fmt;

type TitleDurationMap = Map<String, chrono::Duration>;
type DateDurationMap = Map<NaiveDate, chrono::Duration>;

/// `EntryDetailView` groups detailed `Effort` (start, end) by `FinishedEntry` title.
#[derive(Serialize, Debug)]
pub struct EntryDetailView(Map<String, Vec<TimeSpan>>);

impl fmt::Display for EntryDetailView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r: fmt::Result = Ok(());
        for (title, efforts) in self.0.iter() {
            r = r.and_then(|_| writeln!(f, "{}:", title));
            let mut total_duration: chrono::Duration = chrono::Duration::days(0);
            for eff in efforts.iter() {
                r = r.and_then(|_| writeln!(f, "\t{}", eff));
                total_duration = total_duration + eff.duration();
            }
            r = r.and_then(|_| writeln!(f, "\t(Total): {}\n", strify_duration(&total_duration)))
        }
        r
    }
}

impl EntryDetailView {
    pub fn new(entries: &[FinishedEntry]) -> Self {
        let mut view: Map<String, Vec<TimeSpan>> = Map::new();
        for entry in entries.iter() {
            view.entry(entry.id.title.to_string())
                .and_modify(|efforts| {
                    efforts.push(
                        // TODO: handle invalid timespan (bad data in database)
                        TimeSpan::build(
                            entry.id.start.with_timezone(&Local),
                            entry.end.with_timezone(&Local),
                        )
                        .unwrap(),
                    );
                })
                .or_insert_with(|| {
                    // TODO: handle invalid timespan (bad data in database)
                    vec![TimeSpan::build(
                        entry.id.start.with_timezone(&Local),
                        entry.end.with_timezone(&Local),
                    )
                    .unwrap()]
                });
        }

        EntryDetailView(view)
    }
}

/// `DailySummaryView` groups summarized [`chrono::Duration`] by local naive date of [`FinishedEntry`] start.
#[derive(Debug)]
pub struct DailySummaryView(DateDurationMap);

impl DailySummaryView {
    pub fn new(entries: &[FinishedEntry]) -> Self {
        let mut view: DateDurationMap = Map::new();
        for entry in entries.iter() {
            let duration = entry.end - entry.id.start;
            let start = entry.id.start.with_timezone(&Local).date_naive();
            view.entry(start)
                .and_modify(|dur| *dur = *dur + duration)
                .or_insert(duration);
        }

        DailySummaryView(view)
    }
}

impl fmt::Display for DailySummaryView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r: fmt::Result = Ok(());
        let mut daily_total = chrono::Duration::days(0);
        for (date, duration) in self.0.iter() {
            r = r.and_then(|_| writeln!(f, "{}: {}", date, strify_duration(duration)));
            daily_total = daily_total + *duration;
        }

        if self.0.len() > 1 {
            r = r.and_then(|_| writeln!(f, "(Total): {}", strify_duration(&daily_total)));
        }
        r
    }
}

/// `DailyDetailView` groups `(FinishedEntry::id::title, chrono::Duration)` by local naive date of `FinishedEntry` start.
#[derive(Debug)]
pub struct DailyDetailView(Map<NaiveDate, TitleDurationMap>);
impl DailyDetailView {
    pub fn new(entries: &[FinishedEntry]) -> Self {
        let mut view: Map<NaiveDate, TitleDurationMap> = Map::new();
        for entry in entries.iter() {
            let duration = entry.end - entry.id.start;
            let start = entry.id.start.with_timezone(&Local).date_naive();
            view.entry(start)
                .and_modify(|inner_map| {
                    inner_map
                        .entry(entry.id.title.to_string())
                        .and_modify(|dur| *dur = *dur + duration)
                        .or_insert(duration);
                })
                .or_insert_with(|| {
                    let mut inner_map: TitleDurationMap = TitleDurationMap::new();
                    inner_map.insert(entry.id.title.to_string(), duration);
                    inner_map
                });
        }

        DailyDetailView(view)
    }
}

impl fmt::Display for DailyDetailView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r: fmt::Result = Ok(());
        let mut total_duration = chrono::Duration::days(0);
        for (date, detail) in self.0.iter() {
            r = r.and_then(|_| writeln!(f, "{date}: "));

            let mut daily_total = chrono::Duration::days(0);
            for (title, duration) in detail.iter() {
                r = r.and_then(|_| writeln!(f, "\t{title}: {}", strify_duration(duration)));
                daily_total = daily_total + *duration;
            }

            r = r.and_then(|_| writeln!(f, "\t(Total): {}\n", strify_duration(&daily_total)));
            total_duration = total_duration + daily_total;
        }
        if self.0.len() > 1 {
            r = r.and_then(|_| writeln!(f, "(Total): {}", strify_duration(&total_duration)));
        }
        r
    }
}

#[derive(Serialize, Debug, Eq, PartialEq, Clone)]
struct TimeSpanWithTitle(TimeSpan, String);
impl Ord for TimeSpanWithTitle {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for TimeSpanWithTitle {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

/// `DailyDistributionView` groups sorted `Vec<Effort>` by local naive date of `FinishedEntry` start.
#[derive(Debug)]
pub struct DailyDistributionView(Map<NaiveDate, Vec<TimeSpanWithTitle>>);
impl DailyDistributionView {
    pub fn new(entries: &[FinishedEntry]) -> Self {
        let mut view: Map<NaiveDate, Vec<TimeSpanWithTitle>> = Map::new();
        for entry in entries.iter() {
            let start_date = entry.id.start.with_timezone(&Local).date_naive();
            view.entry(start_date)
                .and_modify(|efforts| {
                    efforts.push(TimeSpanWithTitle(
                        // TODO: handle invalid timespan (bad data in database)
                        TimeSpan::build(
                            entry.id.start.with_timezone(&Local),
                            entry.end.with_timezone(&Local),
                        )
                        .unwrap(),
                        entry.id.title.to_string(),
                    ));
                })
                .or_insert_with(|| {
                    // TODO: handle invalid timespan (bad data in database)
                    vec![TimeSpanWithTitle(
                        TimeSpan::build(
                            entry.id.start.with_timezone(&Local),
                            entry.end.with_timezone(&Local),
                        )
                        .unwrap(),
                        entry.id.title.to_string(),
                    )]
                });
        }

        let today_naive = Local::now().date_naive();
        let local_fixed_offset = Local.offset_from_local_date(&today_naive).unwrap();

        let day_start_time = chrono::naive::NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        let day_end_time = chrono::naive::NaiveTime::from_hms_opt(21, 0, 0).unwrap();

        let idle_title = "<idle>".to_string();
        let view = view
            .iter_mut()
            .map(|(date, efforts)| {
                efforts.sort();
                let mut current_dt = date.and_time(day_start_time);
                let mut with_idles_sorted: Vec<TimeSpanWithTitle> = vec![];
                for eff in efforts.iter() {
                    if current_dt < eff.0.start().naive_local() {
                        // TODO: handle invalid timespan (bad data in database)
                        with_idles_sorted.push(TimeSpanWithTitle(
                            TimeSpan::build(
                                DateTime::from_local(current_dt, local_fixed_offset),
                                eff.0.start(),
                            )
                            .unwrap(),
                            idle_title.clone(),
                        ));
                        current_dt = eff.0.end().naive_local();
                    }

                    with_idles_sorted.push(eff.clone());
                }

                let day_end_dt = date.and_time(day_end_time);
                if current_dt < day_end_dt {
                    // TODO: handle invalid timespan (bad data in database)
                    with_idles_sorted.push(TimeSpanWithTitle(
                        TimeSpan::build(
                            DateTime::from_local(current_dt, local_fixed_offset),
                            DateTime::from_local(day_end_dt, local_fixed_offset),
                        )
                        .unwrap(),
                        idle_title.clone(),
                    ));
                }

                (*date, with_idles_sorted)
            })
            .collect();

        DailyDistributionView(view)
    }
}

impl fmt::Display for DailyDistributionView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r: fmt::Result = Ok(());
        for (date, efforts) in self.0.iter() {
            r = r.and_then(|_| writeln!(f, "{date}: "));
            let filter_func: &dyn Fn(&&TimeSpanWithTitle) -> bool = if f.alternate() {
                &|_eff| true
            } else {
                &|eff| eff.0.duration().num_minutes() > 0
            };

            for eff in efforts.iter().filter(filter_func) {
                r = r.and_then(|_| writeln!(f, "\t{:#}: {}", eff.0, eff.1));
            }
        }
        r
    }
}
