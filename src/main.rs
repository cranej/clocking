//use chrono::prelude::*;
use clap::{Parser, Subcommand};
use clocking::ClockingStore;
use clocking::sqlite_store::{SqliteStore};
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
        title: String
    },

    Report {
        ///Default to today
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        filter: Option<String>,
    },
}

const STORE_FILE_VAR: &str = "CLOCKING_FILE";
fn main() {
    let cli = Cli::parse();

    let store_file = cli.file.or_else(|| env::var(STORE_FILE_VAR).ok())
        .expect("Please specify storage file path either by environment or cli argument.");

    let mut store: Box::<dyn ClockingStore> = Box::new(SqliteStore::new(&store_file));

    match cli.command {
        Commands::Start {title} => {
            let id = store.start_clocking(&title).expect("Failed to start clocking");
            // read notes
            let notes = read_until();
            store.finish_clocking(&id, &notes);
        },
        Commands::Report {..} => {
            panic!("Unimplemented yet.");
        }
    }
}

fn read_until() -> String {
    let mut buf = String::new();
    while let Ok(go_on) = read_line(&mut buf) {
        if !go_on {
            break;
        }
    }

    buf
}

const STOP_SIGN: &str = ":q";
fn read_line(buf: &mut String) -> Result<bool, std::io::Error> {
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(n) if n > 0 => {
            if line.starts_with(STOP_SIGN) {
                Ok(false)
            } else {
                buf.push_str(&line);
                Ok(true)
            }
        },
        Ok(_) => {
            Ok(false)
        }
        Err(e) => Err(e),
    }
}
