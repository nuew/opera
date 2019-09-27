//! An entropy decoder based on range coding.

// This is the part of this implementation most directly based on the reference implementation, and
// is more or less a direct port. I couldn't easily access the papers this was based on, and I
// (even now) don't really quite understand how this works. Regardless, it behaves identically to
// that of the reference implementation, so at least it works.
//
// Still, I'd appriciate it if somebody who better understood the theory behind this rewrote this
// with a more ideomatic API.

/// An entropy decoder based on range coding.
///
/// This is implemented as described in [RFC6716 § 4.1].
///
/// [RFC6716 § 4.1]: https://tools.ietf.org/html/rfc6716#section-4.1
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Default)]
pub(crate) struct RangeDecoder<'a> {
    data: &'a [u8],
    index_ec: usize,
    index_raw: usize,
    range: u32,
    renorm: bool,
    value: u32,
}

impl<'a> RangeDecoder<'a> {
    /// Returns a new range decoder operating on the provided data.
    pub(crate) fn new(data: &'a [u8]) -> RangeDecoder<'a> {
        let b0 = data.first().copied().unwrap_or(0);
        let mut ec_dec = RangeDecoder {
            data,
            index_ec: 1,
            index_raw: 0,
            range: 128,
            renorm: (b0 & 1) != 0,
            value: (127 - (b0 >> 1)).into(),
        };
        ec_dec.normalize();
        ec_dec
    }

    /// Common operations between `RangeDecoder::decode` and `RangeDecoder::decode_bin`.
    fn decode_inner(&self, ft: u16, dividend: u32) -> u16 {
        use std::convert::TryFrom;

        // Unfortunately there's no way to (without unsafe code) guarantee optimization based on
        // our knowledge that `dividend` can't be 0. Hopefully the optimizer notices :)
        //
        // Overflow should hopefully be impossible, but if it somehow happens regardless, return
        // 0 instead, as that *SHOULD* be the only possible result, as `∀ft.(vdd > u16::MAX →
        // ft < vdd), and `ft - ft = 0`.
        match u16::try_from(self.value / dividend) {
            Ok(vdd) => ft - u16::min(ft, vdd.saturating_add(1)), // fs
            Err(_) => 0,
        }
    }

    /// Compute `fs`, a value lying within the range of some symbol in the current context.
    ///
    /// Returns `None` if the provided `ft` is zero or too large for the current range.
    /// If `None` is returned, no change is made to the decoder state.
    pub(crate) fn decode(&self, ft: u16) -> Option<u16> {
        self.range
            .checked_div(ft.into())
            .filter(|dividend| *dividend != 0)
            .map(|dividend| self.decode_inner(ft, dividend))
    }

    /// Identical to `RangeDecoder::decode` where `ft = (1 << ftb)`, avoiding a division.
    ///
    /// Returns `None` if the provided `ftb` is too large for the current range. If `None` is
    /// returned, no change is made to the decoder state.
    pub(crate) fn decode_bin(&self, ftb: u8) -> Option<u16> {
        self.range
            .checked_shr(ftb.into())
            .filter(|dividend| *dividend != 0)
            .into_iter()
            .zip(1u16.checked_shl(ftb.into()))
            .map(|(dividend, ft)| self.decode_inner(ft, dividend))
            .next()
    }

    /// Decodes a single binary symbol.
    ///
    /// `logp` represents the absolute value of the base-2 logarithm of the probability of a
    /// `true` output (i.e., the probabilty of having a `true` is `1/(1 << logp)`).
    ///
    /// This is identical to calling `RangeDecoder::decode` where `ft = 1 << logp`, followed by
    /// `RangeDecoder::update` with the tuple `(0, (1 << logp) - 1, 1 << logp)` if the returned
    /// value was less than `(1 << logp) - 1` (i.e. `false` was returned), or with the tuple
    /// `((1 << logp) - 1, 1 << logp, 1 << logp)` otherwise (i.e. `true` was returned), but
    /// requires no multiplications or divisions.
    ///
    /// Calling `RangeDecoder::update` after this function is unnecessary.
    pub(crate) fn decode_bit_logp(&mut self, logp: u8) -> Option<bool> {
        self.range.checked_shr(logp.into()).map(|rshrlp| {
            let ret = self.value < rshrlp;
            if ret {
                self.range = rshrlp;
            } else {
                self.range -= rshrlp;
                self.value -= rshrlp;
            }

            self.normalize(); // renormalize the decoder state
            ret
        })
    }

    pub(crate) fn decode_icdf(&mut self, icdf: &[u8], ftb: u8) -> Option<usize> {
        self.range.checked_shr(ftb.into()).and_then(|rshrftb| {
            use std::mem::replace;

            let mut old_range;

            for i in 0..icdf.len() {
                // FIXME this might need to be `checked_mul`, but it's unclear if the panics when
                // that happens are due to me using the function wrong or wrapping is intended.
                //
                // Regardless, wrapping matches the reference implementation, and I don't
                // understand what this is doing enough to say either way.
                let rticdfv = rshrftb.wrapping_mul(u32::from(icdf[i]));
                old_range = replace(&mut self.range, rticdfv);

                if self.value >= self.range {
                    self.value -= self.range;
                    self.range = old_range - self.range;
                    self.normalize(); // renormalize the decoder state
                    return Some(i);
                }
            }

            None
        })
    }

    pub(crate) fn decode_raw(&mut self, _ft: u32) -> u32 {
        unimplemented!()
    }

    pub(crate) fn decode_raw_bits(&mut self, _bits: u8) -> u32 {
        unimplemented!()
    }

    /// Renormalizes `value` and `range` such that `range` lies entirely in the high-order symbol.
    fn normalize(&mut self) {
        const RANGE_MAX: u32 = 1 << 23;

        // RFC 6716 § 4.1.2.1
        while self.range <= RANGE_MAX {
            // get and mangle the next byte
            let bn = self.data.get(self.index_ec).copied().unwrap_or(0);
            let sym = (u8::from(self.renorm) << 7) | (bn >> 1);

            // update decoder state
            self.index_ec += 1;
            self.range <<= 8;
            self.renorm = (bn & 1) != 0;
            // This second subtraction may be replaced by (255 & !u32::from(sym)), which might(?)
            // be faster. It's what's used in the reference implementation, anyways.
            self.value = ((self.value << 8) + (255 - u32::from(sym))) & 0x7fffffff;
        }
    }

    /// Update `value` and `range` according to the decoded symbol.
    ///
    /// Additionally, this renormalizes the decoder.
    ///
    /// Step two of a standard symbol decode via `RangeDecoder::decode` or
    /// `RangeDecoder::decode_bin`.
    pub(crate) fn update(&mut self, fl: u16, fh: u16, ft: u16) {
        // TODO consider caching this, as in the reference implementation
        let dividend = self.range / u32::from(ft);

        // update decoder state
        let dmtsh = dividend * u32::from(ft - fh);
        self.value -= dmtsh;
        if fl > 0 {
            self.range = dividend * u32::from(fh - fl);
        } else {
            self.range -= dmtsh;
        }

        self.normalize(); // renormalize the decoder state
    }
}

#[cfg(test)]
mod tests {
    use super::RangeDecoder;
    use opus_rfc8251_sys::ec_dec;
    use rand::Rng;
    use std::{
        marker::PhantomData,
        ops::{Add, Rem},
    };

    const BUFFER_LEN: usize = 32;
    const DUMMY_DATA: &[u8] = &[0xFF, 0x7F, 0, 0x7F, 0xFF];
    const DUMMY_ICDF: &[u8] = &[1, 0];
    const ITERATIONS: usize = 48;

    #[derive(Debug, Clone, Copy)]
    struct EcDec<'a> {
        ec_dec: ec_dec,
        _marker: PhantomData<&'a [u8]>,
    }

    impl<'a> EcDec<'a> {
        fn new(data: &[u8]) -> EcDec<'_> {
            use opus_rfc8251_sys::ec_dec_init;
            use std::mem::MaybeUninit;

            let mut ec_dec = MaybeUninit::uninit();
            unsafe { ec_dec_init(ec_dec.as_mut_ptr(), data.as_ptr() as _, data.len() as _) };

            EcDec {
                ec_dec: unsafe { ec_dec.assume_init() },
                _marker: PhantomData,
            }
        }

        fn tell(&self) -> i32 {
            // FIXME this should be using ec_ilog
            self.ec_dec.nbits_total - (f64::from(self.ec_dec.rng).log10() as i32)
        }

        fn tell_frac(&mut self) -> u32 {
            #![allow(trivial_casts)]

            use opus_rfc8251_sys::ec_tell_frac;

            unsafe { ec_tell_frac(&self.ec_dec as *const _ as *mut _) }
        }

        fn decode(&mut self, ft: u16) -> u16 {
            use opus_rfc8251_sys::ec_decode;
            use std::convert::TryFrom;

            let fs = unsafe { ec_decode(&mut self.ec_dec, ft.into()) };
            u16::try_from(fs).expect("should be in range [fl,fh), both 16-bit values")
        }

        fn decode_bin(&mut self, bits: u8) -> u16 {
            use opus_rfc8251_sys::ec_decode_bin;
            use std::convert::TryFrom;

            let fs = unsafe { ec_decode_bin(&mut self.ec_dec, bits.into()) };
            u16::try_from(fs).expect("should be in range [fl,fh), both 16-bit values")
        }

        fn decode_bit_logp(&mut self, logp: u8) -> bool {
            use opus_rfc8251_sys::ec_dec_bit_logp;

            unsafe { ec_dec_bit_logp(&mut self.ec_dec, logp.into()) != 0 }
        }

        fn decode_icdf(&mut self, icdf: &[u8], ftb: u8) -> usize {
            use opus_rfc8251_sys::ec_dec_icdf;

            unsafe { ec_dec_icdf(&mut self.ec_dec, icdf.as_ptr(), ftb.into()) as usize }
        }

        fn decode_raw(&mut self, ft: u32) -> u32 {
            use opus_rfc8251_sys::ec_dec_uint;

            unsafe { ec_dec_uint(&mut self.ec_dec, ft) }
        }

        fn decode_raw_bits(&mut self, ftb: u8) -> u32 {
            use opus_rfc8251_sys::ec_dec_bits;

            unsafe { ec_dec_bits(&mut self.ec_dec, ftb.into()) }
        }

        fn update(&mut self, fl: u16, fh: u16, ft: u16) {
            use opus_rfc8251_sys::ec_dec_update;

            unsafe { ec_dec_update(&mut self.ec_dec, fl.into(), fh.into(), ft.into()) }
        }
    }

    fn logp1gen<T>(i: T) -> <<T as Rem<u8>>::Output as Add<u8>>::Output
    where
        T: Rem<u8>,
        T::Output: Add<u8>,
    {
        (i % 15) + 1
    }

    fn decode_generic<T>(buf: &[u8], mut get_ft: T)
    where
        T: FnMut(u16) -> u16,
    {
        // initialize decoders
        let mut ref_dec = EcDec::new(&buf);
        let mut opi_dec = RangeDecoder::new(&buf);

        for i in 0..ITERATIONS as u16 {
            let ft = get_ft(i);

            // attempt decodes
            let ref_res = ref_dec.decode(ft);
            let opi_res = opi_dec.decode(ft);

            assert_eq!(ref_res, opi_res.unwrap()); // test decodes

            // finalize decodes
            ref_dec.update(ref_res, ref_res + 1, ft);
            if let Some(our_res) = opi_res {
                opi_dec.update(our_res, our_res + 1, ft);
            }
        }
    }

    #[test]
    fn decode_random_bytes_randomly() {
        use rand::distributions::Uniform;

        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, u16::max_value());
        for _ in 0..ITERATIONS {
            let buf = rng.gen::<[u8; BUFFER_LEN]>();
            decode_generic(&buf, |_| rng.sample(ft_dist));
        }
    }

    #[test]
    fn decode_random_bytes_iteratively() {
        for _ in 0..ITERATIONS {
            let buf = rand::random::<[u8; BUFFER_LEN]>();
            decode_generic(&buf, |i| i + 1);
        }
    }

    #[test]
    fn decode_iterative_bytes_randomly() {
        use rand::distributions::Uniform;

        let buf: Vec<_> = (0..ITERATIONS as u8).collect();
        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, u16::max_value());

        for _ in 0..ITERATIONS {
            decode_generic(&buf[..], |_| rng.sample(ft_dist));
        }
    }

    #[test]
    fn decode_iterative_bytes_iteratively() {
        decode_generic(&[], |i| i + 1);
    }

    #[test]
    fn decode_empty_input() {
        decode_generic(&[], |i| i + 1);
    }

    #[test]
    fn decode_fail_zero_ft() {
        assert_eq!(RangeDecoder::new(DUMMY_DATA).decode(0), None);
    }

    fn decode_bin_generic<T>(buf: &[u8], mut get_ftb: T)
    where
        T: FnMut(u8) -> u8,
    {
        // initialize decoders
        let mut ref_dec = EcDec::new(&buf);
        let mut opi_dec = RangeDecoder::new(&buf);

        for i in 0..ITERATIONS as u8 {
            let ftb = get_ftb(i);

            // attempt decodes
            let ref_res = ref_dec.decode_bin(ftb);
            let opi_res = opi_dec.decode_bin(ftb);

            assert_eq!(ref_res, opi_res.unwrap()); // test decodes

            // finalize decodes
            ref_dec.update(ref_res, ref_res + 1, 1 << ftb);
            if let Some(our_res) = opi_res {
                opi_dec.update(our_res, our_res + 1, 1 << ftb);
            }
        }
    }

    #[test]
    fn decode_bin_random_bits_randomly() {
        use rand::distributions::Uniform;

        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, 16);
        for _ in 0..ITERATIONS {
            let buf = rng.gen::<[u8; BUFFER_LEN]>();
            decode_bin_generic(&buf, |_| rng.sample(ft_dist));
        }
    }

    #[test]
    fn decode_bin_random_bits_iteratively() {
        for _ in 0..ITERATIONS {
            let buf = rand::random::<[u8; BUFFER_LEN]>();
            decode_bin_generic(&buf, logp1gen);
        }
    }

    #[test]
    fn decode_bin_iterative_bits_randomly() {
        use rand::distributions::Uniform;

        let buf: Vec<_> = (0..ITERATIONS as u8).collect();
        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, 16);

        for _ in 0..ITERATIONS {
            decode_bin_generic(&buf[..], |_| rng.sample(ft_dist));
        }
    }

    #[test]
    fn decode_bin_iterative_bits_iteratively() {
        let buf: Vec<_> = (0..ITERATIONS as u8).collect();
        decode_bin_generic(&buf[..], logp1gen);
    }

    #[test]
    fn decode_bin_empty_input() {
        decode_bin_generic(&[], logp1gen);
    }

    #[test]
    fn decode_bin_fail_large_ftb() {
        let opi_dec = RangeDecoder::new(DUMMY_DATA);
        for i in 16..u8::max_value() {
            assert_eq!(opi_dec.decode_bin(i), None);
        }
    }

    fn decode_bit_logp_generic<T>(buf: &[u8], mut get_logp: T)
    where
        T: FnMut(u8) -> u8,
    {
        // initialize decoders
        let mut ref_dec = EcDec::new(&buf);
        let mut opi_dec = RangeDecoder::new(&buf);

        for i in 0..ITERATIONS as u8 {
            let logp = get_logp(i);

            // attempt decodes
            let ref_res = ref_dec.decode_bit_logp(logp);
            let opi_res = opi_dec.decode_bit_logp(logp);

            assert_eq!(ref_res, opi_res.unwrap()); // test decodes
        }
    }

    #[test]
    fn decode_bit_logp_random_bits_randomly() {
        use rand::distributions::Uniform;

        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, 16);
        for _ in 0..ITERATIONS {
            let buf = rng.gen::<[u8; BUFFER_LEN]>();
            decode_bit_logp_generic(&buf, |_| rng.sample(ft_dist));
        }
    }

    #[test]
    fn decode_bit_logp_random_bits_iteratively() {
        for _ in 0..ITERATIONS {
            let buf = rand::random::<[u8; BUFFER_LEN]>();
            decode_bit_logp_generic(&buf, logp1gen);
        }
    }

    #[test]
    fn decode_bit_logp_iterative_bits_randomly() {
        use rand::distributions::Uniform;

        let buf: Vec<_> = (0..ITERATIONS as u8).collect();
        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, 16);

        for _ in 0..ITERATIONS {
            decode_bit_logp_generic(&buf[..], |_| rng.sample(ft_dist));
        }
    }

    #[test]
    fn decode_bit_logp_iterative_bits_iteratively() {
        let buf: Vec<_> = (0..ITERATIONS as u8).collect();
        decode_bit_logp_generic(&buf[..], logp1gen);
    }

    #[test]
    fn decode_bit_logp_empty_input() {
        decode_bit_logp_generic(&[], logp1gen);
    }

    #[test]
    fn decode_bit_logp_fail_large_logp() {
        let mut opi_dec = RangeDecoder::new(DUMMY_DATA);
        // FIXME this should probably be 16, but I'm not entirely sure
        for i in 32..u8::max_value() {
            assert_eq!(opi_dec.decode_bit_logp(i), None);
        }
    }

    fn decode_icdf_generic<'a, T, U>(buf: &[u8], mut get_icdf: T, mut get_ftb: U)
    where
        T: FnMut(u8) -> &'a [u8],
        U: FnMut(u8) -> u8,
    {
        // initialize decoders
        let mut ref_dec = EcDec::new(&buf);
        let mut opi_dec = RangeDecoder::new(&buf);

        for i in 0..ITERATIONS as u8 {
            let icdf = get_icdf(i);
            let ftb = get_ftb(i);

            // attempt decodes
            let ref_res = ref_dec.decode_icdf(icdf, ftb);
            let opi_res = opi_dec.decode_icdf(icdf, ftb);

            println!("{:?}; {}\n{:?}\n{:?}\n", icdf, ftb, ref_dec, opi_dec);
            assert_eq!(ref_res, opi_res.unwrap()); // test decodes
        }
    }

    fn generate_random_icdfs<T>(rng: &mut T) -> Vec<Vec<u8>>
    where
        T: Rng,
    {
        use rand::distributions::Uniform;
        use std::iter::{once, repeat_with};

        let dist = Uniform::new(0, ITERATIONS);
        let icdvd = Uniform::new(1, u8::max_value());

        repeat_with(|| {
            let len = rng.sample(dist);
            repeat_with(|| rng.sample(icdvd))
                .take(len)
                .chain(once(0))
                .collect()
        })
        .take(ITERATIONS)
        .collect()
    }

    #[test]
    fn decode_icdf_random_bits_randomly_with_random_icdfs() {
        use rand::distributions::Uniform;

        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, 16);

        for _ in 0..ITERATIONS {
            let buf = rng.gen::<[u8; BUFFER_LEN]>();
            let icdfs = generate_random_icdfs(&mut rng);

            decode_icdf_generic(
                &buf,
                |i| &icdfs[usize::from(i) % icdfs.len()],
                |_| rng.sample(ft_dist),
            );
        }
    }

    #[test]
    fn decode_icdf_random_bits_iteratively_with_random_icdfs() {
        let mut rng = rand::thread_rng();
        for _ in 0..ITERATIONS {
            let buf = rng.gen::<[u8; BUFFER_LEN]>();
            let icdfs = generate_random_icdfs(&mut rng);

            decode_icdf_generic(&buf, |i| &icdfs[usize::from(i) % icdfs.len()], logp1gen);
        }
    }

    #[test]
    fn decode_icdf_iterative_bits_randomly_with_random_icdfs() {
        use rand::distributions::Uniform;

        let buf: Vec<_> = (0..ITERATIONS as u8).collect();
        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, 16);

        for _ in 0..ITERATIONS {
            let icdfs = generate_random_icdfs(&mut rng);
            decode_icdf_generic(
                &buf[..],
                |i| &icdfs[usize::from(i) % icdfs.len()],
                |_| rng.sample(ft_dist),
            );
        }
    }

    #[test]
    fn decode_icdf_iterative_bits_iteratively_with_random_icdfs() {
        let buf: Vec<_> = (0..ITERATIONS as u8).collect();
        let mut rng = rand::thread_rng();

        for _ in 0..ITERATIONS {
            let icdfs = generate_random_icdfs(&mut rng);
            decode_icdf_generic(&buf[..], |i| &icdfs[usize::from(i) % icdfs.len()], logp1gen);
        }
    }

    #[test]
    fn decode_icdf_empty_input_with_random_icdfs() {
        let mut rng = rand::thread_rng();
        for _ in 0..ITERATIONS {
            let icdfs = generate_random_icdfs(&mut rng);
            decode_icdf_generic(&[], |i| &icdfs[usize::from(i) % icdfs.len()], logp1gen);
        }
    }

    #[test]
    fn decode_icdf_random_bits_randomly_with_static_icdfs() {
        use rand::distributions::Uniform;

        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, 16);

        for _ in 0..ITERATIONS {
            let buf = rng.gen::<[u8; BUFFER_LEN]>();

            decode_icdf_generic(&buf, |_| DUMMY_ICDF, |_| rng.sample(ft_dist));
        }
    }

    #[test]
    fn decode_icdf_random_bits_iteratively_with_static_icdfs() {
        let mut rng = rand::thread_rng();
        for _ in 0..ITERATIONS {
            let buf = rng.gen::<[u8; BUFFER_LEN]>();
            decode_icdf_generic(&buf, |_| DUMMY_ICDF, logp1gen);
        }
    }

    #[test]
    fn decode_icdf_iterative_bits_randomly_with_static_icdfs() {
        use rand::distributions::Uniform;

        let buf: Vec<_> = (0..ITERATIONS as u8).collect();
        let mut rng = rand::thread_rng();
        let ft_dist = Uniform::new(1, 16);

        for _ in 0..ITERATIONS {
            decode_icdf_generic(&buf[..], |_| DUMMY_ICDF, |_| rng.sample(ft_dist));
        }
    }

    #[test]
    fn decode_icdf_iterative_bits_iteratively_with_static_icdfs() {
        let buf: Vec<_> = (0..ITERATIONS as u8).collect();
        decode_icdf_generic(&buf[..], |_| DUMMY_ICDF, logp1gen);
    }

    #[test]
    fn decode_icdf_empty_input_with_static_icdfs() {
        decode_icdf_generic(&[], |_| DUMMY_ICDF, logp1gen);
    }

    #[test]
    fn decode_icdf_empty_input_with_empty_icdfs() {
        decode_icdf_generic(&[], |_| &[0], logp1gen);
    }

    #[test]
    fn decode_icdf_fail_large_logp_with_static_icdf() {
        let mut opi_dec = RangeDecoder::new(DUMMY_DATA);
        // FIXME this should probably be 16, but I'm not entirely sure
        for i in 32..u8::max_value() {
            assert_eq!(opi_dec.decode_icdf(DUMMY_ICDF, i), None);
        }
    }

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<RangeDecoder<'_>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<RangeDecoder<'_>>();
    }
}
