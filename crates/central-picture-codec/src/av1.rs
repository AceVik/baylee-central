//! The bound on an AV1 item before `rav1d` sees it. `avif-decode` sets no
//! frame-size limit, and a frame may override the sequence's size with up
//! to `frame_width_bits` bits per side, so a small `ispe` says nothing of
//! what the decoder would allocate. This reads every sequence header OBU in
//! the item (AV1 §5.5) and refuses one whose frame sizes could exceed
//! [`MAX_SIDE`] or whose largest frame could exceed [`MAX_PIXELS`].
use super::{MAX_PIXELS, MAX_SIDE, Refusal};

/// Every sequence header in `item` allows only frames within the bounds,
/// and there is at least one.
///
/// # Errors
/// `TooManyPixels` beyond the bounds; `Malformed` for an item without a
/// whole sequence header.
pub(crate) fn bounded(item: &[u8]) -> Result<(), Refusal> {
    let mut rest = item;
    let mut seen = false;
    while !rest.is_empty() {
        let (kind, payload, after) = obu(rest).ok_or(Refusal::Malformed)?;
        if kind == 1 {
            let (width_bits, height_bits) = frame_bits(payload).ok_or(Refusal::Malformed)?;
            let widest = 1_u64 << width_bits;
            let tallest = 1_u64 << height_bits;
            if widest > u64::from(MAX_SIDE)
                || tallest > u64::from(MAX_SIDE)
                || widest * tallest > MAX_PIXELS
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
fn frame_bits(payload: &[u8]) -> Option<(u32, u32)> {
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
    let _max_width = b.bits(width_bits)?;
    let _max_height = b.bits(height_bits)?;
    Some((width_bits, height_bits))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

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
        push(0, width_bits);
        push(0, height_bits);
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
        assert_eq!(bounded(&sequence(12, 12)), Ok(()));
        assert_eq!(bounded(&sequence(13, 11)), Ok(()));
        assert_eq!(bounded(&sequence(14, 8)), Err(Refusal::TooManyPixels));
        assert_eq!(bounded(&sequence(13, 12)), Err(Refusal::TooManyPixels));
        assert_eq!(bounded(&sequence(16, 16)), Err(Refusal::TooManyPixels));
        // two headers: the second one counts as much as the first
        let both = [sequence(8, 8), sequence(16, 16)].concat();
        assert_eq!(bounded(&both), Err(Refusal::TooManyPixels));
        // no sequence header at all, a forbidden bit, and every cut
        assert_eq!(bounded(&[0x12, 0x00]), Err(Refusal::Malformed));
        assert_eq!(bounded(&[0x8a, 0x00]), Err(Refusal::Malformed));
        let whole = sequence(10, 10);
        for end in 0..whole.len() {
            assert!(bounded(&whole[..end]).is_err(), "{end}");
        }
        // a size field that never ends
        assert_eq!(
            bounded(&[0x0a, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            Err(Refusal::Malformed)
        );
    }
}
