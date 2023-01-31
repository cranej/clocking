use crate::ClockingItem;
use chrono::prelude::*;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{BTreeMap as Map, HashMap};
use std::fmt;

type TitleDurationMap = Map<String, chrono::Duration>;
type DateDurationMap = Map<NaiveDate, chrono::Duration>;

const HOUR_MINUTES: i64 = 60;
const DAY_MINUTES: i64 = HOUR_MINUTES * 24;
fn strify_duration(d: &chrono::Duration) -> String {
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

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct Effort {
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

/// [`Effort`] collection of give [`ItemEfforts::key`]
#[derive(Serialize, Debug)]
pub struct ItemEfforts {
    pub key: String,
    pub efforts: Vec<Effort>,
}

impl fmt::Display for ItemEfforts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r = writeln!(f, "{}:", &self.key);
        for eff in self.efforts.iter() {
            r = r.and_then(|_| writeln!(f, "\t{}", eff));
        }
        r.and_then(|_| writeln!(f, "\t(Total): {}", strify_duration(&self.total_span())))
    }
}

impl ItemEfforts {
    fn total_span(&self) -> chrono::Duration {
        self.efforts
            .iter()
            .map(|e| e.span())
            .reduce(|acc, e| acc + e)
            .unwrap()
    }
}

/// `ItemDetailView` groups detailed `Effort` (start, end) by `ClockingItem` title.
#[derive(Serialize, Debug)]
pub struct ItemDetailView(Vec<ItemEfforts>);

impl fmt::Display for ItemDetailView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r: fmt::Result = Ok(());
        for agg in self.0.iter() {
            r = r.and_then(|_| writeln!(f, "{}", agg));
        }
        r
    }
}

impl ItemDetailView {
    pub fn new(items: &[ClockingItem]) -> Self {
        let mut view_map: HashMap<String, ItemEfforts> = HashMap::new();
        for item in items.iter() {
            view_map
                .entry(item.id.title.clone())
                .and_modify(|agg| {
                    agg.efforts.push(Effort {
                        start: item.id.start.with_timezone(&Local),
                        end: item.end.unwrap().with_timezone(&Local),
                    });
                })
                .or_insert(ItemEfforts {
                    key: item.id.title.clone(),
                    efforts: vec![Effort {
                        start: item.id.start.with_timezone(&Local),
                        end: item.end.unwrap().with_timezone(&Local),
                    }],
                });
        }

        ItemDetailView(view_map.into_values().collect())
    }
}

/// `DailySummaryView` groups summarized [`chrono::Duration`] by local naive date of [`ClockingItem`] start.
#[derive(Debug)]
pub struct DailySummaryView(DateDurationMap);

impl DailySummaryView {
    pub fn new(items: &[ClockingItem]) -> Self {
        let mut view: DateDurationMap = Map::new();
        for item in items.iter() {
            let duration = item
                .end
                .expect("Item used in view should be in finished status.")
                - item.id.start;
            let start = item.id.start.with_timezone(&Local).date_naive();
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

/// `DailyDetailView` groups `(ClockingItem::id::title, chrono::Duration)` by local naive date of `ClockingItem` start.
#[derive(Debug)]
pub struct DailyDetailView(Map<NaiveDate, TitleDurationMap>);
impl DailyDetailView {
    pub fn new(items: &[ClockingItem]) -> Self {
        let mut view: Map<NaiveDate, TitleDurationMap> = Map::new();
        for item in items.iter() {
            let duration = item
                .end
                .expect("Item used in view should be in finished status.")
                - item.id.start;
            let start = item.id.start.with_timezone(&Local).date_naive();
            view.entry(start)
                .and_modify(|inner_map| {
                    inner_map
                        .entry(item.id.title.clone())
                        .and_modify(|dur| *dur = *dur + duration)
                        .or_insert(duration);
                })
                .or_insert_with(|| {
                    let mut inner_map: TitleDurationMap = TitleDurationMap::new();
                    inner_map.insert(item.id.title.clone(), duration);
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

/// `DailyDistributionView` groups `Vec<Effort>` by local naive date of `ClockingItem` start.
#[derive(Debug)]
pub struct DailyDistributionView(Map<NaiveDate, Vec<Effort>>);
impl DailyDistributionView {
    pub fn new(items: &[ClockingItem]) -> Self {
        let mut view: Map<NaiveDate, Vec<Effort>> = Map::new();
        for item in items.iter() {
            let start_date = item.id.start.with_timezone(&Local).date_naive();
            view.entry(start_date)
                .and_modify(|efforts| {
                    efforts.push(Effort {
                        start: item.id.start.with_timezone(&Local),
                        end: item.end.unwrap().with_timezone(&Local),
                    });
                })
                .or_insert(vec![Effort {
                    start: item.id.start.with_timezone(&Local),
                    end: item.end.unwrap().with_timezone(&Local),
                }]);
        }

        DailyDistributionView(view)
    }

    pub fn idle(&mut self) -> Self {
        let today_naive = Local::now().date_naive();
        let local_fixed_offset = Local.offset_from_local_date(&today_naive).unwrap();

        let day_start_time = chrono::naive::NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        let day_end_time = chrono::naive::NaiveTime::from_hms_opt(21, 0, 0).unwrap();

        let spare_view: Map<NaiveDate, Vec<Effort>> = self
            .0
            .iter_mut()
            .map(|(date, efforts)| {
                let mut current_dt = date.and_time(day_start_time);
                let mut spare_spans: Vec<Effort> = vec![];
                efforts.sort();
                for eff in efforts.iter() {
                    if current_dt < eff.start.naive_local() {
                        spare_spans.push(Effort {
                            start: DateTime::from_local(current_dt, local_fixed_offset),
                            end: eff.start,
                        });
                        current_dt = eff.end.naive_local();
                    }
                }

                let day_end_dt = date.and_time(day_end_time);
                if current_dt < day_end_dt {
                    spare_spans.push(Effort {
                        start: DateTime::from_local(current_dt, local_fixed_offset),
                        end: DateTime::from_local(day_end_dt, local_fixed_offset),
                    });
                }

                (*date, spare_spans)
            })
            .collect();

        DailyDistributionView(spare_view)
    }
}

impl fmt::Display for DailyDistributionView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r: fmt::Result = Ok(());
        for (date, efforts) in self.0.iter() {
            r = r.and_then(|_| writeln!(f, "{date}: "));
            if f.alternate() {
                for eff in efforts.iter() {
                    r = r.and_then(|_| writeln!(f, "\t{eff:#}"));
                }
            } else {
                for eff in efforts.iter().filter(|eff| eff.span().num_minutes() > 0) {
                    r = r.and_then(|_| writeln!(f, "\t{eff:#}"));
                }
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClockingItem, ClockingItemId};

    #[test]
    fn item_view_test() {
        let items = gen_finished_items(3);
        let mut items_2 = gen_finished_items(5);
        items_2.extend_from_slice(&items);

        let item_view = DetailView::new(&items_2);
        let s = item_view.to_string();
        println!("Data: {:#?}", &item_view);
        println!("{s}");
    }

    fn gen_finished_items(count: usize) -> Vec<ClockingItem> {
        let five_mins = chrono::Duration::minutes(5);
        (0..count)
            .map(|i| {
                let start_offset = chrono::Duration::days((count - i - 1) as i64) + five_mins;
                ClockingItem {
                    id: ClockingItemId {
                        title: format!("Item {i}"),
                        start: Utc::now().checked_sub_signed(start_offset).unwrap(),
                    },
                    end: Some(Utc::now()),
                    notes: format!("Init notes for item {i}\n"),
                }
            })
            .collect()
    }
}
