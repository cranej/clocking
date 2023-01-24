//use chrono::prelude::*;
use chrono::prelude::*;
use clap::{Parser, Subcommand};
use clocking::sqlite_store::SqliteStore;
use clocking::ClockingStore;
use std::env;
use std::io;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// File to store the data, take priority of environment variable 'CLOCKING_FILE'.
    #[arg(long)]
    file: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start {
        title: String,
        /// Do not wait for notes input, exit with unfinished status.
        #[arg(short, long)]
        no_wait: bool,
    },
    Finish {
        title: String,
        /// can be specified multiple times, each as a separate line. Sinel value '-' means read from stdin
        #[arg(short, long)]
        notes: Vec<String>,
    },
    Report {
        ///Tail offset. Default to 0 - today
        #[arg(short, long)]
        from: Option<u64>,
        ///Limit days from tail offset. Default to until now
        #[arg(short, long)]
        days: Option<u64>,
        ///Show daily summary
        #[arg(long = "daily")]
        daily_summary: bool,
        ///Show detail report
        #[arg(long)]
        detail: bool,
        ///<Unimplemented yet>.
        #[arg(long)]
        filter: Option<String>,
    },
    /// Details of latest record of item 'title'.
    Latest {
        /// Title of the item to display.
        title: String,
    },
}

const STORE_FILE_VAR: &str = "CLOCKING_FILE";
fn main() {
    let cli = Cli::parse();

    let store_file = cli
        .file
        .or_else(|| env::var(STORE_FILE_VAR).ok())
        .expect("Please specify storage file path either by environment or cli argument.");

    let mut store: Box<dyn ClockingStore> = Box::new(SqliteStore::new(&store_file));

    match cli.command {
        Commands::Start { title, no_wait } => {
            let id = store
                .start_clocking(&title)
                .expect("Failed to start clocking");

            if !no_wait {
                let notes = read_until();
                store.finish_clocking(&id, &notes);
            };
        }
        Commands::Finish { title, notes } => {
            let notes = if notes.len() == 1 && notes[0] == "-" {
                read_until()
            } else {
                notes.join("\n")
            };

            match store.finish_latest_unfinished_by_title(&title, &notes) {
                Ok(true) => println!("Updated."),
                Ok(false) => println!("No unfinished item found by {title}"),
                Err(e) => eprintln!("Unexpected error: {e}"),
            }
        }
        Commands::Report {
            from,
            days,
            daily_summary,
            detail,
            ..
        } => {
            let tail_offset = from.unwrap_or(0);
            let (start, end) = query_start_end(tail_offset, days);

            // dbg!(start, end);
            let items = store.query_clocking(&start, Some(end));
            let view = clocking::views::ItemView::new(&items);

            if daily_summary {
                println!("{}", &view.daily_summary());
            } else if detail {
                println!("{view}");
            } else {
                println!("{}", &view.daily_summary_detail());
            }
        }
        Commands::Latest { title } => match store.latest(&title) {
            Some(item) => println!("{item}"),
            None => println!("Not found"),
        },
    }
}

fn query_start_end(days_offset: u64, days: Option<u64>) -> (DateTime<Utc>, DateTime<Utc>) {
    let today_naive = Local::now().date_naive();
    let local_fixed_offset = Local.offset_from_local_date(&today_naive).unwrap();
    let today_naive = today_naive.and_hms_opt(0, 0, 0).unwrap();

    let start_offset_days = chrono::naive::Days::new(days_offset);
    let start_naive = today_naive.checked_sub_days(start_offset_days).unwrap();
    let end_offset_days = chrono::naive::Days::new(days.unwrap_or(days_offset + 1));
    let end_naive = start_naive.checked_add_days(end_offset_days).unwrap();
    let start_in_utc =
        DateTime::<FixedOffset>::from_local(start_naive, local_fixed_offset).with_timezone(&Utc);
    let end_in_utc =
        DateTime::<FixedOffset>::from_local(end_naive, local_fixed_offset).with_timezone(&Utc);

    (start_in_utc, end_in_utc)
}

fn read_until() -> String {
    let mut buf = String::new();
    while let Ok(n) = io::stdin().read_line(&mut buf) {
        if n == 0 {
            break;
        }
    }
    buf
}
