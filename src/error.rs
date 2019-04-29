use std::{
    error,
    fmt::{self, Display, Formatter},
    io, result,
};

#[derive(Debug)]
/// An error that has occured during decoding.
pub enum Error {
    /// An error in an underlying I/O operation.
    Io(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "Opus decoding error"
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(from: io::Error) -> Error {
        Error::Io(from)
    }
}

/// A specialized [`Result`] type for Opus decoding.
///
/// [`Result`]: https://doc.rust-lang.org/stable/std/result/enum.Result.html
pub type Result<T> = result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Error>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Error>();
    }
}
