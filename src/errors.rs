use std::fmt;

#[derive(Debug)]
pub enum Error {
    UnderlyingError(String),
    ImpossibleState(String),
    InvalidInput(String),
    UnfinishedExists(String),
    DuplicateEntry,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnderlyingError(err) => {
                writeln!(f, "Something wrong with underlying implementation: {err}")
            }
            Error::ImpossibleState(err) => writeln!(f, "While, this should never happen: {err}"),
            Error::InvalidInput(err) => writeln!(f, "Input invalid: {err}"),
            Error::UnfinishedExists(title) => writeln!(
                f,
                "Starting new entry is not allowed when there is unfinished entry: {title}"
            ),
            Error::DuplicateEntry => {
                writeln!(f, "An entry with the same title and start already exists.")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Error {
        Error::UnderlyingError(err.to_string())
    }
}
