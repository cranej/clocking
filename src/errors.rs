use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Error {
    /// Unexpected error from underlying system, db, file system, etc.
    UnderlyingError(String),
    /// Usually caused by corrupted underlying storage, which should not
    /// happen if all operations are performed via this lib.
    ImpossibleState(String),
    /// User input is invald.
    InvalidInput(&'static str),
    /// Unfinished entry exists when trying to start a new one.
    UnfinishedExists(String),
    /// Entry with the same title and exact start time already exists.
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
