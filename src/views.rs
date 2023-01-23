use crate::ClockingItem;
use chrono::prelude::*;
use std::collections::HashMap;
use std::fmt;

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

#[derive(Debug)]
struct Effort {
    start: DateTime<Local>,
    end: DateTime<Local>,
}

const LOCAL_FORMAT: &str = "%Y-%m-%d %a %H:%M";
impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dur_string = strify_duration(&self.span());
        write!(
            f,
            "{} ~ {}, {}",
            self.start.format(LOCAL_FORMAT),
            self.end.format(LOCAL_FORMAT),
            dur_string
        )
    }
}

impl Effort {
    fn span(&self) -> chrono::Duration {
        self.end - self.start
    }
}

#[derive(Debug)]
pub struct ItemAgg {
    title: String,
    efforts: Vec<Effort>,
}

impl fmt::Display for ItemAgg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r = writeln!(f, "{}:", &self.title);
        for eff in self.efforts.iter() {
            r = r.and_then(|_| writeln!(f, "\t{}", eff));
        }
        r.and_then(|_| writeln!(f, "\tTotal: {}", strify_duration(&self.total_span())))
    }
}

impl ItemAgg {
    fn total_span(&self) -> chrono::Duration {
        self.efforts
            .iter()
            .map(|e| e.span())
            .reduce(|acc, e| acc + e)
            .unwrap()
    }
}

#[derive(Debug)]
pub struct ItemView {
    agg_list: Vec<ItemAgg>,
}

impl fmt::Display for ItemView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut r: fmt::Result = Ok(());
        for agg in self.agg_list.iter() {
            r = r.and_then(|_| writeln!(f, "{}", agg));
        }
        r
    }
}

impl ItemView {
    pub fn new(items: &[ClockingItem]) -> Self {
        let mut view_map: HashMap<String, ItemAgg> = HashMap::new();
        for item in items.iter() {
            view_map
                .entry(item.id.title.clone())
                .and_modify(|agg| {
                    agg.efforts.push(Effort {
                        start: item.id.start.with_timezone(&Local),
                        end: item.end.unwrap().with_timezone(&Local),
                    });
                })
                .or_insert(ItemAgg {
                    title: item.id.title.clone(),
                    efforts: vec![Effort {
                        start: item.id.start.with_timezone(&Local),
                        end: item.end.unwrap().with_timezone(&Local),
                    }],
                });
        }

        ItemView {
            agg_list: view_map.into_values().collect(),
        }
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

        let item_view = ItemView::new(&items_2);
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
