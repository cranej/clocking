use crate::types::*;
use crate::{ClockingStore, Result};
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

    fn row_to_finished_entry(row: &rusqlite::Row<'_>) -> FinishedEntry {
        let start_string: String = row.get("start").unwrap();
        let end_string: String = row.get("end").unwrap();
        FinishedEntry {
            id: EntryId {
                title: row.get("title").unwrap(),
                start: DateTime::parse_from_rfc3339(&start_string)
                    .unwrap()
                    .with_timezone(&Utc),
            },
            end: DateTime::parse_from_rfc3339(&end_string)
                .unwrap()
                .with_timezone(&Utc),
            notes: row.get("notes").unwrap(),
        }
    }

    fn row_to_unfinished_entry(row: &rusqlite::Row<'_>) -> UnfinishedEntry {
        let start_string: String = row.get("start").unwrap();
        UnfinishedEntry {
            id: EntryId {
                title: row.get("title").unwrap(),
                start: DateTime::parse_from_rfc3339(&start_string)
                    .unwrap()
                    .with_timezone(&Utc),
            },
            notes: row.get("notes").unwrap(),
        }
    }
}

impl ClockingStore for SqliteStore {
    fn try_start_entry(&mut self, entry: &UnfinishedEntry) -> bool {
        let start_time_string = entry.id.start.to_rfc3339();
        match self.conn.query_row(
            "SELECT id FROM clocking WHERE title = ? and start = ?",
            [&entry.id.title, &start_time_string],
            |_row| Ok(()),
        ) {
            Ok(()) => {
                println!("Existed...");
                false
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                match self.conn.execute(
                    "INSERT INTO clocking (title, start, notes) VALUES(?, ?, ?)",
                    [&entry.id.title, &start_time_string, &entry.notes],
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

    fn try_finish_title(&mut self, title: &str, notes: &str) -> Result<bool> {
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

    fn try_finish_entry(&mut self, id: &EntryId, end: &DateTime<Utc>, notes: &str) -> bool {
        let start_string = id.start.to_rfc3339();
        let end_string = end.to_rfc3339();
        match self.conn.execute("UPDATE clocking SET end = ?, notes = IFNULL(notes, '')||?  WHERE title = ? and start = ? and end IS NULL and start < ?",
                           [&end_string, notes, &id.title, &start_string, &end_string]) {
            Ok(1) => true,
            Ok(updated) => { println!("abnormal updated count: {}", updated); false },
            Err(err) => { println!("Update failed: {}", err); false },
        }
    }

    fn finished(
        &self,
        query_start: &DateTime<Utc>,
        query_end: Option<DateTime<Utc>>,
    ) -> Vec<FinishedEntry> {
        let start_string = query_start.to_rfc3339();
        let end_string = query_end.map_or_else(|| Utc::now().to_rfc3339(), |x| x.to_rfc3339());
        let mut stmt = self.conn.prepare(
            "SELECT title, start, end, notes from clocking where start >= ? and end is not null and end <= ? order by start ").expect("Should be able to prepare statement.");
        stmt.query_map([&start_string, &end_string], |row| {
            Ok(SqliteStore::row_to_finished_entry(row))
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect()
    }

    fn latest_finished(&self, title: &str) -> Option<FinishedEntry> {
        self.conn.query_row(
            "SELECT title, start, end, notes from clocking where title = ? and end is not null order by start desc limit 1",
            [title],
            |row| Ok(SqliteStore::row_to_finished_entry(row)))
            .ok()
    }

    fn recent_titles(&self, limit: usize) -> Vec<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT title, max(start) FROM clocking where end is not null group by title order by max(start) desc limit ?")
            .expect("Should be able to prepare statment.");
        stmt.query_map([limit], |row| Ok(row.get("title").unwrap()))
            .unwrap()
            .map(|x| x.unwrap())
            .collect()
    }

    fn unfinished(&self, limit: usize) -> Vec<UnfinishedEntry> {
        let mut stmt = self
            .conn
            .prepare(
                "select title, start, notes from clocking where end is null order by start desc limit ?",
            )
            .expect("Should be able to prepare statment.");
        stmt.query_map([limit], |row| Ok(SqliteStore::row_to_unfinished_entry(row)))
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
        let entry = UnfinishedEntry {
            id: EntryId {
                title: "The Program".to_string(),
                start: start_time,
            },
            notes: String::new(),
        };

        assert_eq!(mem_store.try_start_entry(&entry), true);
        // add again
        assert_eq!(
            mem_store.try_start_entry(&entry),
            false,
            "Adding the same item twice should fail."
        );

        let finished_entries = mem_store.finished(&start_time, None);
        assert_eq!(
            finished_entries.len(),
            0,
            "Unfinished entries should not included in query."
        );

        let end = Utc::now();
        let note = "A note";
        assert_eq!(mem_store.try_finish_entry(&entry.id, &end, note), true);
        //finish again
        assert_eq!(
            mem_store.try_finish_entry(&entry.id, &end, note),
            false,
            "call try_finish_entry on finished entry should fail"
        );

        let finished_entries = mem_store.finished(&start_time, None);
        assert_eq!(finished_entries.len(), 1);

        let finished_entry = FinishedEntry {
            id: entry.id.clone(),
            end: end,
            notes: String::from(note),
        };

        assert_eq!(&finished_entries[0], &finished_entry);
    }

    #[test]
    fn sqlite_store_query() {
        let mut mem_store = SqliteStore::new(IN_MEMORY);
        // item0..5 starts from @now - 4d5min, @now -3d5min, ... @now - 5min
        let entries = gen_entries(5);
        for entry in entries.iter() {
            assert!(mem_store.try_start_entry(entry));
        }

        // ends entry1, entry3, and entry4
        // after the finish_item calls, data should be like:
        //  0. @now - 4d5min, None
        //  1. @now - 3d5min, @today -3 + 5minutes
        //  2. @now - 2d5min, None
        //  3. @now - 1d5min, @today - 1 + 5minutes
        //  4. @now - 5minutes, @now
        let indices = [1, 3, 4];
        let end_data = gen_end_data(&entries, &indices);
        let add_note = "New note";
        for (end_item, end_time) in end_data.iter() {
            assert!(mem_store.try_finish_entry(&end_item.id, end_time, add_note));
        }

        // should return item 1, 3, 4
        let query_result = mem_store.finished(&entries[0].id.start, None);
        assert_eq!(query_result.len(), 3);
        assert_eq!(query_result[0], gen_finished_entry(&entries[1], add_note));
        assert_eq!(query_result[1], gen_finished_entry(&entries[3], add_note));
        assert_eq!(query_result[2], gen_finished_entry(&entries[4], add_note));

        // should return item 3, 4
        let query_result = mem_store.finished(&entries[2].id.start, None);
        assert_eq!(query_result.len(), 2);
        assert_eq!(query_result[0], gen_finished_entry(&entries[3], add_note));
        assert_eq!(query_result[1], gen_finished_entry(&entries[4], add_note));

        // should return item 3
        let query_result = mem_store.finished(&entries[2].id.start, Some(entries[4].id.start));
        assert_eq!(query_result.len(), 1);
        assert_eq!(query_result[0], gen_finished_entry(&entries[3], add_note));
    }

    #[test]
    fn test_finised_unfinished_by_title() {
        let mut mem_store = SqliteStore::new(IN_MEMORY);
        let mut entries = gen_entries(6);
        assert_eq!(entries.len(), 6);
        entries[2].id.title = "Item 0".to_string();
        entries[3].id.title = "Item 1".to_string();
        entries[4].id.title = "Item 0".to_string();
        entries[5].id.title = "Item 1".to_string();
        // now there are there Item 0 and there Item 1 in collection
        for entry in entries.iter() {
            assert!(mem_store.try_start_entry(entry));
        }

        //finish the fouth 1 , which is a "Item 1"
        assert_eq!(&entries[3].id.title, "Item 1");
        assert!(mem_store.try_finish_entry_now(&entries[3].id, ""));

        let r = mem_store.try_finish_title("Item 1", "");
        assert_eq!(Ok(true), r);

        // finish all 'Item 0'
        let indices: &[usize] = &[0, 2, 4];
        for i in indices.iter() {
            assert_eq!(&entries[*i].id.title, "Item 0");
        }
        let end_data = gen_end_data(&entries, indices);
        for (end_item, end_time) in end_data.iter() {
            assert!(mem_store.try_finish_entry(&end_item.id, end_time, ""));
        }

        // try finish Item 0 by title
        let r = mem_store.try_finish_title("Item 0", "");
        assert_eq!(Ok(false), r);

        // try finish non-exist title
        assert_eq!(Ok(false), mem_store.try_finish_title("non-exists", ""));
    }

    fn gen_entries(count: usize) -> Vec<UnfinishedEntry> {
        let five_mins = chrono::Duration::minutes(5);
        (0..count)
            .map(|i| {
                let start_offset = chrono::Duration::days((count - i - 1) as i64) + five_mins;
                UnfinishedEntry {
                    id: EntryId {
                        title: format!("Item {i}"),
                        start: Utc::now().checked_sub_signed(start_offset).unwrap(),
                    },
                    notes: format!("Init notes for item {i}\n"),
                }
            })
            .collect()
    }

    fn gen_end_data<'a>(
        source: &'a [UnfinishedEntry],
        indices: &[usize],
    ) -> Vec<(&'a UnfinishedEntry, DateTime<Utc>)> {
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

    fn gen_finished_entry(item: &UnfinishedEntry, new_note: &str) -> FinishedEntry {
        let mut final_notes = item.notes.clone();
        final_notes.push_str(new_note);
        FinishedEntry {
            id: item.id.clone(),
            end: item
                .id
                .start
                .checked_add_signed(chrono::Duration::minutes(5))
                .unwrap(),
            notes: final_notes,
        }
    }
}
