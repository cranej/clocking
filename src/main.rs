//use chrono::prelude::*;
use clap::{Parser, Subcommand};

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

fn main() {
    let cli = Cli::parse();
    println!("{cli:#?}");
}
