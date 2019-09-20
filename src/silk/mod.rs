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
    vad: u8,
    lbrr: bool,
}

impl LpChannelHeader {
    fn new(data: &mut RangeDecoder<'_>, frames: u8) -> LpChannelHeader {
        // TODO potential optimization: use bits directly and advance the range decoder

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
    fn new(
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
    fn new(
        data: &mut RangeDecoder<'_>,
        frame_size: FrameSize,
        frames: u8,
        stereo: bool,
    ) -> LpHeader {
        let mid = LpChannelHeader::new(data, frames);
        let side = if stereo {
            Some(LpChannelHeader::new(data, frames))
        } else {
            None
        };

        LpHeader {
            mid,
            mid_lbrr: LbrrFrameHeader::new(data, Some(mid), frame_size),
            side,
            side_lbrr: LbrrFrameHeader::new(data, side, frame_size),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum SignalType {
    Inactive,
    Unvoiced,
    Voiced,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
enum QuantizationOffsetType {
    Low,
    High,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct SilkFrameHeader {
    signal_type: SignalType,
    quantization_offset_type: QuantizationOffsetType,
}

impl SilkFrameHeader {
    fn stereo_pred_weights(data: &mut RangeDecoder<'_>) {
        const ICDF_STEREO_PRED_WEIGHTS_STAGE_1: &[u8] = &[
            249, 247, 246, 245, 244, 234, 210, 202, 201, 200, 197, 174, 82, 59, 56, 55, 54, 46, 22,
            12, 11, 10, 9, 7, 0,
        ];
        const ICDF_STEREO_PRED_WEIGHTS_STAGE_2: &[u8] = &[171, 85, 0];
        const ICDF_STEREO_PRED_WEIGHTS_STAGE_3: &[u8] = &[205, 154, 102, 51, 0];
    }

    fn frame_type(data: &mut RangeDecoder<'_>, vad: bool) -> (SignalType, QuantizationOffsetType) {
        const ICDF_FRAME_TYPE_NO_VAD: &[u8] = &[230, 0];
        const ICDF_FRAME_TYPE_VAD: &[u8] = &[232, 158, 10, 0];

        if vad {
            match data.decode_icdf(ICDF_FRAME_TYPE_VAD, 6).unwrap() {
                0 => (SignalType::Unvoiced, QuantizationOffsetType::Low),
                1 => (SignalType::Unvoiced, QuantizationOffsetType::High),
                2 => (SignalType::Voiced, QuantizationOffsetType::Low),
                3 => (SignalType::Voiced, QuantizationOffsetType::High),
                _ => unreachable!(),
            }
        } else {
            (
                SignalType::Inactive,
                if data.decode_icdf(ICDF_FRAME_TYPE_NO_VAD, 6).unwrap() == 0 {
                    QuantizationOffsetType::Low
                } else {
                    QuantizationOffsetType::High
                },
            )
        }
    }

    fn new(data: &mut RangeDecoder<'_>, vad: bool) -> SilkFrameHeader {
        let (signal_type, quantization_offset_type) = SilkFrameHeader::frame_type(data, vad);

        SilkFrameHeader {
            signal_type,
            quantization_offset_type,
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
        let lp_header = LpHeader::new(data, config.frame_size(), frames, stereo);

        eprintln!("({}, {}); {:?}", frames, subframes, lp_header);

        // decode regular silk frames

        Ok(())
    }
}
