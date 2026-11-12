//! A JPEG's EXIF orientation (tag 0x0112 in IFD0), read by our own small
//! TIFF reader: the only thing Central takes from a photo's metadata, and
//! only to turn its pixels upright. Anything unreadable is orientation 1.

/// The orientation in `tiff` (EXIF data from its TIFF header on): 1 to 8.
pub(crate) fn orientation(tiff: &[u8]) -> u8 {
    read(tiff).filter(|o| (1..=8).contains(o)).unwrap_or(1)
}

fn read(tiff: &[u8]) -> Option<u8> {
    let little = match tiff.get(..4)? {
        [b'I', b'I', 42, 0] => true,
        [b'M', b'M', 0, 42] => false,
        _ => return None,
    };
    let u16_at = |at: usize| -> Option<u16> {
        let bytes: [u8; 2] = tiff.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        })
    };
    let u32_at = |at: usize| -> Option<u32> {
        let bytes: [u8; 4] = tiff.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    };
    let ifd = usize::try_from(u32_at(4)?).ok()?;
    let count = usize::from(u16_at(ifd)?);
    for entry in 0..count.min(512) {
        let at = ifd.checked_add(2)?.checked_add(entry.checked_mul(12)?)?;
        // Tag 0x0112, type SHORT (3), one value, held in the entry itself.
        if u16_at(at)? == 0x0112 && u16_at(at + 2)? == 3 && u32_at(at + 4)? == 1 {
            return u8::try_from(u16_at(at + 8)?).ok();
        }
    }
    None
}

/// The TIFF of an EXIF block with one orientation entry, for tests.
#[cfg(test)]
pub(crate) fn tiff(orientation: u16, little: bool) -> Vec<u8> {
    let u16b = |v: u16| {
        if little {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    let u32b = |v: u32| {
        if little {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    let mut out = if little {
        b"II*\0".to_vec()
    } else {
        b"MM\0*".to_vec()
    };
    out.extend_from_slice(&u32b(8));
    out.extend_from_slice(&u16b(1));
    out.extend_from_slice(&u16b(0x0112));
    out.extend_from_slice(&u16b(3));
    out.extend_from_slice(&u32b(1));
    out.extend_from_slice(&u16b(orientation));
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&u32b(0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_byte_orders_and_only_valid_values() {
        for value in 1..=8 {
            assert_eq!(
                orientation(&tiff(value, true)),
                u8::try_from(value).unwrap()
            );
            assert_eq!(
                orientation(&tiff(value, false)),
                u8::try_from(value).unwrap()
            );
        }
        assert_eq!(orientation(&tiff(0, true)), 1);
        assert_eq!(orientation(&tiff(9, false)), 1);
        assert_eq!(orientation(&tiff(0x0106, true)), 1);
        assert_eq!(orientation(b"not tiff"), 1);
        let whole = tiff(6, true);
        for end in 0..whole.len() {
            let found = orientation(&whole[..end]);
            assert!(found == 1 || found == 6, "{end}");
        }
        // an IFD offset far past the end
        let mut far = tiff(6, false);
        far[4..8].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(orientation(&far), 1);
    }
}
