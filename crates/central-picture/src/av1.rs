//! The bound on an AV1 item before `rav1d` sees it. `avif-decode` sets no
//! frame-size limit, and a frame may override the sequence's size with up
//! to `frame_width_bits` bits per side, so a small `ispe` says nothing of
//! what the decoder would allocate. This reads every sequence header OBU in
//! the item (AV1 §5.5) and refuses one whose frame sizes could exceed
//! [`MAX_SIDE`] or whose largest frame could exceed [`MAX_PIXELS`].
use super::Refusal;

/// Every sequence header in `item` allows only frames within the bounds,
/// and there is at least one.
///
/// # Errors
/// `TooManyPixels` beyond the bounds; `Malformed` for an item without a
/// whole sequence header.
pub fn bounded(item: &[u8], max_side: u32, max_pixels: u64) -> Result<(), Refusal> {
    let mut rest = item;
    let mut seen = false;
    while !rest.is_empty() {
        let (kind, payload, after) = obu(rest).ok_or(Refusal::Malformed)?;
        if kind == 1 {
            let (width_bits, height_bits, _) = frame_bits(payload).ok_or(Refusal::Malformed)?;
            let widest = 1_u64 << width_bits;
            let tallest = 1_u64 << height_bits;
            if widest > u64::from(max_side)
                || tallest > u64::from(max_side)
                || widest * tallest > max_pixels
            {
                return Err(Refusal::TooManyPixels);
            }
            seen = true;
        }
        rest = after;
    }
    if seen {
        Ok(())
    } else {
        Err(Refusal::Malformed)
    }
}

/// The largest frame every sequence header in `item` allows (its
/// `max_frame_width` and `max_frame_height`); `None` for an item without a
/// whole sequence header or with an OBU that cannot be read.
#[must_use]
pub fn largest(item: &[u8]) -> Option<(u32, u32)> {
    let mut rest = item;
    let mut largest: Option<(u32, u32)> = None;
    while !rest.is_empty() {
        let (kind, payload, after) = obu(rest)?;
        if kind == 1 {
            let (_, _, (width, height)) = frame_bits(payload)?;
            let (w, h) = largest.unwrap_or((0, 0));
            largest = Some((w.max(width), h.max(height)));
        }
        rest = after;
    }
    largest
}

/// One OBU: its type, its payload and what follows. An OBU without a size
/// field runs to the end of the item.
fn obu(bytes: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let header = *bytes.first()?;
    if header & 0x80 != 0 {
        return None;
    }
    let kind = (header >> 3) & 0x0f;
    let extension = header & 0x04 != 0;
    let has_size = header & 0x02 != 0;
    let mut at = 1 + usize::from(extension);
    if !has_size {
        return Some((kind, bytes.get(at..)?, &[]));
    }
    let mut size: u64 = 0;
    for i in 0..8 {
        let byte = *bytes.get(at)?;
        at += 1;
        size |= u64::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            break;
        }
        if i == 7 {
            return None;
        }
    }
    let end = at.checked_add(usize::try_from(size).ok()?)?;
    Some((kind, bytes.get(at..end)?, bytes.get(end..)?))
}

/// Bits read most significant first.
struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<u32> {
        let byte = *self.bytes.get(self.at / 8)?;
        let bit = (byte >> (7 - self.at % 8)) & 1;
        self.at += 1;
        Some(u32::from(bit))
    }

    fn bits(&mut self, n: u32) -> Option<u32> {
        (0..n).try_fold(0_u32, |value, _| Some((value << 1) | self.bit()?))
    }

    /// `uvlc()`: leading zeros, then as many bits.
    fn uvlc(&mut self) -> Option<()> {
        let mut zeros = 0;
        while self.bit()? == 0 {
            zeros += 1;
            if zeros >= 32 {
                return None;
            }
        }
        self.bits(zeros).map(|_| ())
    }
}

/// `frame_width_bits` and `frame_height_bits` of a sequence header.
fn frame_bits(payload: &[u8]) -> Option<(u32, u32, (u32, u32))> {
    let mut b = Bits {
        bytes: payload,
        at: 0,
    };
    let _profile = b.bits(3)?;
    let _still = b.bit()?;
    let reduced = b.bit()? == 1;
    if reduced {
        let _level = b.bits(5)?;
    } else {
        let timing = b.bit()? == 1;
        let mut decoder_model = false;
        let mut buffer_delay_bits = 0;
        if timing {
            let _units = b.bits(32)?;
            let _scale = b.bits(32)?;
            if b.bit()? == 1 {
                b.uvlc()?;
            }
            decoder_model = b.bit()? == 1;
            if decoder_model {
                buffer_delay_bits = b.bits(5)? + 1;
                let _tick = b.bits(32)?;
                let _removal = b.bits(5)?;
                let _presentation = b.bits(5)?;
            }
        }
        let display_delay = b.bit()? == 1;
        let points = b.bits(5)? + 1;
        for _ in 0..points {
            let _idc = b.bits(12)?;
            let level = b.bits(5)?;
            if level > 7 {
                let _tier = b.bit()?;
            }
            if decoder_model && b.bit()? == 1 {
                let _decoder_delay = b.bits(buffer_delay_bits)?;
                let _encoder_delay = b.bits(buffer_delay_bits)?;
                let _low_delay = b.bit()?;
            }
            if display_delay && b.bit()? == 1 {
                let _initial = b.bits(4)?;
            }
        }
    }
    let width_bits = b.bits(4)? + 1;
    let height_bits = b.bits(4)? + 1;
    // The maxima themselves must be readable too.
    let max_width = b.bits(width_bits)?.checked_add(1)?;
    let max_height = b.bits(height_bits)?.checked_add(1)?;
    Some((width_bits, height_bits, (max_width, max_height)))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The worker's bounds: 8,192 px a side, 4096² pixels.
    fn bounds(item: &[u8]) -> Result<(), Refusal> {
        bounded(item, 8_192, 16_777_216)
    }

    /// A reduced still-picture sequence header OBU with these bit counts.
    pub(crate) fn sequence(width_bits: u32, height_bits: u32) -> Vec<u8> {
        let mut bits: Vec<u8> = Vec::new();
        let mut push = |value: u32, n: u32| {
            for i in (0..n).rev() {
                bits.push(u8::from((value >> i) & 1 == 1));
            }
        };
        push(0, 3);
        push(1, 1);
        push(1, 1);
        push(31, 5);
        push(width_bits - 1, 4);
        push(height_bits - 1, 4);
        // the largest frames the bits allow
        push((1 << width_bits) - 1, width_bits);
        push((1 << height_bits) - 1, height_bits);
        let payload: Vec<u8> = bits
            .chunks(8)
            .map(|chunk| {
                chunk
                    .iter()
                    .enumerate()
                    .fold(0, |byte, (i, bit)| byte | (bit << (7 - i)))
            })
            .collect();
        let mut out = vec![(1 << 3) | 0x02, u8::try_from(payload.len()).unwrap()];
        out.extend_from_slice(&payload);
        out
    }

    #[test]
    fn frame_bits_bound_the_frame() {
        assert_eq!(bounds(&sequence(12, 12)), Ok(()));
        assert_eq!(bounds(&sequence(13, 11)), Ok(()));
        assert_eq!(bounds(&sequence(14, 8)), Err(Refusal::TooManyPixels));
        assert_eq!(bounds(&sequence(13, 12)), Err(Refusal::TooManyPixels));
        assert_eq!(bounds(&sequence(16, 16)), Err(Refusal::TooManyPixels));
        // two headers: the second one counts as much as the first
        let both = [sequence(8, 8), sequence(16, 16)].concat();
        assert_eq!(bounds(&both), Err(Refusal::TooManyPixels));
        // no sequence header at all, a forbidden bit, and every cut
        assert_eq!(bounds(&[0x12, 0x00]), Err(Refusal::Malformed));
        assert_eq!(bounds(&[0x8a, 0x00]), Err(Refusal::Malformed));
        let whole = sequence(10, 10);
        for end in 0..whole.len() {
            assert!(bounds(&whole[..end]).is_err(), "{end}");
        }
        // a size field that never ends
        assert_eq!(
            bounds(&[0x0a, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            Err(Refusal::Malformed)
        );
    }

    /// Writes fields of any width, most significant bit first.
    #[derive(Default)]
    struct Writer(Vec<bool>);

    impl Writer {
        fn put(&mut self, value: u32, n: u32) -> &mut Self {
            self.0.extend((0..n).rev().map(|i| (value >> i) & 1 == 1));
            self
        }

        /// The header as one OBU whose size is written.
        fn obu(&self) -> Vec<u8> {
            let payload: Vec<u8> = self
                .0
                .chunks(8)
                .map(|bits| {
                    bits.iter()
                        .enumerate()
                        .fold(0, |byte, (i, &bit)| byte | (u8::from(bit) << (7 - i)))
                })
                .collect();
            let mut out = vec![0x0a, u8::try_from(payload.len()).unwrap()];
            out.extend(payload);
            out
        }
    }

    /// A full sequence header (AV1 § 5.5.1) with every optional part that is asked for,
    /// each filled with ones, so that a part skipped by the wrong width moves the rest.
    fn full(
        timing: bool,
        ticks: Option<u32>,
        model: bool,
        delay: bool,
        width_bits: u32,
    ) -> Vec<u8> {
        let mut w = Writer::default();
        w.put(0, 3).put(0, 1).put(0, 1);
        w.put(u32::from(timing), 1);
        if timing {
            w.put(u32::MAX, 32).put(u32::MAX, 32);
            w.put(u32::from(ticks.is_some()), 1);
            if let Some(zeros) = ticks {
                w.put(0, zeros).put(1, 1).put(u32::MAX, zeros);
            }
            w.put(u32::from(model), 1);
            if model {
                // Delays of ten bits each.
                w.put(9, 5).put(u32::MAX, 32).put(31, 5).put(31, 5);
            }
        }
        w.put(u32::from(delay), 1);
        // Three operating points: below, at and above the level that has a tier.
        w.put(2, 5);
        for (level, with_model, with_delay) in [(7, 1, 0), (8, 0, 1), (31, 1, 1)] {
            w.put(0xfff, 12).put(level, 5);
            if level > 7 {
                w.put(1, 1);
            }
            if timing && model {
                w.put(with_model, 1);
                if with_model == 1 {
                    w.put(0x3ff, 10).put(0x3ff, 10).put(1, 1);
                }
            }
            if delay {
                w.put(with_delay, 1);
                if with_delay == 1 {
                    w.put(15, 4);
                }
            }
        }
        w.put(width_bits - 1, 4).put(8, 4);
        w.put((1 << width_bits) - 2, width_bits).put(0x155, 9);
        w.obu()
    }

    #[test]
    fn a_full_header_is_read_through_every_optional_part() {
        for timing in [false, true] {
            for ticks in [None, Some(0), Some(1), Some(31)] {
                for model in [false, true] {
                    for delay in [false, true] {
                        let header = full(timing, ticks, model, delay, 13);
                        let what = format!("{timing} {ticks:?} {model} {delay}");
                        assert_eq!(largest(&header), Some((0x1fff, 0x156)), "{what}");
                        assert_eq!(bounds(&header), Ok(()), "{what}");
                        let wide = full(timing, ticks, model, delay, 14);
                        assert_eq!(bounds(&wide), Err(Refusal::TooManyPixels), "{what}");
                        for end in 1..header.len() {
                            assert!(bounds(&header[..end]).is_err(), "{what} cut at {end}");
                        }
                    }
                }
            }
        }
        // A tick count that never begins is not read as one.
        let mut endless = full(true, Some(31), false, false, 13);
        endless[2..].fill(0);
        endless[2] = 0b0000_0100;
        endless[10] = 0b0000_0010;
        assert_eq!(bounds(&endless), Err(Refusal::Malformed));
    }

    #[test]
    fn whatever_passes_the_bound_shows_no_larger_frame() {
        // Deterministic noise (xorshift) over valid headers: no input panics, and no
        // accepted one declares a frame past the bound it was accepted under.
        let seeds = [
            sequence(12, 12),
            sequence(16, 16),
            full(true, Some(1), true, true, 12),
            [
                sequence(8, 8),
                vec![0x32, 0x02, 0xaa, 0xbb],
                sequence(13, 11),
            ]
            .concat(),
        ];
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let (mut accepted, mut refused) = (0, 0);
        for round in 0..40_000 {
            let mut item = seeds[round % seeds.len()].clone();
            for _ in 0..=next() % 3 {
                let at = usize::try_from(next() % item.len() as u64).unwrap();
                item[at] = u8::try_from(next() & 0xff).unwrap();
            }
            if next() % 4 == 0 {
                item.truncate(usize::try_from(next() % item.len() as u64).unwrap());
            }
            if bounds(&item).is_err() {
                refused += 1;
                continue;
            }
            accepted += 1;
            let (width, height) = largest(&item).expect("an accepted item is readable");
            assert!(
                width <= 8_192 && height <= 8_192,
                "{width} x {height}: {item:x?}"
            );
        }
        // Both outcomes were reached, or the noise proves nothing.
        assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
    }

    #[test]
    fn a_size_is_at_most_eight_octets_and_inside_the_item() {
        let payload = sequence(8, 8).split_off(2);
        let sized = |size: &[u8]| [&[0x0a], size, &payload[..]].concat();
        let length = u8::try_from(payload.len()).unwrap();
        // The same size in eight octets, with empty high ones, is the same item.
        let mut long = [0x80; 8];
        long[0] |= length;
        long[7] = 0;
        assert_eq!(bounds(&sized(&long)), Ok(()));
        // In nine octets it is none.
        let mut longer = [0x80; 9];
        longer[0] |= length;
        longer[8] = 0;
        assert_eq!(bounds(&sized(&longer)), Err(Refusal::Malformed));
        for size in [
            &[length + 1][..],
            &[0x7f],
            &[0xff; 7],
            &[0xff, 0xff, 0xff, 0xff, 0x0f],
        ] {
            assert!(bounds(&sized(size)).is_err(), "{size:x?}");
        }
        // Without a size the rest of the item is the unit, whatever it holds.
        let open = [&[0x08][..], &payload].concat();
        assert_eq!(bounds(&open), Ok(()));
        // An extension octet stands before the payload.
        let extended = [&[0x0c, 0xff][..], &payload].concat();
        assert_eq!(bounds(&extended), Ok(()));
    }
}
