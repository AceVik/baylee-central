//! Operator-only asset inspection. The child receives one still and no credentials.
use std::io::{Cursor, Read, Write};
use std::process::ExitCode;

const LIMIT: usize = 16 * 1024 * 1024;

fn inspect(bytes: &[u8]) -> Option<[u32; 2]> {
    let size = |data: &[u8]| usize::try_from(u32::from_le_bytes(data.try_into().ok()?)).ok();
    if bytes.get(..4)? != b"RIFF"
        || size(bytes.get(4..8)?)?.checked_add(8)? != bytes.len()
        || bytes.get(8..12)? != b"WEBP"
        || !matches!(bytes.get(12..16)?, b"VP8 " | b"VP8L")
    {
        return None;
    }
    // Only one image chunk. EXIF, XMP, ICC, animation, extra data and padding other than zero
    // are refused, so encoded application data cannot be published as image metadata.
    let len = size(bytes.get(16..20)?)?;
    let end = 20usize.checked_add(len)?;
    let padding = bytes.get(end..)?;
    if padding.len() != len % 2 || padding.first().is_some_and(|p| *p != 0) {
        return None;
    }
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(bytes)).ok()?;
    decoder.set_memory_limit(128 * 1024 * 1024);
    let (width, height) = decoder.dimensions();
    if ![width, height]
        .into_iter()
        .all(|side| (1..=4096).contains(&side))
        || decoder.is_animated()
    {
        return None;
    }
    let size = decoder.output_buffer_size()?;
    if size > 4096 * 4096 * 4 {
        return None;
    }
    decoder.read_image(&mut vec![0; size]).ok()?;
    Some([width, height])
}
pub(super) fn run() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {}));
    std::thread::spawn(|| {
        std::thread::sleep(super::END_BY);
        std::process::exit(2);
    });
    let mut bytes = Vec::new();
    if std::io::stdin()
        .lock()
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > LIMIT
    {
        return ExitCode::FAILURE;
    }
    let Some(dimensions) = inspect(&bytes) else {
        return ExitCode::FAILURE;
    };
    if std::io::stdout()
        .lock()
        .write_all(&dimensions.map(u32::to_be_bytes).concat())
        .is_err()
    {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::inspect;

    #[test]
    fn only_complete_metadata_free_stills_decode() {
        let original = include_bytes!("../tests/fixtures/lossy.webp");
        assert!(inspect(original).is_some());
        assert!(inspect(include_bytes!("../tests/fixtures/animated.webp")).is_none());
        for i in 0..original.len() {
            assert!(inspect(&original[..i]).is_none());
        }
        let mut extra = original.to_vec();
        extra.extend_from_slice(b"XMP \x04\0\0\0test");
        let new_len = u32::try_from(extra.len() - 8).unwrap();
        extra[4..8].copy_from_slice(&new_len.to_le_bytes());
        assert!(inspect(&extra).is_none());
        let mut corrupt = *original;
        corrupt[23..26].fill(0);
        assert!(inspect(&corrupt).is_none());
    }
}
