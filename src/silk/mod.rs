use crate::{
    ec::RangeDecoder,
    packet::{Config, FrameSize},
};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum SilkError {
    InvalidFrameSize,
}

impl Display for SilkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SilkError::InvalidFrameSize => "invalid frame size",
        })
    }
}

impl Error for SilkError {}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct LpChannelHeader {
    vad: [Option<bool>; 3],
    lbrr: bool,
}

impl LpChannelHeader {
    fn new(data: &mut RangeDecoder<'_>, frames: u8) -> LpChannelHeader {
        // TODO potential optimization: use bits directly and advance the range decoder
        let mut vad = [Option::default(); 3];

        for i in 0..frames {
            vad[usize::from(i)] = data.decode_bit_logp(1);
        }

        LpChannelHeader {
            vad,
            lbrr: data.decode_bit_logp(1).unwrap(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct LpHeader {
    mid: LpChannelHeader,
    side: Option<LpChannelHeader>,
}

impl LpHeader {
    fn new(data: &mut RangeDecoder<'_>, frames: u8, stereo: bool) -> LpHeader {
        LpHeader {
            mid: LpChannelHeader::new(data, frames),
            side: if stereo {
                Some(LpChannelHeader::new(data, frames))
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SilkDecoder;

impl SilkDecoder {
    pub(crate) fn decode(
        &mut self,
        data: &mut RangeDecoder<'_>,
        config: Config,
        stereo: bool,
    ) -> Result<(), SilkError> {
        let (frames, subframes) = match config.frame_size() {
            FrameSize::Ten => (1, 2),
            FrameSize::Twenty => (1, 4),
            FrameSize::Fourty => (2, 4),
            FrameSize::Sixty => (3, 4),
            _ => return Err(SilkError::InvalidFrameSize),
        };
        let lp_header = LpHeader::new(data, frames, stereo);

        eprintln!("({}, {}); {:?}", frames, subframes, lp_header);

        Ok(())
    }
}
