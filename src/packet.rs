//! Decodes Opus codec packets into sequences of frames.

use crate::{
    channel::MappingTable,
    error::{Error, Result},
    slice_ext::SliceExt,
};
use std::{
    error,
    fmt::{self, Debug, Display, Formatter},
    time::Duration,
};

/// A packet's Config Number, from [RFC 6716 § 3.1]
///
/// [RFC 6716 § 3.1]: https://tools.ietf.org/html/rfc6716#section-3.1
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
struct ConfigNumber(u8);

impl ConfigNumber {
    fn new(config: u8) -> Option<ConfigNumber> {
        use std::u8::MAX;

        match config {
            0..=31 => Some(ConfigNumber(config)),
            32..=MAX => None,
        }
    }
}

impl Debug for ConfigNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?})", self.0, Config::from(*self))
    }
}

impl From<ConfigNumber> for u8 {
    fn from(from: ConfigNumber) -> u8 {
        from.0
    }
}

impl From<u8> for ConfigNumber {
    fn from(from: u8) -> ConfigNumber {
        const CONFIG_SHIFT: u32 = TableOfContents::MASK_CONFIG.trailing_zeros();
        ConfigNumber::new((from & TableOfContents::MASK_CONFIG) >> CONFIG_SHIFT).unwrap()
    }
}

impl From<TableOfContents> for ConfigNumber {
    fn from(from: TableOfContents) -> ConfigNumber {
        from.0.into()
    }
}

/// The codec(s) of each frame within a specific packet.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum Mode {
    /// Uses only the SILK codec
    Silk,
    /// Uses both the SILK and CELT codecs
    Hybrid,
    /// Uses only the CELT codec
    Celt,
}

impl From<ConfigNumber> for Mode {
    fn from(config: ConfigNumber) -> Mode {
        use std::u8::MAX;

        // See Table 2 of RFC 6716
        match config.into() {
            0..=11 => Mode::Silk,
            12..=15 => Mode::Hybrid,
            16..=31 => Mode::Celt,
            32..=MAX => unreachable!(),
        }
    }
}

impl Default for Mode {
    fn default() -> Mode {
        TableOfContents::default().mode()
    }
}

/// The bandwidth of each frame within a specific packet.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum Bandwidth {
    /// 4 kHz audio bandwidth, with an effective sample rate of 8 kHz
    Narrowband,
    /// 6 kHz audio bandwidth, with an effective sample rate of 12 kHz
    MediumBand,
    /// 8 kHz audio bandwidth, with an effective sample rate of 16 kHz
    Wideband,
    /// 12 kHz audio bandwidth, with an effective sample rate of 24 kHz
    SuperWideband,
    /// 20 kHz audio bandwidth, with an effective sample rate of 48 kHz
    Fullband,
}

impl From<ConfigNumber> for Bandwidth {
    fn from(config: ConfigNumber) -> Bandwidth {
        use std::u8::MAX;

        // See Table 2 of RFC 6716
        match config.into() {
            0..=3 | 16..=19 => Bandwidth::Narrowband,
            4..=7 => Bandwidth::MediumBand,
            8..=11 | 20..=23 => Bandwidth::Wideband,
            12 | 13 | 24..=27 => Bandwidth::SuperWideband,
            14 | 15 | 28..=31 => Bandwidth::Fullband,
            32..=MAX => unreachable!(),
        }
    }
}

impl Default for Bandwidth {
    fn default() -> Bandwidth {
        TableOfContents::default().bandwidth()
    }
}

/// The duration of frames within a specific packet.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum FrameSize {
    /// 2.5 ms
    TwoPointFive,
    /// 5 ms
    Five,
    /// 10 ms
    Ten,
    /// 20 ms
    Twenty,
    /// 40 ms
    Fourty,
    /// 60 ms
    Sixty,
}

impl FrameSize {
    /// Returns the corresponding number of microseconds as an integer.
    fn as_microseconds(self) -> u16 {
        match self {
            FrameSize::TwoPointFive => 2_500,
            FrameSize::Five => 5_000,
            FrameSize::Ten => 10_000,
            FrameSize::Twenty => 20_000,
            FrameSize::Fourty => 40_000,
            FrameSize::Sixty => 60_000,
        }
    }
}

impl Debug for FrameSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ms",
            match self {
                FrameSize::TwoPointFive => "2.5",
                FrameSize::Five => "5",
                FrameSize::Ten => "10",
                FrameSize::Twenty => "20",
                FrameSize::Fourty => "40",
                FrameSize::Sixty => "60",
            }
        )
    }
}

impl From<ConfigNumber> for FrameSize {
    fn from(config: ConfigNumber) -> FrameSize {
        use std::u8::MAX;

        // See Table 2 of RFC 6716
        match config.into() {
            16 | 20 | 24 | 28 => FrameSize::TwoPointFive,
            17 | 21 | 25 | 29 => FrameSize::Five,
            0 | 4 | 8 | 12 | 14 | 18 | 22 | 26 | 30 => FrameSize::Ten,
            1 | 5 | 9 | 13 | 15 | 19 | 23 | 27 | 31 => FrameSize::Twenty,
            2 | 6 | 10 => FrameSize::Fourty,
            3 | 7 | 11 => FrameSize::Sixty,
            32..=MAX => unreachable!(),
        }
    }
}

impl From<FrameSize> for Duration {
    fn from(from: FrameSize) -> Duration {
        Duration::from_micros(u64::from(from.as_microseconds()))
    }
}

impl Default for FrameSize {
    fn default() -> FrameSize {
        TableOfContents::default().frame_size()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
struct Config {
    mode: Mode,
    bandwidth: Bandwidth,
    frame_size: FrameSize,
}

impl From<ConfigNumber> for Config {
    fn from(config: ConfigNumber) -> Config {
        Config {
            mode: config.into(),
            bandwidth: config.into(),
            frame_size: config.into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
enum FramesLayout {
    /// 1 frame.
    One,
    /// 2 frames, each with an equal compressed size.
    TwoEqual,
    /// 2 frames, with different compressed sizes.
    TwoDifferent,
    /// An arbitrary number of frames.
    Arbitrary,
}

impl FramesLayout {
    fn new(c: u8) -> Option<FramesLayout> {
        use std::u8::MAX;

        // See Page 15 of RFC 6716
        match c {
            0 => Some(FramesLayout::One),
            1 => Some(FramesLayout::TwoEqual),
            2 => Some(FramesLayout::TwoDifferent),
            3 => Some(FramesLayout::Arbitrary),
            4..=MAX => None,
        }
    }
}

impl Default for FramesLayout {
    fn default() -> FramesLayout {
        TableOfContents::default().frames_layout()
    }
}

/// The table-of-contents (TOC) byte of a packet.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
struct TableOfContents(u8);

impl TableOfContents {
    /// A mask over the `config` field.
    const MASK_CONFIG: u8 = 0b1111_1000;

    /// A mask over the `s` field.
    const MASK_S: u8 = 0b0000_0100;

    /// A mask over the `c` field.
    const MASK_C: u8 = 0b0000_0011;

    /// Returns the overall configuration specified.
    fn config(self) -> Config {
        ConfigNumber::from(self).into()
    }

    /// Returns the specified codec ("mode").
    fn mode(self) -> Mode {
        ConfigNumber::from(self).into()
    }

    /// Returns the specified bandwidth.
    fn bandwidth(self) -> Bandwidth {
        ConfigNumber::from(self).into()
    }

    /// Returns the specified frame size.
    fn frame_size(self) -> FrameSize {
        ConfigNumber::from(self).into()
    }

    fn stereo(self) -> bool {
        (self.0 & TableOfContents::MASK_S) != 0
    }

    fn frames_layout(self) -> FramesLayout {
        FramesLayout::new(self.0 & TableOfContents::MASK_C).unwrap()
    }
}

impl Debug for TableOfContents {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableOfContents")
            .field("config", &self.config())
            .field("s", &self.stereo())
            .field("c", &self.frames_layout())
            .finish()
    }
}

impl From<u8> for TableOfContents {
    fn from(from: u8) -> TableOfContents {
        TableOfContents(from)
    }
}

impl From<TableOfContents> for u8 {
    fn from(from: TableOfContents) -> u8 {
        from.0
    }
}

/// The frame count byte of a code 3 packet.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
struct FrameCount(u8);

impl FrameCount {
    /// A mask over the `v` field.
    const MASK_V: u8 = 0b1000_0000;

    /// A mask over the `p` field.
    const MASK_P: u8 = 0b0100_0000;

    /// A mask over the `M` field.
    const MASK_M: u8 = 0b0011_1111;

    /// Returns weather this packet is VBR (`true`) or CBR (`false`).
    fn vbr(self) -> bool {
        (self.0 & FrameCount::MASK_V) != 0
    }

    /// Returns weather or not this packet contains Opus padding.
    fn padding(self) -> bool {
        (self.0 & FrameCount::MASK_P) != 0
    }

    /// Returns the number of frames in this packet.
    fn frame_count(self) -> u8 {
        self.0 & FrameCount::MASK_M
    }
}

impl Debug for FrameCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameCount").finish()
    }
}

impl From<u8> for FrameCount {
    fn from(from: u8) -> FrameCount {
        FrameCount(from)
    }
}

impl From<FrameCount> for u8 {
    fn from(from: FrameCount) -> u8 {
        from.0
    }
}

/// The error type returned when a packet is malformed.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum MalformedPacketError {
    /// A contained frame is longer than the limit of 1275 bytes ([RFC 6716 § 3.4:R2])
    ///
    /// [RFC 6716 § 3.4:R2]: https://tools.ietf.org/html/rfc6716#ref-R2
    OverlongFrame,
    /// The packet has an invalid payload length for its contents, such that the length of its
    /// frames cannot be inferred. ([RFC 6716 § 3.4:R3]/[RFC 6716 § 3.4:R6])
    ///
    /// [RFC 6716 § 3.4:R3]: https://tools.ietf.org/html/rfc6716#ref-R3
    /// [RFC 6716 § 3.4:R6]: https://tools.ietf.org/html/rfc6716#ref-R6
    UnevenFrameLengths,
    /// The packet's first frame purports to be longer than the packet itself.
    /// ([RFC 6716 § 3.4:R4])
    ///
    /// [RFC 6716 § 3.4:R4]: https://tools.ietf.org/html/rfc6716#ref-R4
    FrameOverflow,
    /// The packet contained zero frames. ([RFC 6716 § 3.4:R5] clause 1)
    ///
    /// [RFC 6716 § 3.4:R5]: https://tools.ietf.org/html/rfc6716#ref-R5
    ZeroFrames,
    /// The audio duration within this packet exceeded 120 ms. ([RFC 6716 § 3.4:R5] clause 2)
    ///
    /// [RFC 6716 § 3.4:R5]: https://tools.ietf.org/html/rfc6716#ref-R5
    OverlongDuration,
}

impl Display for MalformedPacketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MalformedPacketError::OverlongFrame => "contained frame exceeds 1275 byte limit",
            MalformedPacketError::UnevenFrameLengths => "packet has invalid payload length",
            MalformedPacketError::FrameOverflow => "contained frame longer than packet itself",
            MalformedPacketError::ZeroFrames => "contained zero frames",
            MalformedPacketError::OverlongDuration => "frames totaled longer than 120 ms",
        })
    }
}

impl error::Error for MalformedPacketError {}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Frame {
    config: Config,
    stereo: bool,
    data: Vec<u8>,
}

impl Frame {
    /// The maximum implicit length of a frame, in bytes, according to RFC 6716 § 3.4:R2
    const IMPLICIT_LEN_MAX: usize = 1275;

    fn new(config: Config, stereo: bool, data: &[u8]) -> Frame {
        Frame {
            config,
            stereo,
            data: data.to_owned(),
        }
    }
}

/// An Opus codec packet—that is, a group of [`Frame`]s with a shared configuration.
///
/// [`Frame`]: struct.Frame.html
#[derive(Debug, Clone)]
pub struct Packet {
    /// Decoder configuration necessary for each Frame.
    config: Config,
    /// Are the frames encoded as stereo (true) or mono (false)
    stereo: bool,
    /// Slices of each individual frame.
    frames: Vec<Vec<u8>>,
}

type DecodeFunction = fn(Config, bool, bool, &[u8]) -> Result<(Packet, &[u8])>;

impl Packet {
    /// The maximum allowable duration of a packet in microseconds.
    const DURATION_MAX: u32 = 120_000;

    /// Returns the length code of the packet and the offset to add.
    fn length_code(data: &[u8]) -> Result<(usize, usize)> {
        // RFC 6716 § 3.2.1
        match data.first() {
            Some(len @ 0..=251) => Ok((usize::from(*len), 1)),
            Some(first @ 252..=255) => {
                let second = data.get_res(1)?;
                Ok(((usize::from(*second) * 4) + usize::from(*first), 2))
            }
            None => Err(Error::UnexpectedEof),
        }
    }

    /// Returns data necessary for self-delimiting framing, or the default data if not using
    /// self-delimiting framing.
    fn framing<T>(data: &[u8], self_delimited: bool, implicit: T) -> Result<(usize, usize, &[u8])>
    where
        T: Fn(usize) -> Option<usize>,
    {
        if self_delimited {
            let (len, offset) = Packet::length_code(data)?;
            let data = &data.get_res(offset + len..)?;
            Ok((len, offset, data))
        } else {
            let len = implicit(data.len()).ok_or(MalformedPacketError::UnevenFrameLengths)?;
            if len <= Frame::IMPLICIT_LEN_MAX {
                Ok((len, 0, data))
            } else {
                Err(MalformedPacketError::OverlongFrame.into())
            }
        }
    }

    /// Returns the length of the padding bytes at the end of the current packet, based on the
    /// padding size bytes. Also returns where to continue reading from after the padding
    /// size bytes.
    fn padding(data: &[u8]) -> Result<(usize, &[u8])> {
        let mut padding = 0;
        let mut offset = 0;

        while let Some(byte) = data.get(offset) {
            use std::u8::MAX;

            match *byte {
                MAX => padding += 254,
                fin => return Ok((padding + usize::from(fin), &data[offset + 1..])),
            };

            offset += 1;
        }

        Err(Error::UnexpectedEof)
    }

    /// Decodes the body of a code 0 packet.
    fn decode_code_0(
        config: Config,
        stereo: bool,
        self_delimiting: bool,
        data: &[u8],
    ) -> Result<(Packet, &[u8])> {
        let (len, offset, more_data) = Packet::framing(data, self_delimiting, Some)?;
        Ok((
            Packet {
                config,
                stereo,
                frames: vec![data.get_res(offset..offset + len)?.to_owned()],
            },
            more_data,
        ))
    }

    /// Decodes the body of a code 1 packet.
    fn decode_code_1(
        config: Config,
        stereo: bool,
        self_delimiting: bool,
        data: &[u8],
    ) -> Result<(Packet, &[u8])> {
        let (len, offset, more_data) = Packet::framing(data, self_delimiting, |len| {
            if len & 1 == 0 {
                Some(len / 2)
            } else {
                None
            }
        })?;
        let data = &data[offset..];

        Ok((
            Packet {
                config,
                stereo,
                frames: vec![
                    data.get_res(..len)?.to_owned(),
                    data.get_res(len..len * 2)?.to_owned(),
                ],
            },
            more_data,
        ))
    }

    /// Decodes the body of a code 2 packet.
    fn decode_code_2(
        config: Config,
        stereo: bool,
        self_delimiting: bool,
        data: &[u8],
    ) -> Result<(Packet, &[u8])> {
        let (len1, offset1) = Packet::length_code(data)?;
        let (len2, offset2, more_data) =
            Packet::framing(&data[offset1..], self_delimiting, |len| Some(len - len1))?;
        let data = &data[offset1 + offset2..];

        if len1 <= data.len() {
            let end = len1 + len2;
            Ok((
                Packet {
                    config,
                    stereo,
                    frames: vec![
                        data.get_res(..len1)?.to_owned(),
                        data.get_res(len1..end)?.to_owned(),
                    ],
                },
                &more_data[end..],
            ))
        } else if self_delimiting {
            Err(Error::UnexpectedEof)
        } else {
            Err(MalformedPacketError::FrameOverflow.into())
        }
    }

    /// Decodes the body of a code 3 packet.
    fn decode_code_3(
        config: Config,
        stereo: bool,
        self_delimiting: bool,
        data: &[u8],
    ) -> Result<(Packet, &[u8])> {
        let fc = FrameCount::from(*data.first_res()?);

        // Handle R5 exclusions
        let frame_count = u32::from(fc.frame_count());
        let length_us = u32::from(config.frame_size.as_microseconds());
        if frame_count == 0 {
            return Err(MalformedPacketError::ZeroFrames.into());
        } else if frame_count * length_us > Packet::DURATION_MAX {
            return Err(MalformedPacketError::OverlongDuration.into());
        }

        // Handle padding
        let (padding, data) = if fc.padding() {
            Packet::padding(&data[1..])?
        } else {
            (0, &data[1..])
        };

        // Dispatch to either VBR or CBR parser
        let func = if fc.vbr() {
            Packet::decode_code_3_vbr
        } else {
            Packet::decode_code_3_cbr
        };
        func(config, stereo, self_delimiting, data, frame_count, padding)
    }

    /// Decodes the body of a code 3 VBR packet.
    fn decode_code_3_vbr(
        config: Config,
        stereo: bool,
        self_delimiting: bool,
        data: &[u8],
        frame_count: u32,
        padding: usize,
    ) -> Result<(Packet, &[u8])> {
        let mut offset = 0;

        Ok((
            Packet {
                config,
                stereo,
                frames: (0..frame_count)
                    .scan(0, |total_len, i| {
                        Some(if self_delimiting || i < frame_count - 1 {
                            match Packet::length_code(&data[offset..]) {
                                Ok((len, lc_size)) => {
                                    offset += lc_size;
                                    *total_len += len;
                                    Ok(len)
                                }
                                Err(err) => Err(err),
                            }
                        } else {
                            Ok(data.len() - *total_len - offset - padding)
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .map(|len| {
                        let new_offset = offset + len;
                        let data = data.get_res(offset..new_offset)?.to_owned();
                        offset = new_offset;
                        Ok(data)
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            &data.get_res(offset + padding..)?,
        ))
    }

    /// Decodes the body of a code 3 CBR packet.
    fn decode_code_3_cbr(
        config: Config,
        stereo: bool,
        self_delimiting: bool,
        data: &[u8],
        frame_count: u32,
        padding: usize,
    ) -> Result<(Packet, &[u8])> {
        let frame_count = frame_count as usize;
        let (len, offset) = if self_delimiting {
            Packet::length_code(data)?
        } else if data.len() % frame_count == 0 {
            (data.len() / frame_count, 0)
        } else {
            return Err(MalformedPacketError::UnevenFrameLengths.into());
        };

        let data = &data[offset..];
        Ok((
            Packet {
                config,
                stereo,
                frames: (0..frame_count)
                    .map(|i| Ok(data.get_res(len * i..len * (i + 1))?.to_owned()))
                    .collect::<Result<Vec<_>>>()?,
            },
            &data.get_res(len * frame_count + padding..)?,
        ))
    }

    /// Returns the decoding function corresponding to the specified frame layout.
    fn layout_function(frames_layout: FramesLayout) -> DecodeFunction {
        match frames_layout {
            FramesLayout::One => Packet::decode_code_0,
            FramesLayout::TwoEqual => Packet::decode_code_1,
            FramesLayout::TwoDifferent => Packet::decode_code_2,
            FramesLayout::Arbitrary => Packet::decode_code_3,
        }
    }

    /// Decodes an internally-framed packet from bytes.
    ///
    /// The length of `data` __must__ be exactly the length of the packet; otherwise, the packet
    /// may fail to decode, or worse, end in garbage.
    pub fn new(data: &[u8]) -> Result<Packet> {
        Self::new_with_framing(data, false).map(|(packet, _)| packet)
    }

    /// Decodes a potentially self-delimited packet from bytes.
    ///
    /// See [RFC 6716 Appendix B].
    ///
    /// [RFC 6716 Appendix B]: https://tools.ietf.org/html/rfc6716#appendix-B
    pub fn new_with_framing(data: &[u8], self_delimiting: bool) -> Result<(Packet, &[u8])> {
        let toc = TableOfContents::from(*data.first_res()?);
        Packet::layout_function(toc.frames_layout())(
            toc.config(),
            toc.stereo(),
            self_delimiting,
            &data[1..],
        )
    }

    pub fn frames(self) -> impl Iterator<Item = Frame> + ExactSizeIterator {
        let (config, stereo, frames) = (self.config, self.stereo, self.frames);
        frames
            .into_iter()
            .map(move |slice| Frame::new(config, stereo, &slice[..]))
    }
}

#[derive(Debug, Clone)]
pub struct Multistream {
    packets: Vec<Packet>,
}

impl Multistream {
    pub fn new<T>(data: &[u8], mapping_table: &T) -> Result<Multistream>
    where
        T: ?Sized + MappingTable,
    {
        let streams = mapping_table.streams();
        Ok(Multistream {
            packets: (0..streams)
                .scan(data, |data, i| {
                    match Packet::new_with_framing(data, i != streams - 1) {
                        Ok((packet, new_data)) => {
                            *data = new_data;
                            Ok(packet)
                        }
                        Err(err) => Err(err),
                    }
                    .into()
                })
                .collect::<Result<Vec<_>>>()?,
        })
    }

    pub fn frames(self) -> impl Iterator<Item = Frame> {
        self.packets.into_iter().rev().map(Packet::frames).flatten()
    }
}
