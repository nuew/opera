//! Decoding of Ogg-encapsulated Opus streams.
#![cfg(feature = "ogg")]

use crate::slice_ext::{BoundsError, SliceExt};
use ogg::PacketReader;
use std::{
    convert::TryFrom,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    io::prelude::*,
    iter::FusedIterator,
    num::{NonZeroU32, NonZeroU8},
};

/// The error type returned when the Ogg Opus stream is malformed.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum OggOpusError {
    /// Has the same meaning as [`std::io::ErrorKind::UnexpectedEof`]
    ///
    /// [`std::io::ErrorKind::UnexpectedEof`]: https://doc.rust-lang.org/nightly/std/io/enum.ErrorKind.html#variant.UnexpectedEof
    UnexpectedEof,
    /// Stream rejected due to a suspected denial-of-service attack.
    DenialOfService,
    /// The Ogg Header packets are incorrectly page-aligned.
    BadPaging,
    /// Either of the Identification Header or the Comment Header had the wrong magic number.
    BadMagic,
    /// The Identificaion Header indicated that this Ogg file conforms to an unsupported version of
    /// the specification.
    UnsupportedVersion,
    /// The specified channel layout is malformed or otherwise invalid.
    InvalidChannelLayout,
}

impl Display for OggOpusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OggOpusError::UnexpectedEof => "data ended early (unexpected eof)",
            OggOpusError::DenialOfService => "suspected denial-of-service attack",
            OggOpusError::BadPaging => "bad ogg paging alignment",
            OggOpusError::BadMagic => "invalid magic number",
            OggOpusError::UnsupportedVersion => "unsupported encapsulation specification version",
            OggOpusError::InvalidChannelLayout => "invalid channel layout",
        })
    }
}

impl Error for OggOpusError {}

impl From<BoundsError> for OggOpusError {
    fn from(_from: BoundsError) -> OggOpusError {
        OggOpusError::UnexpectedEof
    }
}

type Result<T> = ::std::result::Result<T, OggOpusError>;

/// RTP-style channel mapping layouts.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum RtpChannelLayout {
    /// monomorphic
    Mono = 1,
    /// stereo (left, right)
    Stereo = 2,
}

impl TryFrom<u8> for RtpChannelLayout {
    type Error = OggOpusError;

    fn try_from(v: u8) -> ::std::result::Result<Self, Self::Error> {
        match v {
            1 => Ok(RtpChannelLayout::Mono),
            2 => Ok(RtpChannelLayout::Stereo),
            _ => Err(OggOpusError::InvalidChannelLayout),
        }
    }
}

/// Vorbis-style channel mapping layouts.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum VorbisChannelLayout {
    /// monomorphic
    Mono = 1,
    /// stereo (left, right)
    Stereo = 2,
    /// linear surround (left, center, right)
    LinearSurround = 3,
    /// quadraphonic (front left, front right, rear left, rear right)
    Quadraphonic = 4,
    /// 5.0 surround (front left, front center, front right, rear left, rear right)
    FivePointZero = 5,
    /// 5.1 surround (front left, front center, front right, rear left, rear right, LFE)
    FivePointOne = 6,
    /// 6.1 surround (front left, front center, front right, side left, side right, rear center, LFE)
    SixPointOne = 7,
    /// 7.1 surround (front left, front center, front right, side left, side right, rear left, rear right, LFE)
    SevenPointOne = 8,
}

impl TryFrom<u8> for VorbisChannelLayout {
    type Error = OggOpusError;

    fn try_from(v: u8) -> ::std::result::Result<Self, Self::Error> {
        match v {
            1 => Ok(VorbisChannelLayout::Mono),
            2 => Ok(VorbisChannelLayout::Stereo),
            3 => Ok(VorbisChannelLayout::LinearSurround),
            4 => Ok(VorbisChannelLayout::Quadraphonic),
            5 => Ok(VorbisChannelLayout::FivePointZero),
            6 => Ok(VorbisChannelLayout::FivePointOne),
            7 => Ok(VorbisChannelLayout::SixPointOne),
            8 => Ok(VorbisChannelLayout::SevenPointOne),
            _ => Err(OggOpusError::InvalidChannelLayout),
        }
    }
}

/// The channel mapping family and channel layout for an Ogg Opus stream.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum ChannelMappingFamily {
    /// RTP-style channel mapping
    RTP(RtpChannelLayout),
    /// Vorbis-style channel mapping
    Vorbis(VorbisChannelLayout),
    /// Undefined channel mapping
    Undefined {
        /// The number of channels in use.
        channels: u8,
    },
}

impl ChannelMappingFamily {
    fn new(channels: u8, family: u8) -> Result<Self> {
        match family {
            0 => Ok(ChannelMappingFamily::RTP(RtpChannelLayout::try_from(
                channels,
            )?)),
            1 => Ok(ChannelMappingFamily::Vorbis(VorbisChannelLayout::try_from(
                channels,
            )?)),
            255 | _ => Ok(ChannelMappingFamily::Undefined { channels }),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
struct ChannelMapping {
    /// The channel mappping family.
    family: ChannelMappingFamily,
    /// The number of streams encoded in each Ogg packet.
    streams: NonZeroU8,
    /// The number of streams whose decoders are to be configured to produce two channels (stereo
    /// sound). This must not be larger then `streams`.
    coupled_streams: u8,
}

impl ChannelMapping {
    fn new(channels: u8, family: u8, _table: &[u8]) -> Result<ChannelMapping> {
        let family = ChannelMappingFamily::new(channels, family)?;

        let (streams, coupled_streams) = if let ChannelMappingFamily::RTP(_) = family {
            (NonZeroU8::new(1).unwrap(), channels - 1)
        } else {
            unimplemented!()
        };

        Ok(ChannelMapping {
            family,
            streams,
            coupled_streams,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct IdHeader {
    /// Encapsulation specification version.
    version: u8,
    /// Output channel configuration.
    channels: ChannelMapping,
    /// Number of samples (at 48 kHz) to discard when beginning playback.
    pre_skip: u16,
    /// Sample rate of the original input (before encoding) in Hz.
    ///
    /// This is _not_ the sample rate to use for playback.
    sample_rate: Option<NonZeroU32>,
    /// 20*log_10 of the factor by which to scale the decoder output to
    /// receive the desired playback volume.
    output_gain: i16,
}

impl IdHeader {
    /// Human-Readable codec identification.
    const MAGIC: [u8; 8] = *b"OpusHead";

    /// Major (incompatible) version subfield mask.
    const VERSION_MAJOR_MASK: u8 = 0b1111_0000;

    /// Minor (compatible) version subfield mask.
    #[allow(unused)]
    const VERSION_MINOR_MASK: u8 = 0b0000_1111;

    /// Create a new ID header representation from bytes.
    fn new(data: &[u8]) -> Result<Self> {
        use byteorder::{ByteOrder, LE};

        if data.get_res(..8)? == Self::MAGIC {
            let version = *data.get_res(8)?;

            if version & IdHeader::VERSION_MAJOR_MASK == 0 {
                Ok(IdHeader {
                    version,
                    channels: ChannelMapping::new(
                        *data.get_res(9)?,
                        *data.get_res(18)?,
                        data.get_res(19..)?,
                    )?,
                    pre_skip: LE::read_u16(data.get_res(10..=11)?),
                    sample_rate: NonZeroU32::new(LE::read_u32(data.get_res(12..=15)?)),
                    output_gain: LE::read_i16(data.get_res(15..=16)?),
                })
            } else {
                Err(OggOpusError::UnsupportedVersion)
            }
        } else {
            Err(OggOpusError::BadMagic)
        }
    }

    /// Returns the encoding sample rate.
    fn sample_rate(&self) -> Option<NonZeroU32> {
        self.sample_rate
    }
}

/// An iterator over user comments.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Comments<'a> {
    comments: &'a [u8],
    comments_num: u32,
    comments_read: u32,
    pos: usize,
}

impl<'a> Iterator for Comments<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        use byteorder::{ByteOrder, LE};
        use std::str::from_utf8;

        if self.pos < self.comments.len() && self.comments_read < self.comments_num {
            // get comment length
            let cmt_start = self.pos + 4;
            let cmt_len = LE::read_u32(self.comments.get(self.pos..cmt_start)?) as usize;

            // bookkeeping
            self.pos = cmt_start + cmt_len;
            self.comments_read += 1;

            // parse comment; if it's invalid utf-8, just move on to the next one.
            let (name, value) = match from_utf8(self.comments.get(cmt_start..self.pos)?) {
                Ok(cmt) => cmt.split_at(cmt.find('=')?),
                Err(_) => return self.next(),
            };

            Some((name, &value[1..]))
        } else {
            None
        }
    }
}

impl FusedIterator for Comments<'_> {}

#[derive(PartialEq, Eq, Clone, Hash)]
struct CommentHeader {
    comments: Box<[u8]>,
    comments_num: u32,
    vendor: String,
}

impl CommentHeader {
    /// Human-Readable codec identification.
    const MAGIC: [u8; 8] = *b"OpusTags";

    /// Maximum length of the packet.
    const PACKET_LEN_MAX: usize = 125_829_120;

    /// Packet position after which to ignore comments.
    const COMMENTS_IGNORE_LEN: usize = 61_440;

    /// Create a new comment header representation from bytes.
    fn new(data: &[u8]) -> Result<Self> {
        use byteorder::{ByteOrder, LE};

        // Denial-of-Service check
        if data.len() > Self::PACKET_LEN_MAX {
            Err(OggOpusError::DenialOfService)
        } else if data.get_res(..8)? == Self::MAGIC {
            // only parses the vendor string (for debugging) at initialization
            let comments_start = 12 + LE::read_u32(data.get_res(8..12)?) as usize;
            let vendor = String::from_utf8_lossy(data.get_res(12..comments_start)?).into_owned();
            let num_comments = LE::read_u32(data.get_res(comments_start..comments_start + 4)?);

            // we still save the comment data so that we can parse it later if necessary.
            // also, some more DOS checks
            let comments = if data.len() <= Self::COMMENTS_IGNORE_LEN {
                &data[comments_start + 4..]
            } else {
                &data[comments_start + 4..Self::COMMENTS_IGNORE_LEN]
            }
            .to_owned()
            .into_boxed_slice();

            Ok(CommentHeader {
                comments,
                comments_num: num_comments,
                vendor,
            })
        } else {
            Err(OggOpusError::BadMagic)
        }
    }

    /// Returns an iterator over the user comments.
    fn comments(&self) -> Comments<'_> {
        Comments {
            comments: &self.comments[..],
            comments_num: self.comments_num,
            comments_read: 0,
            pos: 0,
        }
    }

    /// Returns the vendor string.
    fn vendor(&self) -> &str {
        &self.vendor[..]
    }
}

impl Debug for CommentHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("CommentHeader");

        ds.field("vendor", &self.vendor());
        for (name, value) in self.comments() {
            ds.field(name, &value);
        }

        ds.finish()
    }
}

/// A reader for Ogg Opus files and/or streams.
pub struct OggOpusReader<R: Read + Seek> {
    reader: PacketReader<R>,
    id_header: IdHeader,
    comments: CommentHeader,
}

impl<R> OggOpusReader<R>
where
    R: Read + Seek,
{
    /// Creates a new `OggOpusReader` from the given reader.
    pub fn new(reader: R) -> crate::error::Result<Self> {
        let mut reader = PacketReader::new(reader);

        // read id header
        let id_packet = reader.read_packet_expected()?;
        let id_header =
            if id_packet.first_in_stream() && id_packet.first_in_page() && id_packet.last_in_page()
            {
                IdHeader::new(&id_packet.data[..])?
            } else {
                return Err(OggOpusError::BadPaging.into());
            };

        // read comment header
        let comments_packet = reader.read_packet_expected()?;
        let comments = if id_packet.first_in_page() && id_packet.last_in_page() {
            CommentHeader::new(&comments_packet.data[..])?
        } else {
            return Err(OggOpusError::BadPaging.into());
        };

        Ok(OggOpusReader {
            reader,
            id_header,
            comments,
        })
    }

    /// Returns an iterator over user comments contained in the Vorbis comments block.
    pub fn comments(&self) -> Comments<'_> {
        self.comments.comments()
    }

    /// Returns the sample rate of the media this file was encoded from, in Hz.
    ///
    /// Note that this is not necessarily the sample rate it will be played back at.
    pub fn sample_rate(&self) -> Option<NonZeroU32> {
        self.id_header.sample_rate()
    }

    /// Returns the encoder vendor string from the Vorbis comment block.
    pub fn vendor(&self) -> &str {
        self.comments.vendor()
    }

    /// Returns the wrapped reader, consuming the `OggOpusReader`.
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }
}

impl<R> Debug for OggOpusReader<R>
where
    R: Read + Seek,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        #[derive(PartialEq, Eq, Clone, Copy, Hash)]
        struct ElidedStruct<'a>(&'a str);
        impl Debug for ElidedStruct<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.pad(self.0)
            }
        }

        f.debug_struct("OggOpusReader")
            .field("reader", &ElidedStruct("PacketReader"))
            .field("id_header", &self.id_header)
            .field("comments", &self.comments)
            .finish()
    }
}
