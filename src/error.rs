use crate::packet::MalformedPacketError;
use std::{
    error,
    fmt::{self, Display, Formatter},
    result,
};

#[cfg(feature = "ogg")]
use ogg::OggReadError;

#[cfg(feature = "ogg")]
use crate::ogg::OggOpusError;

#[derive(Debug)]
#[cfg_attr(not(feature = "ogg"), derive(Clone, Copy))]
#[allow(variant_size_differences)]
/// An error that has occured during decoding.
pub enum Error {
    /// A received packet was malformed.
    MalformedPacket(MalformedPacketError),
    #[cfg(feature = "ogg")]
    /// The Ogg container itself could not be read.
    Ogg(OggReadError),
    #[cfg(feature = "ogg")]
    /// The Opus stream within the Ogg container could not be read.
    OggOpus(OggOpusError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::MalformedPacket(err) => err.fmt(f),
            #[cfg(feature = "ogg")]
            Error::Ogg(err) => err.fmt(f),
            #[cfg(feature = "ogg")]
            Error::OggOpus(err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        "Opus decoding error"
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::MalformedPacket(err) => Some(err),
            #[cfg(feature = "ogg")]
            Error::Ogg(err) => Some(err),
            #[cfg(feature = "ogg")]
            Error::OggOpus(err) => Some(err),
        }
    }
}

impl From<MalformedPacketError> for Error {
    fn from(from: MalformedPacketError) -> Error {
        Error::MalformedPacket(from)
    }
}

#[cfg(feature = "ogg")]
impl From<OggReadError> for Error {
    fn from(from: OggReadError) -> Error {
        Error::Ogg(from)
    }
}

#[cfg(feature = "ogg")]
impl From<OggOpusError> for Error {
    fn from(from: OggOpusError) -> Error {
        Error::OggOpus(from)
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
