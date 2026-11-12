//! JPEG, GIF, WebP, BMP and ICO: orientation, animation, polyglots, headers
//! that claim too much, and cut or damaged files.
use crate::support::{at, fixture, gif_of, image_of, near, quadrant, timed, valid, webp_of};
use central_picture::{Format, Output, Refusal};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A cut or damaged file ends in a refusal or a valid pair, never a panic.
fn survives(format: Format, bytes: &[u8]) {
    if let Ok(output) = timed(format, bytes) {
        valid(&output);
    }
}

/// An EXIF APP1 segment with one orientation entry, big-endian.
fn exif_segment(orientation: u16) -> Vec<u8> {
    let mut tiff = b"MM\0*\0\0\0\x08\0\x01".to_vec();
    tiff.extend_from_slice(&[0x01, 0x12, 0, 3, 0, 0, 0, 1]);
    tiff.extend_from_slice(&orientation.to_be_bytes());
    tiff.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    let mut segment = vec![0xff, 0xe1];
    segment.extend_from_slice(&u16::try_from(2 + 6 + tiff.len()).unwrap().to_be_bytes());
    segment.extend_from_slice(b"Exif\0\0");
    segment.extend_from_slice(&tiff);
    segment
}

fn with_exif(jpeg: &[u8], orientation: u16) -> Vec<u8> {
    [&jpeg[..2], &exif_segment(orientation), &jpeg[2..]].concat()
}

#[test]
fn jpeg_baseline_progressive_and_grey_are_taken() {
    for name in ["quadrant.jpg", "progressive.jpg"] {
        let rgba = valid(&timed(Format::Jpeg, &fixture(name)).unwrap());
        assert!(near(at(&rgba, 20, 20), RED, 40), "{name}");
        assert!(near(at(&rgba, 200, 200), BLUE, 40), "{name}");
    }
    let grey = valid(&timed(Format::Jpeg, &fixture("grey.jpg")).unwrap());
    let [r, g, b, a] = at(&grey, 20, 20);
    assert!(r == g && g == b && a == 255);
}

#[test]
fn jpeg_exif_orientation_turns_the_picture_upright() {
    let plain = fixture("quadrant.jpg");
    // where the red quarter ends up for each orientation
    for (orientation, (x, y)) in [
        (1, (20, 20)),
        (3, (235, 235)),
        (6, (235, 20)),
        (8, (20, 235)),
        (2, (235, 20)),
    ] {
        let rgba = valid(&timed(Format::Jpeg, &with_exif(&plain, orientation)).unwrap());
        assert!(near(at(&rgba, x, y), RED, 40), "orientation {orientation}");
    }
    // an orientation out of range, and EXIF that is no TIFF, change nothing
    let upright = timed(Format::Jpeg, &plain).unwrap();
    assert_eq!(
        timed(Format::Jpeg, &with_exif(&plain, 9)),
        Ok(upright.clone())
    );
    let junk = [
        &plain[..2],
        &[0xff, 0xe1, 0, 12][..],
        b"Exif\0\0junk",
        &plain[2..],
    ]
    .concat();
    assert_eq!(timed(Format::Jpeg, &junk), Ok(upright.clone()));
    // the output holds nothing of the EXIF
    let output = timed(Format::Jpeg, &with_exif(&plain, 6)).unwrap();
    assert!(
        !output.png.windows(4).any(|w| w == b"Exif")
            && !output.avif.windows(4).any(|w| w == b"Exif")
    );
}

#[test]
fn jpeg_frames_that_claim_too_much_are_refused_and_polyglots_are_only_pictures() {
    let plain = fixture("quadrant.jpg");
    let sof = plain.windows(2).position(|w| w == [0xff, 0xc0]).unwrap();
    for (height, width) in [(30_000_u16, 30_000_u16), (100, 9_000), (5_000, 5_000)] {
        let mut huge = plain.clone();
        huge[sof + 5..sof + 7].copy_from_slice(&height.to_be_bytes());
        huge[sof + 7..sof + 9].copy_from_slice(&width.to_be_bytes());
        assert_eq!(
            timed(Format::Jpeg, &huge),
            Err(Refusal::TooManyPixels),
            "{width}x{height}"
        );
    }
    let upright = timed(Format::Jpeg, &plain).unwrap();
    let polyglot = [
        plain.clone(),
        b"<html><script>alert(1)</script></html>PK\x05\x06\0\0".to_vec(),
    ]
    .concat();
    assert_eq!(timed(Format::Jpeg, &polyglot), Ok(upright));
    for end in (0..plain.len()).step_by(7) {
        survives(Format::Jpeg, &plain[..end]);
    }
    for at in (2..plain.len()).step_by(11) {
        let mut damaged = plain.clone();
        damaged[at] ^= 0xa5;
        survives(Format::Jpeg, &damaged);
    }
}

#[test]
fn gif_gives_its_first_frame_and_bounds_its_screen() {
    let animated = gif_of(64, 64, 3);
    let rgba = valid(&timed(Format::Gif, &animated).unwrap());
    assert!(rgba.chunks(4).all(|p| p == RED));
    let mut huge = gif_of(64, 64, 1);
    huge[6..10].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
    assert_eq!(timed(Format::Gif, &huge), Err(Refusal::TooManyPixels));
    let polyglot = [
        gif_of(64, 64, 1),
        b"<html><script>alert(1)</script>".to_vec(),
    ]
    .concat();
    assert_eq!(
        timed(Format::Gif, &polyglot),
        timed(Format::Gif, &gif_of(64, 64, 1))
    );
    // a frame that sits partly outside a smaller screen
    let mut outside = gif_of(64, 64, 1);
    outside[6..10].copy_from_slice(&[32, 0, 32, 0]);
    survives(Format::Gif, &outside);
    let plain = gif_of(48, 48, 2);
    for end in 0..plain.len() {
        survives(Format::Gif, &plain[..end]);
    }
    for at in (6..plain.len()).step_by(3) {
        let mut damaged = plain.clone();
        damaged[at] ^= 0x5a;
        survives(Format::Gif, &damaged);
    }
}

#[test]
fn webp_lossy_lossless_and_animated_are_taken_and_bounded() {
    let lossy = valid(&timed(Format::Webp, &fixture("lossy.webp")).unwrap());
    assert!(near(at(&lossy, 20, 20), RED, 48) && near(at(&lossy, 200, 200), BLUE, 48));
    let animated = valid(&timed(Format::Webp, &fixture("animated.webp")).unwrap());
    assert!(
        near(at(&animated, 128, 128), RED, 8),
        "{:?}",
        at(&animated, 128, 128)
    );
    let mut claim = b"RIFF\x16\0\0\0WEBPVP8X\x0a\0\0\0\x10\0\0\0".to_vec();
    claim.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    assert_eq!(timed(Format::Webp, &claim), Err(Refusal::TooManyPixels));
    let lossless = webp_of(40, 40, quadrant(40, 40));
    for end in 0..lossless.len() {
        survives(Format::Webp, &lossless[..end]);
    }
    for at in (12..lossless.len()).step_by(2) {
        let mut damaged = lossless.clone();
        damaged[at] ^= 0x33;
        survives(Format::Webp, &damaged);
    }
}

#[test]
fn bmp_and_ico_are_bounded_by_their_own_headers() {
    let bmp = image_of(image::ImageFormat::Bmp, 40, 40, quadrant(40, 40));
    let mut huge = bmp.clone();
    huge[18..22].copy_from_slice(&100_000_i32.to_le_bytes());
    huge[22..26].copy_from_slice(&100_000_i32.to_le_bytes());
    assert_eq!(timed(Format::Bmp, &huge), Err(Refusal::TooManyPixels));
    // top-down rows (a negative height) are drawn upside down from bottom-up ones
    let mut top_down = bmp.clone();
    top_down[22..26].copy_from_slice(&(-40_i32).to_le_bytes());
    let flipped = valid(&timed(Format::Bmp, &top_down).unwrap());
    let upright = valid(&timed(Format::Bmp, &bmp).unwrap());
    assert_eq!(at(&upright, 20, 20), RED);
    assert_eq!(at(&flipped, 20, 235), RED);
    for end in 0..bmp.len().min(400) {
        survives(Format::Bmp, &bmp[..end]);
    }
    let ico = image_of(image::ImageFormat::Ico, 40, 40, quadrant(40, 40));
    let Output { png, .. } = timed(Format::Ico, &ico).unwrap();
    assert!(central_picture::png_master(&png));
    let mut past = ico.clone();
    past[18..22].copy_from_slice(&0x00ff_ffff_u32.to_le_bytes());
    assert_eq!(timed(Format::Ico, &past), Err(Refusal::Malformed));
    // an entry whose embedded PNG claims 9000 × 9000
    let mut claim = ico.clone();
    let ihdr = claim.windows(4).position(|w| w == b"IHDR").unwrap();
    claim[ihdr + 4..ihdr + 12].copy_from_slice(&[0, 0, 0x23, 0x28, 0, 0, 0x23, 0x28]);
    assert_eq!(timed(Format::Ico, &claim), Err(Refusal::TooManyPixels));
    let mut cursor = ico.clone();
    cursor[2] = 2;
    assert_eq!(timed(Format::Ico, &cursor), Err(Refusal::Unsupported));
    for end in 0..ico.len() {
        survives(Format::Ico, &ico[..end]);
    }
}
