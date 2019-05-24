//! Decoding of Ogg-encapsulated Opus streams.
#![cfg(feature = "ogg")]

use crate::{
    packet::{Frame, Packet},
    slice_ext::{BoundsError, SliceExt},
};
use ogg::PacketReader;
use std::{
    collections::VecDeque,
    convert::TryFrom,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    io::prelude::*,
    num::NonZeroU32,
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
    /// The specified channel layout or mapping is malformed, unsupported, or otherwise invalid.
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
enum RtpChannelLayout {
    /// monomorphic
    Mono = 1,
    /// stereo (left, right)
    Stereo = 2,
}

impl TryFrom<u8> for RtpChannelLayout {
    type Error = OggOpusError;

    fn try_from(v: u8) -> Result<Self> {
        match v {
            1 => Ok(RtpChannelLayout::Mono),
            2 => Ok(RtpChannelLayout::Stereo),
            _ => Err(OggOpusError::InvalidChannelLayout),
        }
    }
}

/// Vorbis-style channel mapping layouts.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum VorbisChannelLayout {
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

    fn try_from(v: u8) -> Result<Self> {
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

/// Ambisonics channel mapping layouts.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum AmbisonicsChannelLayout {
    /// Zeroth-order Ambisonics
    Zero = 1,
    /// Zeroth-order Ambisonics with non-diegetic stereo stream
    ZeroNonDiegetic = 3,
    /// First-order Ambisonics
    One = 4,
    /// First-order Ambisonics with non-diegetic stereo stream
    OneNonDiegetic = 6,
    /// Second-order Ambisonics
    Two = 9,
    /// Second-order Ambisonics with non-diegetic stereo stream
    TwoNonDiegetic = 11,
    /// Third-order Ambisonics
    Three = 16,
    /// Third-order Ambisonics with non-diegetic stereo stream
    ThreeNonDiegetic = 18,
    /// Fourth-order Ambisonics
    Four = 25,
    /// Fourth-order Ambisonics with non-diegetic stereo stream
    FourNonDiegetic = 27,
    /// Fifth-order Ambisonics
    Five = 36,
    /// Fifth-order Ambisonics with non-diegetic stereo stream
    FiveNonDiegetic = 38,
    /// Sixth-order Ambisonics
    Six = 49,
    /// Sixth-order Ambisonics with non-diegetic stereo stream
    SixNonDiegetic = 51,
    /// Seventh-order Ambisonics
    Seven = 64,
    /// Seventh-order Ambisonics with non-diegetic stereo stream
    SevenNonDiegetic = 66,
    /// Eighth-order Ambisonics
    Eight = 81,
    /// Eighth-order Ambisonics with non-diegetic stereo stream
    EightNonDiegetic = 83,
    /// Ninth-order Ambisonics
    Nine = 100,
    /// Ninth-order Ambisonics with non-diegetic stereo stream
    NineNonDiegetic = 102,
    /// Tenth-order Ambisonics
    Ten = 121,
    /// Tenth-order Ambisonics with non-diegetic stereo stream
    TenNonDiegetic = 123,
    /// Eleventh-order Ambisonics
    Eleven = 144,
    /// Eleventh-order Ambisonics with non-diegetic stereo stream
    ElevenNonDiegetic = 146,
    /// Twelfth-order Ambisonics
    Twelve = 169,
    /// Twelfth-order Ambisonics with non-diegetic stereo stream
    TwelveNonDiegetic = 171,
    /// Thirteenth-order Ambisonics
    Thirteen = 196,
    /// Thirteenth-order Ambisonics with non-diegetic stereo stream
    ThirteenNonDiegetic = 198,
    /// Fourteenth-order Ambisonics
    Fourteen = 225,
    /// Fourteenth-order Ambisonics with non-diegetic stereo stream
    FourteenNonDiegetic = 227,
}

impl TryFrom<u8> for AmbisonicsChannelLayout {
    type Error = OggOpusError;

    fn try_from(v: u8) -> Result<Self> {
        match v {
            1 => Ok(AmbisonicsChannelLayout::Zero),
            3 => Ok(AmbisonicsChannelLayout::ZeroNonDiegetic),
            4 => Ok(AmbisonicsChannelLayout::One),
            6 => Ok(AmbisonicsChannelLayout::OneNonDiegetic),
            9 => Ok(AmbisonicsChannelLayout::Two),
            11 => Ok(AmbisonicsChannelLayout::TwoNonDiegetic),
            16 => Ok(AmbisonicsChannelLayout::Three),
            18 => Ok(AmbisonicsChannelLayout::ThreeNonDiegetic),
            25 => Ok(AmbisonicsChannelLayout::Four),
            27 => Ok(AmbisonicsChannelLayout::FourNonDiegetic),
            36 => Ok(AmbisonicsChannelLayout::Five),
            38 => Ok(AmbisonicsChannelLayout::FiveNonDiegetic),
            49 => Ok(AmbisonicsChannelLayout::Six),
            51 => Ok(AmbisonicsChannelLayout::SixNonDiegetic),
            64 => Ok(AmbisonicsChannelLayout::Seven),
            66 => Ok(AmbisonicsChannelLayout::SevenNonDiegetic),
            81 => Ok(AmbisonicsChannelLayout::Eight),
            83 => Ok(AmbisonicsChannelLayout::EightNonDiegetic),
            100 => Ok(AmbisonicsChannelLayout::Nine),
            102 => Ok(AmbisonicsChannelLayout::NineNonDiegetic),
            121 => Ok(AmbisonicsChannelLayout::Ten),
            123 => Ok(AmbisonicsChannelLayout::TenNonDiegetic),
            144 => Ok(AmbisonicsChannelLayout::Eleven),
            146 => Ok(AmbisonicsChannelLayout::ElevenNonDiegetic),
            169 => Ok(AmbisonicsChannelLayout::Twelve),
            171 => Ok(AmbisonicsChannelLayout::TwelveNonDiegetic),
            196 => Ok(AmbisonicsChannelLayout::Thirteen),
            198 => Ok(AmbisonicsChannelLayout::ThirteenNonDiegetic),
            225 => Ok(AmbisonicsChannelLayout::Fourteen),
            227 => Ok(AmbisonicsChannelLayout::FourteenNonDiegetic),
            _ => Err(OggOpusError::InvalidChannelLayout),
        }
    }
}

/// Channel Mapping table as defined in RFC 7845
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
struct StandardMappingTable {
    /// The number of streams encoded in each Ogg packet.
    streams: u8,
    /// The number of streams whose decoders are to be configured to produce two channels
    /// (stereo sound). Must not be larger then `streams`.
    coupled: u8,
    /// A mapping of decoded channels to output channels.
    mapping: Vec<u8>,
}

impl StandardMappingTable {
    fn new(channels: u8, table: &[u8]) -> Result<Self> {
        let streams = *table.get_res(0)?;
        let coupled = *table.get_res(1)?;

        // check for invalid mappings
        if streams == 0 || streams < coupled || usize::from(streams) + usize::from(coupled) > 255 {
            return Err(OggOpusError::InvalidChannelLayout);
        }

        Ok(StandardMappingTable {
            streams,
            coupled,
            mapping: table.get_res(2..2 + usize::from(channels))?.to_owned(),
        })
    }

    /// Returns the number of streams encoded in each Ogg packet.
    #[inline]
    fn streams(&self) -> u8 {
        self.streams
    }

    /// Returns the number of streams whose decoders are to be configured to produce two channels
    /// (stereo sound). Will not be larger then the return value of [`StandardMappingTable::streams`].
    ///
    /// [`StandardMappingTable::streams`]: #method.streams
    #[inline]
    fn coupled(&self) -> u8 {
        self.coupled
    }
}

/// Ambisonics channel mapping table (for mapping type 3)
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
struct AmbisonicsMappingTable {
    /// The number of streams encoded in each Ogg packet.
    streams: u8,
    /// The number of streams whose decoders are to be configured to produce two channels
    /// (stereo sound). Must not be larger then `streams`.
    coupled: u8,
    /// The demixing matrix
    matrix: Vec<u16>,
}

impl AmbisonicsMappingTable {
    fn new(channels: u8, table: &[u8]) -> Result<Self> {
        use byteorder::{ByteOrder, LE};

        let streams = *table.get_res(0)?;
        let coupled = *table.get_res(1)?;

        // check for invalid mappings
        if streams == 0 || streams < coupled || usize::from(streams) + usize::from(coupled) > 255 {
            return Err(OggOpusError::InvalidChannelLayout);
        }

        Ok(AmbisonicsMappingTable {
            streams,
            coupled,
            matrix: table
                .get_res(2..2 + (2 * usize::from(channels)))?
                .chunks_exact(2)
                .map(LE::read_u16)
                .collect(),
        })
    }

    /// Returns the number of streams encoded in each Ogg packet.
    #[inline]
    fn streams(&self) -> u8 {
        self.streams
    }

    /// Returns the number of streams whose decoders are to be configured to produce two channels
    /// (stereo sound). Will not be larger then the return value of [`AmbisonicsMappingTable::streams`].
    ///
    /// [`AmbisonicsMappingTable::streams`]: #method.streams
    #[inline]
    fn coupled(&self) -> u8 {
        self.coupled
    }
}

/// The channel mapping family and channel layout for an Ogg Opus stream.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
enum ChannelMapping {
    /// Mono, L/R stereo
    RTP(RtpChannelLayout),
    /// 1-8 channel surround
    Vorbis {
        /// The channel layout.
        layout: VorbisChannelLayout,
        /// The Ogg packet channel to output channel mapping.
        mapping: StandardMappingTable,
    },
    /// Ambisonics as individual channels
    AmbisonicsIndividual {
        /// The channel layout.
        layout: AmbisonicsChannelLayout,
        /// The Ogg packet channel to output channel mapping.
        mapping: StandardMappingTable,
    },
    /// Ambisonics with demixing matrix
    AmbisonicsDemixed {
        /// The channel layout.
        layout: AmbisonicsChannelLayout,
        /// The Ogg packet channel to output channel mapping.
        mapping: AmbisonicsMappingTable,
    },
    /// Discrete channels
    Discrete {
        /// The number of channels in use.
        channels: u8,
        /// The Ogg packet channel to output channel mapping.
        mapping: StandardMappingTable,
    },
}

impl ChannelMapping {
    fn new(channels: u8, family: u8, table: &[u8]) -> Result<Self> {
        match family {
            0 => Ok(ChannelMapping::RTP(RtpChannelLayout::try_from(channels)?)),
            1 => Ok(ChannelMapping::Vorbis {
                layout: VorbisChannelLayout::try_from(channels)?,
                mapping: StandardMappingTable::new(channels, table)?,
            }),
            2 => Ok(ChannelMapping::AmbisonicsIndividual {
                layout: AmbisonicsChannelLayout::try_from(channels)?,
                mapping: StandardMappingTable::new(channels, table)?,
            }),
            3 => Ok(ChannelMapping::AmbisonicsDemixed {
                layout: AmbisonicsChannelLayout::try_from(channels)?,
                mapping: AmbisonicsMappingTable::new(channels, table)?,
            }),
            255 => Ok(ChannelMapping::Discrete {
                channels,
                mapping: StandardMappingTable::new(channels, table)?,
            }),
            _ => Err(OggOpusError::InvalidChannelLayout),
        }
    }

    /// Returns the number of streams encoded in each Ogg packet.
    fn streams(&self) -> u8 {
        match self {
            ChannelMapping::RTP(_) => 1,
            ChannelMapping::Vorbis { mapping, .. } => mapping.streams(),
            ChannelMapping::AmbisonicsIndividual { mapping, .. } => mapping.streams(),
            ChannelMapping::AmbisonicsDemixed { mapping, .. } => mapping.streams(),
            ChannelMapping::Discrete { mapping, .. } => mapping.streams(),
        }
    }

    /// Returns the number of streams whose decoders are to be configured to produce two channels
    /// (stereo sound). Will not be larger then the return value of [`ChannelMapping::streams`].
    ///
    /// [`ChannelMapping::streams`]: #method.streams
    fn coupled_streams(&self) -> u8 {
        match self {
            ChannelMapping::RTP(layout) => *layout as u8 - 1,
            ChannelMapping::Vorbis { mapping, .. } => mapping.coupled(),
            ChannelMapping::AmbisonicsIndividual { mapping, .. } => mapping.coupled(),
            ChannelMapping::AmbisonicsDemixed { mapping, .. } => mapping.coupled(),
            ChannelMapping::Discrete { mapping, .. } => mapping.coupled(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
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

    /// Returns the encapsulation specification version as (major, minor).
    fn version(&self) -> (u8, u8) {
        const MAJOR_SHIFT_RIGHT: u32 = IdHeader::VERSION_MAJOR_MASK.trailing_zeros();
        (
            (self.version & IdHeader::VERSION_MAJOR_MASK) >> MAJOR_SHIFT_RIGHT,
            self.version & IdHeader::VERSION_MINOR_MASK,
        )
    }

    /// Returns the output channel configuration.
    fn channels(&self) -> &ChannelMapping {
        &self.channels
    }

    /// Returns the number of samples (at 48 kHz) to discard when beginning playback.
    fn pre_skip(&self) -> u16 {
        self.pre_skip
    }

    /// Returns the encoding sample rate.
    fn sample_rate(&self) -> Option<NonZeroU32> {
        self.sample_rate
    }

    /// Returns 20*log_10 of the factor by which to scale the decoder output to
    /// receive the desired playback volume.
    fn output_gain(&self) -> i16 {
        self.output_gain
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
            // this is located here so that on comment parse failure, calling .next() again returns
            // the next comment
            self.pos = cmt_start + cmt_len;
            self.comments_read += 1;

            // parse comment
            let cmt = from_utf8(self.comments.get(cmt_start..self.pos)?).ok()?;
            let (name, value) = cmt.split_at(cmt.find('=')?);

            Some((name, &value[1..]))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // this has a lower-bound of zero as the next comment might be malformed
        (0, Some((self.comments_num - self.comments_read) as usize))
    }
}

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

/// An iterator over the Frames in an Ogg container.
#[derive(Debug)]
pub struct Frames<'a, R: Read + Seek> {
    reader: &'a mut OggOpusReader<R>,
    frames: VecDeque<Frame>,
}

impl<'a, R> Frames<'a, R>
where
    R: Read + Seek,
{
    fn new(reader: &'a mut OggOpusReader<R>) -> Self {
        Frames {
            reader,
            frames: VecDeque::new(),
        }
    }

    /// This overwrites anything currently in `frames`.
    fn read_packet(&mut self) -> crate::error::Result<Option<()>> {
        let ogg_packet = match self.reader.reader.read_packet()? {
            Some(ogg_packet) => ogg_packet.data,
            None => return Ok(None),
        };

        let streams = self.reader.id_header.channels().streams() as usize;
        self.frames = (0..streams)
            .scan(&ogg_packet[..], |data, i| {
                match Packet::new_with_framing(data, i != streams - 1) {
                    Ok((packet, new_data)) => {
                        *data = new_data;
                        Ok(packet)
                    }
                    Err(err) => Err(err),
                }
                .into()
            })
            .collect::<crate::packet::Result<Vec<_>>>()?
            .into_iter()
            .map(Packet::frames)
            .flatten()
            .collect();
        Ok(Some(()))
    }
}

impl<'a, R> Iterator for Frames<'a, R>
where
    R: Read + Seek,
{
    type Item = crate::error::Result<Frame>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(frame) = self.frames.pop_front() {
            Some(Ok(frame))
        } else if let Err(err) = self.read_packet().transpose()? {
            Some(Err(err))
        } else {
            self.next()
        }
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
    #[inline]
    pub fn comments(&self) -> Comments<'_> {
        self.comments.comments()
    }

    /// Returns an iterator over the contained audio frames.
    #[inline]
    pub fn frames(&mut self) -> Frames<'_, R> {
        Frames::new(self)
    }

    /// Returns the number of samples (at 48 kHz) to discard when beginning playback.
    #[inline]
    pub fn pre_skip(&self) -> u16 {
        self.id_header.pre_skip()
    }

    /// Returns the sample rate of the media this file was encoded from, in Hz.
    ///
    /// Note that this is not necessarily the sample rate it will be played back at.
    #[inline]
    pub fn sample_rate(&self) -> Option<NonZeroU32> {
        self.id_header.sample_rate()
    }

    /// Returns 20&thinsp;log<sub>10</sub> of the factor by which to scale the decoder output to
    /// receive the desired playback volume.
    #[inline]
    pub fn output_gain(&self) -> i16 {
        self.id_header.output_gain()
    }

    /// Returns the encoder vendor string from the Vorbis comment block.
    #[inline]
    pub fn vendor(&self) -> &str {
        self.comments.vendor()
    }

    /// Returns the encapsulation specification version as (major, minor).
    #[inline]
    pub fn version(&self) -> (u8, u8) {
        self.id_header.version()
    }

    /// Returns the wrapped reader, consuming the `OggOpusReader`.
    #[inline]
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
