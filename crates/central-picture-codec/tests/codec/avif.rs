//! AVIF originals: other encoders' files, 10-bit, alpha, the item sizes
//! (`ispe`) and the AV1 sequence header bounded before `rav1d` allocates,
//! other HEIF brands, and cut or damaged files.
use crate::support::{at, avif_of, fixture, near, through_worker, timed, valid};
use central_picture::{Format, Refusal};

const RED: [u8; 4] = [255, 0, 0, 255];

/// Damaged AVIF goes through the worker: avif-parse 2.1.0 panics on some
/// (caught), and rav1d 1.1.0 aborts the whole process on others.
fn survives(bytes: &[u8]) {
    if let Ok(output) = through_worker(Format::Avif, bytes) {
        valid(&output);
    }
}

#[test]
fn other_encoders_ten_bit_and_alpha_are_taken() {
    // macOS ImageIO (48 × 32 quadrant) and SVT-AV1 through ffmpeg (10-bit)
    let imageio = valid(&timed(Format::Avif, &fixture("imageio.avif")).unwrap());
    assert!(
        near(at(&imageio, 20, 20), RED, 40),
        "{:?}",
        at(&imageio, 20, 20)
    );
    let ten = valid(&timed(Format::Avif, &fixture("ten-bit.avif")).unwrap());
    assert!(
        near(at(&ten, 128, 128), [0x33, 0x66, 0x99, 255], 12),
        "{:?}",
        at(&ten, 128, 128)
    );
    let alpha = avif_of(32, 32, |x, _| if x < 16 { [255, 0, 0, 0] } else { RED });
    let rgba = valid(&timed(Format::Avif, &alpha).unwrap());
    assert_eq!(at(&rgba, 10, 10)[3], 0);
    assert!(near(at(&rgba, 240, 10), RED, 24));
}

/// The first `ispe` of a file, as (offset of its width, width, height).
fn ispe(file: &[u8]) -> usize {
    file.windows(4).position(|w| w == b"ispe").unwrap() + 8
}

#[test]
fn item_sizes_that_claim_too_much_are_refused_before_decoding() {
    let file = avif_of(32, 32, |_, _| RED);
    for (width, height) in [(30_000_u32, 30_000_u32), (9_000, 16), (5_000, 5_000)] {
        let mut huge = file.clone();
        let at = ispe(&huge);
        huge[at..at + 4].copy_from_slice(&width.to_be_bytes());
        huge[at + 4..at + 8].copy_from_slice(&height.to_be_bytes());
        assert_eq!(
            timed(Format::Avif, &huge),
            Err(Refusal::TooManyPixels),
            "{width}x{height}"
        );
    }
}

#[test]
fn an_av1_sequence_header_that_allows_huge_frames_is_refused() {
    // The item's own sequence header: a small ispe, but frame sizes of up
    // to 65536 per side, which rav1d (with no frame-size limit set by
    // avif-decode) would allocate for.
    let file = avif_of(32, 32, |_, _| RED);
    let mdat = file.windows(4).rposition(|w| w == b"mdat").unwrap() + 4;
    // the first OBU in the item: a sequence header (type 1) with a size field
    let start = (mdat..file.len() - 2)
        .find(|&i| {
            file[i] == 0x0a
                && file[i + 1] < 64
                && file[i + 2] >> 5 <= 2
                && (file[i + 2] >> 3) & 3 == 3
        })
        .expect("sequence header OBU");
    let payload = start + 2;
    let mut huge = file.clone();
    // reduced still header: profile 3 bits, still 1, reduced 1, level 5;
    // then frame_width_bits_minus_1 and frame_height_bits_minus_1, 4 bits each
    let mut bits: Vec<u8> = (0..32)
        .map(|i| (huge[payload + i / 8] >> (7 - i % 8)) & 1)
        .collect();
    assert_eq!(bits[4], 1, "a reduced still-picture header");
    bits[10..18].fill(1);
    for (i, chunk) in bits.chunks(8).enumerate() {
        huge[payload + i] = chunk.iter().fold(0, |b, bit| (b << 1) | bit);
    }
    assert_eq!(timed(Format::Avif, &huge), Err(Refusal::TooManyPixels));
}

#[test]
fn an_avif_image_sequence_is_refused_as_unsupported() {
    // three frames of `testsrc2`, major brand `avis` (fixtures/README.md)
    let sequence = fixture("animated.avif");
    assert_eq!(&sequence[8..12], b"avis");
    assert_eq!(timed(Format::Avif, &sequence), Err(Refusal::Unsupported));
    assert_eq!(
        through_worker(Format::Avif, &sequence),
        Err(Refusal::Unsupported)
    );
    // the same file naming `avif` first gives its still item
    let mut still = sequence.clone();
    still[8..12].copy_from_slice(b"avif");
    let rgba = valid(&timed(Format::Avif, &still).unwrap());
    assert_eq!(rgba.len(), 256 * 256 * 4);
}

#[test]
fn other_brands_cuts_and_damage_are_refused() {
    let file = avif_of(32, 32, |x, y| {
        [
            u8::try_from(x * 8).unwrap(),
            u8::try_from(y * 8).unwrap(),
            0,
            255,
        ]
    });
    let mut heic = file.clone();
    heic[8..12].copy_from_slice(b"heic");
    let compatible = heic
        .windows(4)
        .skip(12)
        .position(|w| w == b"avif")
        .map(|p| p + 12);
    if let Some(at) = compatible {
        heic[at..at + 4].copy_from_slice(b"mif1");
    }
    assert_eq!(timed(Format::Avif, &heic), Err(Refusal::Unsupported));
    for end in 0..file.len() {
        assert!(
            through_worker(Format::Avif, &file[..end]).is_err(),
            "cut at {end}"
        );
    }
    for at in (12..file.len()).step_by(3) {
        let mut damaged = file.clone();
        damaged[at] ^= 0x41;
        survives(&damaged);
    }
    // an AVIF followed by a document
    let polyglot = [file.clone(), b"<html><script>alert(1)</script>".to_vec()].concat();
    survives(&polyglot);
}

#[test]
fn a_decoder_that_aborts_the_worker_harms_nothing() {
    // Found by the damage loop above: a flipped byte in the AV1 data that
    // makes rav1d 1.1.0 unwrap a None inside its C ABI and abort.
    let file = avif_of(32, 32, |x, y| {
        [
            u8::try_from(x * 8).unwrap(),
            u8::try_from(y * 8).unwrap(),
            0,
            255,
        ]
    });
    let mut aborted = 0;
    for at in (12..file.len()).step_by(3) {
        let mut damaged = file.clone();
        damaged[at] ^= 0x41;
        if through_worker(Format::Avif, &damaged) == Err(Refusal::Failed) {
            aborted += 1;
        }
    }
    assert!(
        through_worker(Format::Avif, &file).is_ok(),
        "the next picture still converts"
    );
    println!("{aborted} damaged files ended the worker");
}
