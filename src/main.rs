use clap::{Parser, Subcommand};
use clocking::sqlite_store::SqliteStore;
use clocking::ClockingStore;
use std::env;
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// File to store the data, take priority of environment variable 'CLOCKING_FILE'.
    #[arg(long)]
    file: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start clocking
    ///
    /// Unless '-n/--no-wait' is specified, waits for Ctrl-D to finish clocking.
    ///
    /// All input before Ctrl-D will be saved as notes.
    Start {
        /// If not specified, interactively choose from recent titles.
        title: Option<String>,
        /// Do not wait for notes input, exit with unfinished status.
        #[arg(short, long)]
        no_wait: bool,
    },
    /// Finish latest unfinished clocking of title
    Finish {
        title: String,
        /// Can be specified multiple times, each as a separate line. Sinel value '-' means read from stdin
        #[arg(short, long)]
        notes: Vec<String>,
    },
    /// Report clocking data
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
    /// Server mode
    Server {
        /// Defatut to 8080
        #[arg(long, short)]
        port: Option<u16>,
        /// Default to 127.0.0.1
        #[arg(long, short)]
        addr: Option<std::net::IpAddr>,
    },
}

const STORE_FILE_VAR: &str = "CLOCKING_FILE";
#[rocket::main]
async fn main() {
    let cli = Cli::parse();

    let store_file = cli
        .file
        .or_else(|| env::var(STORE_FILE_VAR).ok())
        .expect("Please specify storage file path either by environment or cli argument.");

    match cli.command {
        Commands::Start { title, no_wait } => {
            let mut store: Box<dyn ClockingStore> = Box::new(SqliteStore::new(&store_file));

            let empty_title_err = "Empty title".to_string();
            let title = title
                .ok_or(empty_title_err.clone())
                .and_then(|x| {
                    if x.is_empty() {
                        Err(empty_title_err.clone())
                    } else {
                        Ok(x)
                    }
                })
                .or_else(|_| {
                    let recent_titles = store.recent_titles(5);
                    read_title(&recent_titles)
                });

            match title {
                Ok(title) => {
                    let id = store
                        .start_clocking(&title)
                        .expect("Failed to start clocking");
                    println!("(Started)");
                    if !no_wait {
                        println!("(Ctrl-D to finish clocking)");
                        let notes = read_to_end();
                        store.finish_clocking(&id, &notes);
                        println!("(Finished)");
                    };
                }
                Err(e) => {
                    eprintln!("{e}");
                }
            };
        }
        Commands::Finish { title, notes } => {
            let mut store: Box<dyn ClockingStore> = Box::new(SqliteStore::new(&store_file));

            let notes = if notes.len() == 1 && notes[0] == "-" {
                read_to_end()
            } else {
                notes.join("\n")
            };

            match store.finish_latest_unfinished_by_title(&title, &notes) {
                Ok(true) => println!("(Updated)"),
                Ok(false) => println!("(No unfinished item found by {title})"),
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
            let store: Box<dyn ClockingStore> = Box::new(SqliteStore::new(&store_file));
            let items = store.query_clocking_offset(from.unwrap_or(0), days);
            let view = clocking::views::ItemView::new(&items);

            if daily_summary {
                println!("{}", &view.daily_summary());
            } else if detail {
                println!("{view}");
            } else {
                println!("{}", &view.daily_summary_detail());
            }
        }
        Commands::Latest { title } => {
            let store: Box<dyn ClockingStore> = Box::new(SqliteStore::new(&store_file));

            match store.latest(&title) {
                Some(item) => println!("{item}"),
                None => println!("(Not found)"),
            }
        }
        Commands::Server { port, addr } => {
            let config = rocket::config::Config {
                port: port.unwrap_or(8080),
                address: addr
                    .unwrap_or_else(|| std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
                ..rocket::config::Config::default()
            };

            let server_config = clocking::server::ServerConfig {
                db_file: store_file.clone(),
            };
            let _rocket = rocket::custom(&config)
                .manage(server_config)
                .mount(
                    "/api",
                    rocket::routes![
                        clocking::server::api_recent,
                        clocking::server::api_latest,
                        clocking::server::api_unfinished,
                        clocking::server::api_start,
                        clocking::server::api_finish,
                        clocking::server::api_report,
                    ],
                )
                .mount(
                    "/",
                    rocket::routes![clocking::server::index, clocking::server::anyfile,],
                )
                .ignite()
                .await
                .unwrap()
                .launch()
                .await;
        }
    }
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
        for (i, t) in recent_titles.iter().enumerate() {
            println!("{}: {t}", i + 1);
        }
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
