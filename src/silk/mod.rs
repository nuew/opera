use crate::{
    ec::RangeDecoder,
    packet::{Bandwidth, Config, FrameSize},
};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    iter::FusedIterator,
};

mod frame;

use self::frame::{SilkFrame, StereoPredWeights};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum SilkError {
    InvalidFrameSize,
}

impl Display for SilkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl Error for SilkError {
    fn description(&self) -> &str {
        match self {
            SilkError::InvalidFrameSize => "invalid frame size",
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub(crate) enum Channel {
    Mid,
    Side,
}

impl Channel {
    fn new(_lp_header: LpHeader, channel_num: u8) -> Channel {
        if channel_num & 1 != 0 {
            Channel::Side
        } else {
            Channel::Mid
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct LpChannelHeader {
    vad: u8,
    lbrr: bool,
}

impl LpChannelHeader {
    fn from_stream(data: &mut RangeDecoder<'_>, frames: u8) -> LpChannelHeader {
        LpChannelHeader {
            vad: (0..frames).fold(0, |vad, i| {
                let frame_vad = data.decode_bit_logp(1).unwrap();
                vad | (u8::from(frame_vad) << i)
            }),
            lbrr: data.decode_bit_logp(1).unwrap(),
        }
    }

    fn vad(self, frame: u8) -> bool {
        self.vad & (1 << frame) != 0
    }

    fn lbrr(self) -> bool {
        self.lbrr
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct LbrrFrameHeader(u8);

impl LbrrFrameHeader {
    fn from_stream(
        data: &mut RangeDecoder<'_>,
        channel_header: Option<LpChannelHeader>,
        frame_size: FrameSize,
    ) -> Option<LbrrFrameHeader> {
        const ICDF_LBRR_FLAGS_2BIT: &[u8] = &[203, 150, 0];
        const ICDF_LBRR_FLAGS_3BIT: &[u8] = &[215, 195, 166, 125, 110, 82, 0];

        channel_header
            .filter(|channel_header| channel_header.lbrr())
            .and_then(|_| match frame_size {
                FrameSize::Fourty => data.decode_icdf(ICDF_LBRR_FLAGS_2BIT, 8),
                FrameSize::Sixty => data.decode_icdf(ICDF_LBRR_FLAGS_3BIT, 8),
                _ => None,
            })
            .map(|lbrr_sym| LbrrFrameHeader(lbrr_sym as u8 + 1))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct LpHeader {
    mid: LpChannelHeader,
    mid_lbrr: Option<LbrrFrameHeader>,
    side: Option<LpChannelHeader>,
    side_lbrr: Option<LbrrFrameHeader>,
}

impl LpHeader {
    fn from_stream(
        data: &mut RangeDecoder<'_>,
        frame_size: FrameSize,
        frames: u8,
        stereo: bool,
    ) -> LpHeader {
        let mid = LpChannelHeader::from_stream(data, frames);
        let side = if stereo {
            Some(LpChannelHeader::from_stream(data, frames))
        } else {
            None
        };

        LpHeader {
            mid,
            mid_lbrr: LbrrFrameHeader::from_stream(data, Some(mid), frame_size),
            side,
            side_lbrr: LbrrFrameHeader::from_stream(data, side, frame_size),
        }
    }

    fn channel(&self, channel: Channel) -> Option<LpChannelHeader> {
        match channel {
            Channel::Mid => Some(self.mid),
            Channel::Side => self.side,
        }
    }

    fn lbrr(&self, channel: Channel) -> Option<LbrrFrameHeader> {
        match channel {
            Channel::Mid => self.mid_lbrr,
            Channel::Side => self.side_lbrr,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct SilkPacket<'a, 'b> {
    bandwidth: Bandwidth,
    data: &'a mut RangeDecoder<'b>,
    lp_header: LpHeader,
    stereo: bool,

    frames: u8,
    cur_frame: u8,
    subframes: u8,
}

impl<'a, 'b> SilkPacket<'a, 'b> {
    fn from_stream(
        data: &'a mut RangeDecoder<'b>,
        config: Config,
        stereo: bool,
    ) -> Result<SilkPacket<'a, 'b>, SilkError> {
        let (frames, subframes) = match config.frame_size() {
            FrameSize::Ten => (1, 2),
            FrameSize::Twenty => (1, 4),
            FrameSize::Fourty => (2, 4),
            FrameSize::Sixty => (3, 4),
            _ => return Err(SilkError::InvalidFrameSize),
        };
        Ok(SilkPacket {
            bandwidth: match config.bandwidth() {
                Bandwidth::SuperWideband | Bandwidth::Fullband => Bandwidth::Wideband,
                other => other,
            },
            lp_header: LpHeader::from_stream(data, config.frame_size(), frames, stereo),
            data,
            stereo,
            frames,
            cur_frame: 0,
            subframes,
        })
    }
}

impl<'a, 'b> Iterator for SilkPacket<'a, 'b> {
    type Item = SilkFrame;

    fn next(&mut self) -> Option<Self::Item> {
        use self::frame::SilkFrameEnvironment;

        if self.cur_frame < self.frames {
            let channel = Channel::new(self.lp_header, self.cur_frame);
            // FIXME temporarily assume that LBRR frames don't exist
            let lbrr = false;

            let frame = SilkFrameEnvironment {
                channel,
                lbrr,
                stereo: self.stereo,
                vad: self.lp_header.channel(channel).unwrap().vad(self.cur_frame),
            }
            .frame_from_stream(self.data);

            self.cur_frame += 1;
            Some(frame)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unimplemented!()
    }
}

impl ExactSizeIterator for SilkPacket<'_, '_> {}
impl FusedIterator for SilkPacket<'_, '_> {}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SilkDecoder {
    stereo: bool,
    stereo_pred_weights: StereoPredWeights,
}

impl SilkDecoder {
    pub(crate) fn new(stereo: bool) -> SilkDecoder {
        SilkDecoder {
            stereo,
            stereo_pred_weights: StereoPredWeights::default(),
        }
    }

    pub(crate) fn decode(
        &mut self,
        data: &mut RangeDecoder<'_>,
        config: Config,
        stereo: bool,
    ) -> Result<(), SilkError> {
        let mut silk_packet = SilkPacket::from_stream(data, config, stereo)?;
        let frame0 = silk_packet.next().unwrap();

        println!("{:?}\n{:?}", silk_packet, frame0);
        Ok(())
    }
}
