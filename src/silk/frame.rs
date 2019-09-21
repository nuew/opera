use crate::ec::RangeDecoder;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
pub(crate) struct StereoPredWeights {
    w0_q13: usize,
    w1_q13: usize,
}

impl StereoPredWeights {
    const STEREO_PRED_WEIGHTS_Q13: &'static [i16] = &[
        -13732, -10050, -8266, -7526, -6500, -5000, -2950, -820, 820, 2950, 5000, 6500, 7526, 8266,
        10050, 13732,
    ];

    fn from_stream(data: &mut RangeDecoder<'_>) -> StereoPredWeights {
        const ICDF_STEREO_PRED_WEIGHTS_STAGE_1: &[u8] = &[
            249, 247, 246, 245, 244, 234, 210, 202, 201, 200, 197, 174, 82, 59, 56, 55, 54, 46, 22,
            12, 11, 10, 9, 7, 0,
        ];
        const ICDF_STEREO_PRED_WEIGHTS_STAGE_2: &[u8] = &[171, 85, 0];
        const ICDF_STEREO_PRED_WEIGHTS_STAGE_3: &[u8] = &[205, 154, 102, 51, 0];

        unimplemented!()
    }

    fn w0(self) -> i16 {
        StereoPredWeights::STEREO_PRED_WEIGHTS_Q13[self.w0_q13]
    }

    fn w1(self) -> i16 {
        StereoPredWeights::STEREO_PRED_WEIGHTS_Q13[self.w1_q13]
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
    stereo_pred_weights: Option<StereoPredWeights>,
    signal_type: SignalType,
    quantization_offset_type: QuantizationOffsetType,
}

impl SilkFrameHeader {
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

    fn from_stream(data: &mut RangeDecoder<'_>, stereo: bool, vad: bool) -> SilkFrameHeader {
        // TODO and this is the middle channel
        let stereo_pred_weights = if stereo {
            Some(StereoPredWeights::from_stream(data))
        } else {
            None
        };
        let (signal_type, quantization_offset_type) = SilkFrameHeader::frame_type(data, vad);

        SilkFrameHeader {
            stereo_pred_weights,
            signal_type,
            quantization_offset_type,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub(crate) struct SilkFrame {
    header: SilkFrameHeader,
}

impl SilkFrame {
    pub(crate) fn from_stream(data: &mut RangeDecoder<'_>, stereo: bool, vad: bool) -> SilkFrame {
        SilkFrame {
            header: SilkFrameHeader::from_stream(data, stereo, vad),
        }
    }
}
