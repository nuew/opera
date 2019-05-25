//! Audio channel control and mapping.
use crate::slice_ext::{BoundsError, SliceExt};
use std::{
    convert::TryFrom,
    error::Error,
    fmt::{self, Display, Formatter},
};

/// The error type returned when a channel layout is malformed.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum ChannelLayoutError {
    /// Has the same meaning as [`std::io::ErrorKind::UnexpectedEof`].
    ///
    /// [`std::io::ErrorKind::UnexpectedEof`]: https://doc.rust-lang.org/nightly/std/io/enum.ErrorKind.html#variant.UnexpectedEof
    UnexpectedEof,
    /// The specified family is not of a known type.
    UnknownFamily,
    /// The specified channel layout family and the number of channels requested are incompatiable.
    BadChannelsForFamily,
    /// There are either zero streams, too many streams, or the number of coupled streams exceeds
    /// the total number of streams.
    IllegalStreams,
}

impl Display for ChannelLayoutError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ChannelLayoutError::UnexpectedEof => "mapping table ended early",
            ChannelLayoutError::UnknownFamily => "unknown channel layout family",
            ChannelLayoutError::BadChannelsForFamily => {
                "invalid number of channels for the specified family"
            }
            ChannelLayoutError::IllegalStreams => "illegal stream specification",
        })
    }
}

impl Error for ChannelLayoutError {}

impl From<BoundsError> for ChannelLayoutError {
    fn from(_from: BoundsError) -> ChannelLayoutError {
        ChannelLayoutError::UnexpectedEof
    }
}

/// RTP-style channel mapping layouts.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum RtpChannelLayout {
    /// monomorphic
    Mono = 1,
    /// stereo (left, right)
    Stereo = 2,
}

impl TryFrom<u8> for RtpChannelLayout {
    type Error = ChannelLayoutError;

    fn try_from(v: u8) -> Result<Self, ChannelLayoutError> {
        match v {
            1 => Ok(RtpChannelLayout::Mono),
            2 => Ok(RtpChannelLayout::Stereo),
            _ => Err(ChannelLayoutError::BadChannelsForFamily),
        }
    }
}

impl seal::Sealed for RtpChannelLayout {}

impl MappingTable for RtpChannelLayout {
    fn streams(&self) -> u8 {
        1
    }

    fn coupled(&self) -> u8 {
        *self as u8 - 1
    }
}

/// Vorbis-style channel mapping layouts.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub(crate) enum VorbisChannelLayout {
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
    type Error = ChannelLayoutError;

    fn try_from(v: u8) -> Result<Self, ChannelLayoutError> {
        match v {
            1 => Ok(VorbisChannelLayout::Mono),
            2 => Ok(VorbisChannelLayout::Stereo),
            3 => Ok(VorbisChannelLayout::LinearSurround),
            4 => Ok(VorbisChannelLayout::Quadraphonic),
            5 => Ok(VorbisChannelLayout::FivePointZero),
            6 => Ok(VorbisChannelLayout::FivePointOne),
            7 => Ok(VorbisChannelLayout::SixPointOne),
            8 => Ok(VorbisChannelLayout::SevenPointOne),
            _ => Err(ChannelLayoutError::BadChannelsForFamily),
        }
    }
}

/// Ambisonics channel mapping layouts.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub(crate) enum AmbisonicsChannelLayout {
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
    type Error = ChannelLayoutError;

    fn try_from(v: u8) -> Result<Self, ChannelLayoutError> {
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
            _ => Err(ChannelLayoutError::BadChannelsForFamily),
        }
    }
}

mod seal {
    pub trait Sealed {}
}

pub trait MappingTable: seal::Sealed {
    /// Returns the number of streams encoded in each Ogg packet.
    fn streams(&self) -> u8;

    /// Returns the number of streams whose decoders are to be configured to produce two channels
    /// (stereo sound). Will not be larger then the return value of [`MappingTable::streams`].
    ///
    /// [`MappingTable::streams`]: #method.streams
    fn coupled(&self) -> u8;
}

/// Channel Mapping table as defined in RFC 7845
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct StandardMappingTable {
    /// The number of streams encoded in each Ogg packet.
    streams: u8,
    /// The number of streams whose decoders are to be configured to produce two channels
    /// (stereo sound). Must not be larger then `streams`.
    coupled: u8,
    /// A mapping of decoded channels to output channels.
    mapping: Vec<u8>,
}

impl StandardMappingTable {
    pub fn new(channels: u8, table: &[u8]) -> Result<Self, ChannelLayoutError> {
        let streams = *table.get_res(0)?;
        let coupled = *table.get_res(1)?;

        // check for invalid mappings
        if streams == 0 || streams < coupled || usize::from(streams) + usize::from(coupled) > 255 {
            return Err(ChannelLayoutError::IllegalStreams);
        }

        Ok(StandardMappingTable {
            streams,
            coupled,
            mapping: table.get_res(2..2 + usize::from(channels))?.to_owned(),
        })
    }
}

impl seal::Sealed for StandardMappingTable {}

impl MappingTable for StandardMappingTable {
    fn streams(&self) -> u8 {
        self.streams
    }

    fn coupled(&self) -> u8 {
        self.coupled
    }
}

/// Ambisonics channel mapping table (for mapping type 3)
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct AmbisonicsMappingTable {
    /// The number of streams encoded in each Ogg packet.
    streams: u8,
    /// The number of streams whose decoders are to be configured to produce two channels
    /// (stereo sound). Must not be larger then `streams`.
    coupled: u8,
    /// The demixing matrix
    matrix: Vec<u16>,
}

impl AmbisonicsMappingTable {
    pub fn new(channels: u8, table: &[u8]) -> Result<Self, ChannelLayoutError> {
        use byteorder::{ByteOrder, LE};

        let streams = *table.get_res(0)?;
        let coupled = *table.get_res(1)?;

        // check for invalid mappings
        if streams == 0 || streams < coupled || usize::from(streams) + usize::from(coupled) > 255 {
            return Err(ChannelLayoutError::IllegalStreams);
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
}

impl seal::Sealed for AmbisonicsMappingTable {}

impl MappingTable for AmbisonicsMappingTable {
    fn streams(&self) -> u8 {
        self.streams
    }

    fn coupled(&self) -> u8 {
        self.coupled
    }
}

/// The channel mapping family and channel layout for an Ogg Opus stream.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub(crate) enum ChannelMapping {
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
    pub(crate) fn new(channels: u8, family: u8, table: &[u8]) -> Result<Self, ChannelLayoutError> {
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
            _ => Err(ChannelLayoutError::UnknownFamily),
        }
    }

    pub(crate) fn mapping_table(&self) -> &dyn MappingTable {
        match self {
            ChannelMapping::RTP(ref layout) => layout,
            ChannelMapping::Vorbis { ref mapping, .. } => mapping,
            ChannelMapping::AmbisonicsIndividual { ref mapping, .. } => mapping,
            ChannelMapping::AmbisonicsDemixed { ref mapping, .. } => mapping,
            ChannelMapping::Discrete { ref mapping, .. } => mapping,
        }
    }
}
