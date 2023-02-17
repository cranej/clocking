use std::fmt;

#[derive(Debug)]
pub enum Error {
    DbError(rusqlite::Error),
    ImpossibleState(String),
    InvalidInput(String),
    UnfinishedExists(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DbError(err) => err.fmt(f),
            Error::ImpossibleState(err) =>
                writeln!(f, "While, this should never happen: {err}"),
            Error::InvalidInput(err) => writeln!(f, "Input invalid: {err}"),
            Error::UnfinishedExists(title) =>
                writeln!(f, "Starting new entry is not allowed when there is unfinished entry: {title}"),

        }
    }
}

impl std::error::Error for Error {}
