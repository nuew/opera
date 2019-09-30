use crate::{
    ec::RangeDecoder,
    silk::{Channel, LpHeader},
};

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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
pub(super) struct StereoPredWeights {
    w0_q13: i16,
    w1_q13: i16,
}

impl StereoPredWeights {
    fn from_stream(data: &mut RangeDecoder<'_>) -> StereoPredWeights {
        const STEREO_PRED_WEIGHTS_Q13: &[i16] = &[
            -13732, -10050, -8266, -7526, -6500, -5000, -2950, -820, 820, 2950, 5000, 6500, 7526,
            8266, 10050, 13732,
        ];
        const ICDF_STEREO_PRED_WEIGHTS_STAGE_1: &[u8] = &[
            249, 247, 246, 245, 244, 234, 210, 202, 201, 200, 197, 174, 82, 59, 56, 55, 54, 46, 22,
            12, 11, 10, 9, 7, 0,
        ];
        const ICDF_STEREO_PRED_WEIGHTS_STAGE_2: &[u8] = &[171, 85, 0];
        const ICDF_STEREO_PRED_WEIGHTS_STAGE_3: &[u8] = &[205, 154, 102, 51, 0];

        // decode (n, (i0, i1, i2, i3))
        let n = data
            .decode_icdf(ICDF_STEREO_PRED_WEIGHTS_STAGE_1, 8)
            .unwrap();
        let i = {
            let mut dec_2in = || {
                Some((
                    data.decode_icdf(ICDF_STEREO_PRED_WEIGHTS_STAGE_2, 8)?,
                    data.decode_icdf(ICDF_STEREO_PRED_WEIGHTS_STAGE_3, 8)?,
                ))
            };
            let (i0, i1) = dec_2in().unwrap();
            let (i2, i3) = dec_2in().unwrap();
            (i0, i1, i2, i3)
        };

        let wn_q13 = |win, i_n| {
            let low = STEREO_PRED_WEIGHTS_Q13[win];
            let step = i32::from(STEREO_PRED_WEIGHTS_Q13[win + 1] - low);
            low + ((step * 6554) >> 16) as i16 * (2 * i_n + 1)
        };
        let w1_q13 = wn_q13(i.2 + 3 * (n % 5), i.3 as i16);
        StereoPredWeights {
            w0_q13: wn_q13(i.0 + 3 * (n / 5), i.1 as i16) - w1_q13,
            w1_q13,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub(super) struct SubframeGains;

impl SubframeGains {
    fn from_stream(data: &mut RangeDecoder<'_>, signal_type: SignalType) -> SubframeGains {
        const ICDF_SUBFR_INDEPENDENT_INACTIVE: &[u8] = &[224, 112, 44, 15, 3, 2, 1, 0];
        const ICDF_SUBFR_INDEPENDENT_UNVOICED: &[u8] = &[254, 237, 192, 132, 70, 23, 4, 0];
        const ICDF_SUBFR_INDEPENDENT_VOICED: &[u8] = &[255, 252, 226, 155, 61, 11, 2, 0];
        const ICDF_SUBFR_INDEPENDENT_COMMON: &[u8] = &[224, 192, 160, 128, 96, 64, 32, 0];

        unimplemented!()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub(super) struct SilkFrame {
    stereo_pred_weights: Option<StereoPredWeights>,
    mid_only: Option<bool>,
    signal_type: SignalType,
    quantization_offset_type: QuantizationOffsetType,
}

impl SilkFrame {
    pub(super) fn from_stream(
        data: &mut RangeDecoder<'_>,
        env: SilkFrameEnvironment<'_>,
    ) -> SilkFrame {
        let stereo_mid = |data: &mut RangeDecoder<'_>| {
            const ICDF_MID_ONLY: &[u8] = &[64, 0];

            if env.stereo && env.channel == Channel::Mid {
                let stereo_pred_weights = StereoPredWeights::from_stream(data);

                // decode mid-only flag
                let active_coded = if !env.lbrr {
                    env.lp_header
                        .channel(Channel::Side)
                        .unwrap()
                        .vad(env.frame_no)
                } else {
                    env.lp_header
                        .lbrr(Channel::Side)
                        .map(|lbrr_flags| lbrr_flags.lbrr(env.frame_no))
                        .unwrap_or(true)
                };
                let mid_only = if !active_coded {
                    Some(data.decode_icdf(ICDF_MID_ONLY, 8).unwrap() != 0)
                } else {
                    None
                };

                (Some(stereo_pred_weights), mid_only)
            } else {
                Default::default()
            }
        };

        let frame_type = |data: &mut RangeDecoder<'_>, vad| {
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
        };

        let channel = env.lp_header.channel(env.channel).unwrap();
        let vad = channel.vad(env.frame_no);

        let (stereo_pred_weights, mid_only) = stereo_mid(data);
        let (signal_type, quantization_offset_type) = frame_type(data, vad);

        SilkFrame {
            stereo_pred_weights,
            mid_only,
            signal_type,
            quantization_offset_type,
        }
    }

    pub(super) fn mid_only(&self) -> Option<bool> {
        self.mid_only
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub(super) struct SilkFrameEnvironment<'a> {
    pub(super) channel: Channel,
    pub(super) frame_no: u8,
    pub(super) lbrr: bool,
    pub(super) lp_header: &'a LpHeader,
    pub(super) stereo: bool,
}
