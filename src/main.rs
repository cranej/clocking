use clap::{Parser, Subcommand};
use clocking::{errors, new_sqlite_store, ClockingStore};
use std::env;
use std::io::{self, Write};
use std::sync::Mutex;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about, propagate_version = true)]
/// Observe the time
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// File path to store the data, required if no environment variable 'CLOCKING_FILE' detected.  Take priority of environment variable 'CLOCKING_FILE'.
    #[arg(long)]
    file: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start clocking
    ///
    /// Unless '-n/--no-wait' is specified, waits for Ctrl-D to finish clocking. \
    /// All input before Ctrl-D will be saved as notes.
    Start {
        /// If not specified, interactively choose from recent titles.
        title: Option<String>,
        /// Do not wait for notes input, exit with unfinished status.
        #[arg(short, long)]
        no_wait: bool,
    },
    /// Finish latest unfinished clocking of title.
    Finish {
        /// Can be specified multiple times, each as a separate line. Sinel value '-' means read from stdin.
        #[arg(short, long)]
        notes: Vec<String>,
    },
    /// Report clocking data.
    Report {
        ///Tail offset. Default to 0 - today
        #[arg(short, long)]
        from: Option<u64>,
        ///Limit days from tail offset. Default to until now
        #[arg(short, long, value_parser = clap::value_parser!(u64).range(1..))]
        days: Option<u64>,
        ///Show daily summary
        #[arg(long = "daily")]
        daily_summary: bool,
        ///Show detail report
        #[arg(long)]
        detail: bool,
        /// Show daily distribution
        #[arg(long = "dist")]
        daily_dist: bool,
        ///<Unimplemented yet>.
        #[arg(long)]
        filter: Option<String>,
    },
    /// Show details of latest record of item 'title'.
    Latest {
        /// Title of the item to display. Choose interactively if not specified.
        title: Option<String>,
    },
    /// Shoe latest unfinished entry
    Ongoing,
    /// Show latest n titles
    Titles {
        /// Number of titles to show
        #[arg(long, short, default_value_t = 5)]
        number: usize,
        /// If on, prefix titles by index start from 1
        #[arg(long, short, default_value_t = false)]
        index: bool,
    },
    /// Server mode
    Server {
        /// Default to 8080
        #[arg(long, short)]
        port: Option<u16>,
        /// Default to 127.0.0.1
        #[arg(long, short)]
        addr: Option<std::net::IpAddr>,
    },
}

const STORE_FILE_VAR: &str = "CLOCKING_FILE";
const RECENT_TITLE_LIMIT: usize = 5;
#[rocket::main]
async fn main() -> Result<(), errors::Error> {
    let cli = Cli::parse();

    let store_file = cli
        .file
        .or_else(|| env::var(STORE_FILE_VAR).ok())
        .expect("Please specify storage file path either by environment or cli argument --file before any command.");

    match cli.command {
        Commands::Start { title, no_wait } => {
            let mut store = new_sqlite_store(&store_file);
            let title = handle_title(title, &store.recent_titles(RECENT_TITLE_LIMIT)?);
            match title {
                Ok(title) => {
                    let _ = store.start(&title)?;
                    println!("(Started)");
                    if !no_wait {
                        println!("(Ctrl-D to finish clocking)");
                        let notes = read_to_end();
                        if store.try_finish_any(&notes).is_ok() {
                            println!("(Finished)");
                        } else {
                            return Err(errors::Error::ImpossibleState(
                                    "We should be able to finish it, but somehow it's already finished...".to_string()));
                        }
                    };
                }
                Err(e) => {
                    eprintln!("{e}");
                }
            };
        }
        Commands::Finish { notes } => {
            let mut store = new_sqlite_store(&store_file);
            let notes = if notes.len() == 1 && notes[0] == "-" {
                read_to_end()
            } else {
                notes.join("\n")
            };
            match store.try_finish_any(&notes) {
                Ok(Some(title)) => println!("(Finished: {title})"),
                Ok(None) => println!("(No unfinished item found)"),
                Err(e) => eprintln!("Unexpected error: {e}"),
            }
        }
        Commands::Report {
            from,
            days,
            daily_summary,
            detail,
            daily_dist,
            ..
        } => {
            let store = new_sqlite_store(&store_file);
            let entries = store.finished_by_offset(from.unwrap_or(0), days)?;

            if daily_summary {
                let view = clocking::views::DailySummaryView::new(&entries);
                println!("{view}");
            } else if detail {
                let view = clocking::views::EntryDetailView::new(&entries);
                println!("{view}");
            } else if daily_dist {
                let view = clocking::views::DailyDistributionView::new(&entries);
                println!("{view}");
            } else {
                let view = clocking::views::DailyDetailView::new(&entries);
                println!("{view}");
            }
        }
        Commands::Latest { title } => {
            let store = new_sqlite_store(&store_file);

            let title = handle_title(title, &store.recent_titles(RECENT_TITLE_LIMIT)?);
            match title {
                Ok(title) => match store.latest_finished(&title)? {
                    Some(item) => println!("{item}"),
                    None => println!("(Not found)"),
                },
                Err(err) => eprintln!("Error reading or choosing title: {err}."),
            }
        }
        Commands::Ongoing => match new_sqlite_store(&store_file).unfinished(1)?.pop() {
            Some(entry) => {
                println!("{}", &entry.id.title);
                println!("{} minutes ago", entry.started_minutes());
            }
            None => println!("No ongoing entry."),
        },
        Commands::Titles { number, index } => {
            let store = new_sqlite_store(&store_file);
            print_titles(&store.recent_titles(number)?, index);
        }
        Commands::Server { port, addr } => {
            // TODO: understand why T is Send makes Mutex<T> both Send and Sync
            let store = Box::new(Mutex::new(new_sqlite_store(&store_file)));
            let _ = clocking::server::launch_server(
                port.unwrap_or(8080),
                addr.unwrap_or_else(|| std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
                None,
                store,
            )
            .await;
        }
    }

    Ok(())
}

fn handle_title(title: Option<String>, recent_titles: &[String]) -> Result<String, String> {
    let empty_title_err = "Empty title".to_string();
    title
        .ok_or_else(|| empty_title_err.clone())
        .and_then(|x| {
            if x.is_empty() {
                Err(empty_title_err.clone())
            } else {
                Ok(x)
            }
        })
        .or_else(|_| read_title(recent_titles))
}

fn read_title(recent_titles: &[String]) -> Result<String, String> {
    if recent_titles.is_empty() {
        // read title from input
        print!("Input Title: ");
        io::stdout().flush().unwrap();
        let input = read_or_panic();
        let input = input.trim();
        if input.is_empty() {
            Err("Title cannot be empty.".to_string())
        } else {
            Ok(input.to_string())
        }
    } else {
        // choose from recent titles
        print_titles(recent_titles, true);
        print!("Choose by index (default 1): ");
        io::stdout().flush().unwrap();
        let input = read_or_panic();
        let input = input.trim();
        if input.is_empty() {
            Ok(recent_titles[0].clone())
        } else {
            match input.parse::<usize>() {
                Ok(i) if i <= recent_titles.len() && i > 0 => Ok(recent_titles[i - 1].clone()),
                Ok(i) => Err(format!("Invalid index: {i}.")),
                Err(e) => Err(format!("Invalid input: {e}.")),
            }
        }
    }
}

fn print_titles(titles: &[String], index: bool) {
    if index {
        for (i, t) in titles.iter().enumerate() {
            println!("{}: {t}", i + 1);
        }
    } else {
        for t in titles.iter() {
            println!("{t}");
        }
    }
}

fn read_to_end() -> String {
    let mut buf = String::new();
    while let Ok(n) = io::stdin().read_line(&mut buf) {
        if n == 0 {
            break;
        }
    }
    buf
}

fn read_or_panic() -> String {
    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
        Ok(n) if n > 0 => buf,
        Ok(n) if n == 0 => buf,
        Ok(n) => panic!("Unexpected read bytes: {n}"),
        Err(e) => panic!("Unexpected read error: {e}"),
    }
}
