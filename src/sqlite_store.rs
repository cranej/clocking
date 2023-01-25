use crate::{ClockingItem, ClockingItemId, ClockingStore, Result};
use chrono::prelude::*;
use rusqlite::Connection;

pub struct SqliteStore {
    conn: Connection,
}

pub const IN_MEMORY: &str = ":memory:";
impl SqliteStore {
    pub fn new(p: &str) -> Self {
        let conn = if p == IN_MEMORY {
            Connection::open_in_memory().expect("Should be able to open in memory sqlite.")
        } else {
            Connection::open(p).expect("Falied to open sqlite at specified location.")
        };

        conn.execute(
            "CREATE TABLE IF NOT EXISTS clocking (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                start TEXT NOT NULL,
                end TEXT NULL,
                notes TEXT NULL
             )",
            (),
        )
        .expect("Initialize table failed.");

        SqliteStore { conn }
    }

    fn row_to_item(row: &rusqlite::Row<'_>) -> ClockingItem {
        let start_string: String = row.get("start").unwrap();
        let end_string: Option<String> = row.get("end").unwrap();
        ClockingItem {
            id: ClockingItemId {
                title: row.get("title").unwrap(),
                start: DateTime::parse_from_rfc3339(&start_string)
                    .unwrap()
                    .with_timezone(&Utc),
            },
            end: end_string.map(|end_str| {
                DateTime::parse_from_rfc3339(&end_str)
                    .unwrap()
                    .with_timezone(&Utc)
            }),
            notes: row.get("notes").unwrap(),
        }
    }
}

impl ClockingStore for SqliteStore {
    fn start_clocking(&mut self, title: &str) -> Result<ClockingItemId> {
        let item = ClockingItem {
            id: ClockingItemId {
                title: title.to_string(),
                start: Utc::now(),
            },
            end: None,
            notes: String::new(),
        };

        if self.start_clocking_item(&item) {
            Ok(item.id.clone())
        } else {
            Err("Falied to start clocking.".to_string())
        }
    }

    fn start_clocking_item(&mut self, item: &ClockingItem) -> bool {
        let start_time_string = item.id.start.to_rfc3339();
        match self.conn.query_row(
            "SELECT id FROM clocking WHERE title = ? and start = ?",
            [&item.id.title, &start_time_string],
            |_row| Ok(()),
        ) {
            Ok(()) => {
                println!("Existed...");
                false
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                match self.conn.execute(
                    "INSERT INTO clocking (title, start, notes) VALUES(?, ?, ?)",
                    [&item.id.title, &start_time_string, &item.notes],
                ) {
                    Ok(1) => true,
                    Ok(inserted) => {
                        println!("abnormal inserted count: {}", inserted);
                        false
                    }
                    Err(err) => {
                        println!("Insert failed: {}", err);
                        false
                    }
                }
            }
            Err(other_err) => {
                println!("Error when query existing item: {}", other_err);
                false
            }
        }
    }
    fn finish_clocking(&mut self, id: &ClockingItemId, notes: &str) -> bool {
        let end = Utc::now();
        self.finish_clocking_item(id, &end, notes)
    }

    fn finish_latest_unfinished_by_title(&mut self, title: &str, notes: &str) -> Result<bool> {
        let end_string = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE clocking set end = ?, notes = IFNULL(notes, '')||? where id in (
                    SELECT max(id) FROM clocking WHERE end is NULL and title = ?
            )",
                [&end_string, notes, title],
            )
            .map_err(|e| e.to_string())
            .and_then(|n| match n {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(format!("{n} rows updated! should be 1...")),
            })
    }

    fn finish_clocking_item(
        &mut self,
        id: &ClockingItemId,
        end: &DateTime<Utc>,
        notes: &str,
    ) -> bool {
        let start_string = id.start.to_rfc3339();
        let end_string = end.to_rfc3339();
        match self.conn.execute("UPDATE clocking SET end = ?, notes = IFNULL(notes, '')||?  WHERE title = ? and start = ? and end IS NULL",
                           [&end_string, notes, &id.title, &start_string]) {
            Ok(1) => true,
            Ok(updated) => { println!("abnormal updated count: {}", updated); false },
            Err(err) => { println!("Update failed: {}", err); false },
        }
    }

    fn query_clocking(
        &self,
        start: &DateTime<Utc>,
        end: Option<DateTime<Utc>>,
    ) -> Vec<ClockingItem> {
        let start_string = start.to_rfc3339();
        let end_string = end.map_or_else(|| Utc::now().to_rfc3339(), |x| x.to_rfc3339());
        let mut stmt = self.conn.prepare(
            "SELECT title, start, end, notes from clocking where start >= ? and end is not null and end <= ? order by start ").expect("Should be able to prepare statement.");
        stmt.query_map([&start_string, &end_string], |row| {
            Ok(SqliteStore::row_to_item(row))
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect()
    }

    fn latest(&self, title: &str) -> Option<ClockingItem> {
        self.conn.query_row(
            "SELECT title, start, end, notes from clocking where title = ? and end is not null order by start desc limit 1",
            [title],
            |row| Ok(SqliteStore::row_to_item(row)))
            .ok()
    }

    fn recent_titles(&self, count: usize) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT title, max(start) FROM clocking group by title order by max(start) desc limit ?")
            .expect("Should be able to prepare statment.");
        stmt.query_map([count], |row| Ok(row.get("title").unwrap()))
            .unwrap()
            .map(|x| x.unwrap())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_store_basic_workflow() {
        let mut mem_store = SqliteStore::new(IN_MEMORY);
        let start_time = Utc::now();
        let item = ClockingItem {
            id: ClockingItemId {
                title: "The Program".to_string(),
                start: start_time.clone(),
            },
            end: None,
            notes: String::new(),
        };

        assert_eq!(mem_store.start_clocking_item(&item), true);
        // add again
        assert_eq!(
            mem_store.start_clocking_item(&item),
            false,
            "Adding the same item twice should fail."
        );

        let in_store_items = mem_store.query_clocking(&start_time, None);
        assert_eq!(
            in_store_items.len(),
            0,
            "Unfinished items should not included in query."
        );

        let end = Utc::now();
        let note = "A note";
        assert_eq!(mem_store.finish_clocking_item(&item.id, &end, &note), true);
        //finish again
        assert_eq!(
            mem_store.finish_clocking_item(&item.id, &end, &note),
            false,
            "call finish_clocking_item on finished item should fail"
        );

        let in_store_items = mem_store.query_clocking(&start_time, None);
        assert_eq!(in_store_items.len(), 1);

        let expected_item = ClockingItem {
            end: Some(end.clone()),
            notes: String::from(note),
            ..item
        };

        assert_eq!(&in_store_items[0], &expected_item);
    }

    #[test]
    fn sqlite_store_query() {
        let mut mem_store = SqliteStore::new(IN_MEMORY);
        // item0..5 starts from @now - 4d5min, @now -3d5min, ... @now - 5min
        let items = gen_items(5);
        for item in items.iter() {
            assert!(mem_store.start_clocking_item(item));
        }

        // ends item1, item3, and item4
        // after the finish_clocking_item calls, data should be like:
        //  0. @now - 4d5min, None
        //  1. @now - 3d5min, @today -3 + 5minutes
        //  2. @now - 2d5min, None
        //  3. @now - 1d5min, @today - 1 + 5minutes
        //  4. @now - 5minutes, @now
        let indices = [1, 3, 4];
        let end_data = gen_end_data(&items, &indices);
        let add_note = "New note";
        for (end_item, end_time) in end_data.iter() {
            assert!(mem_store.finish_clocking_item(&end_item.id, &end_time, add_note));
        }

        // should return item 1, 3, 4
        let query_result = mem_store.query_clocking(&items[0].id.start, None);
        assert_eq!(query_result.len(), 3);
        assert_eq!(query_result[0], gen_expected_item(&items[1], add_note));
        assert_eq!(query_result[1], gen_expected_item(&items[3], add_note));
        assert_eq!(query_result[2], gen_expected_item(&items[4], add_note));

        // should return item 3, 4
        let query_result = mem_store.query_clocking(&items[2].id.start, None);
        assert_eq!(query_result.len(), 2);
        assert_eq!(query_result[0], gen_expected_item(&items[3], add_note));
        assert_eq!(query_result[1], gen_expected_item(&items[4], add_note));

        // should return item 3
        let query_result = mem_store.query_clocking(&items[2].id.start, Some(items[4].id.start));
        assert_eq!(query_result.len(), 1);
        assert_eq!(query_result[0], gen_expected_item(&items[3], add_note));
    }

    #[test]
    fn test_finised_unfinished_by_title() {
        let mut mem_store = SqliteStore::new(IN_MEMORY);
        let mut items = gen_items(6);
        assert_eq!(items.len(), 6);
        items[2].id.title = "Item 0".to_string();
        items[3].id.title = "Item 1".to_string();
        items[4].id.title = "Item 0".to_string();
        items[5].id.title = "Item 1".to_string();
        // now there are there Item 0 and there Item 1 in collection
        for item in items.iter() {
            assert!(mem_store.start_clocking_item(item));
        }

        //finish the fouth 1 , which is a "Item 1"
        assert_eq!(&items[3].id.title, "Item 1");
        assert!(mem_store.finish_clocking(&items[3].id, ""));

        let r = mem_store.finish_latest_unfinished_by_title("Item 1", "");
        assert_eq!(Ok(true), r);

        // finish all 'Item 0'
        let indices: &[usize] = &[0, 2, 4];
        for i in indices.iter() {
            assert_eq!(&items[*i].id.title, "Item 0");
        }
        let end_data = gen_end_data(&items, &indices);
        for (end_item, end_time) in end_data.iter() {
            assert!(mem_store.finish_clocking_item(&end_item.id, &end_time, ""));
        }

        // try finish Item 0 by title
        let r = mem_store.finish_latest_unfinished_by_title("Item 0", "");
        assert_eq!(Ok(false), r);

        // try finish non-exist title
        assert_eq!(
            Ok(false),
            mem_store.finish_latest_unfinished_by_title("non-exists", "")
        );
    }

    fn gen_items(count: usize) -> Vec<ClockingItem> {
        let five_mins = chrono::Duration::minutes(5);
        (0..count)
            .map(|i| {
                let start_offset = chrono::Duration::days((count - i - 1) as i64) + five_mins;
                ClockingItem {
                    id: ClockingItemId {
                        title: format!("Item {i}"),
                        start: Utc::now().checked_sub_signed(start_offset).unwrap(),
                    },
                    end: None,
                    notes: format!("Init notes for item {i}\n"),
                }
            })
            .collect()
    }

    fn gen_end_data<'a>(
        source: &'a [ClockingItem],
        indices: &[usize],
    ) -> Vec<(&'a ClockingItem, DateTime<Utc>)> {
        let five_mins = chrono::Duration::minutes(5);
        indices
            .iter()
            .map(|i| {
                let item = &source[*i];
                let end = item.id.start.checked_add_signed(five_mins).unwrap();
                (item, end)
            })
            .collect()
    }

    fn gen_expected_item(item: &ClockingItem, new_note: &str) -> ClockingItem {
        let mut final_notes = item.notes.clone();
        final_notes.push_str(new_note);
        ClockingItem {
            id: item.id.clone(),
            end: Some(
                item.id
                    .start
                    .checked_add_signed(chrono::Duration::minutes(5))
                    .unwrap(),
            ),
            notes: final_notes,
        }
    }
}
