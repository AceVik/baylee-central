//! Every format converts to the pair the service accepts, the same bytes
//! each time, and the AVIF decodes again to the same picture.
use crate::support::{
    at, avif_of, decode_png, fixture, gif_of, image_of, near, png_of, quadrant, timed, valid,
    webp_of,
};
use central_picture::{Format, Refusal};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// One valid file of each format, drawn as the quadrant picture where the
/// format allows.
pub(crate) fn samples() -> Vec<(Format, Vec<u8>)> {
    vec![
        (
            Format::Png,
            png_of(64, 64, png::ColorType::Rgba, png::BitDepth::Eight, false, quadrant(64, 64)),
        ),
        (Format::Jpeg, fixture("quadrant.jpg")),
        (Format::Gif, gif_of(64, 64, 1)),
        (Format::Webp, webp_of(64, 64, quadrant(64, 64))),
        (Format::Avif, avif_of(64, 64, |x, y| quadrant(64, 64)(u32::try_from(x).unwrap(), u32::try_from(y).unwrap()))),
        (
            Format::Svg,
            b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 64 64'>\
              <rect width='64' height='64' fill='#00f'/><rect width='32' height='32' fill='#f00'/></svg>"
                .to_vec(),
        ),
        (Format::Bmp, image_of(image::ImageFormat::Bmp, 64, 64, quadrant(64, 64))),
        (Format::Ico, image_of(image::ImageFormat::Ico, 64, 64, quadrant(64, 64))),
    ]
}

#[test]
fn every_format_converts_and_the_pair_passes_the_service_s_checks() {
    for (format, bytes) in samples() {
        let output = timed(format, &bytes).unwrap_or_else(|r| panic!("{format:?}: {r}"));
        let rgba = valid(&output);
        // the same pixels give the same bytes, every time
        assert_eq!(timed(format, &bytes).unwrap(), output, "{format:?}");
        if format == Format::Gif {
            assert!(rgba.chunks(4).all(|p| p == RED), "gif");
            continue;
        }
        // the quadrant picture keeps its red quarter at the top left
        let tolerance = if matches!(format, Format::Jpeg | Format::Avif | Format::Webp) {
            40
        } else {
            0
        };
        assert!(
            near(at(&rgba, 20, 20), RED, tolerance),
            "{format:?} {:?}",
            at(&rgba, 20, 20)
        );
        assert!(near(at(&rgba, 200, 200), BLUE, tolerance), "{format:?}");
        assert!(near(at(&rgba, 200, 20), BLUE, tolerance), "{format:?}");
    }
}

#[test]
fn the_avif_decodes_again_to_the_master_s_picture() {
    for (format, bytes) in samples() {
        let output = timed(format, &bytes).unwrap();
        let image = avif_decode::Decoder::from_avif(&output.avif)
            .and_then(avif_decode::Decoder::to_image)
            .unwrap_or_else(|_| panic!("{format:?}"));
        let (width, height, pixel) = match image {
            avif_decode::Image::Rgb8(i) => (i.width(), i.height(), {
                let p = i.buf()[20 * i.stride() + 20];
                [p.r, p.g, p.b, 255]
            }),
            avif_decode::Image::Rgba8(i) => (i.width(), i.height(), {
                let p = i.buf()[20 * i.stride() + 20];
                [p.r, p.g, p.b, p.a]
            }),
            // the stored AVIF is 8 bits a sample (proposal §0.8)
            _ => panic!("{format:?}: 8-bit colour expected"),
        };
        assert_eq!((width, height), (256, 256), "{format:?}");
        let master = at(&decode_png(&output.png).2, 20, 20);
        assert!(
            near(pixel, master, 24),
            "{format:?}: {pixel:?} vs {master:?}"
        );
    }
}

#[test]
fn the_master_is_rgb_when_opaque_and_rgba_otherwise() {
    let opaque = png_of(
        32,
        32,
        png::ColorType::Rgb,
        png::BitDepth::Eight,
        false,
        |_, _| RED,
    );
    let png = timed(Format::Png, &opaque).unwrap().png;
    assert_eq!(png[25], 2, "colour type RGB");
    let clear = png_of(
        32,
        32,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        false,
        |x, _| {
            if x < 16 { [10, 20, 30, 0] } else { RED }
        },
    );
    let output = timed(Format::Png, &clear).unwrap();
    assert_eq!(output.png[25], 6, "colour type RGBA");
    let rgba = valid(&output);
    assert_eq!(at(&rgba, 10, 10), [0, 0, 0, 0], "no colour under alpha 0");
    // a master processed again gives itself back
    assert_eq!(timed(Format::Png, &output.png).unwrap().png, output.png);
}

#[test]
fn a_file_is_taken_only_as_what_its_bytes_are() {
    let samples = samples();
    for (declared, _) in &samples {
        for (actual, bytes) in &samples {
            if declared != actual {
                assert_eq!(
                    timed(*declared, bytes),
                    Err(Refusal::WrongType),
                    "{declared:?} as {actual:?}"
                );
            }
        }
    }
    for format in Format::ALL {
        assert_eq!(timed(format, b""), Err(Refusal::Unsupported), "{format:?}");
        assert_eq!(
            timed(format, b"<html><script>alert(1)</script></html>"),
            Err(Refusal::Unsupported)
        );
        assert_eq!(timed(format, b"%PDF-1.7\n"), Err(Refusal::Unsupported));
        assert_eq!(
            timed(format, &vec![0; format.max_bytes() + 1]),
            Err(Refusal::TooLarge)
        );
    }
    // gzip (an svgz) is no SVG
    assert_eq!(
        timed(Format::Svg, &[0x1f, 0x8b, 8, 0, 0, 0, 0, 0]),
        Err(Refusal::Unsupported)
    );
}

#[test]
fn sides_below_the_minimum_are_refused_and_tall_ones_cropped() {
    for (width, height) in [(1, 1), (15, 200), (300, 15)] {
        let file = png_of(
            width,
            height,
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            false,
            |_, _| RED,
        );
        assert_eq!(
            timed(Format::Png, &file),
            Err(Refusal::TooSmall),
            "{width}x{height}"
        );
    }
    for side in [16, 255, 256, 257, 511, 512, 1024] {
        let file = png_of(
            side,
            side,
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            false,
            quadrant(side, side),
        );
        let rgba = valid(&timed(Format::Png, &file).unwrap());
        assert_eq!(at(&rgba, 0, 0), RED, "{side}");
        assert_eq!(at(&rgba, 255, 255), BLUE, "{side}");
    }
    let tall = png_of(
        16,
        4000,
        png::ColorType::Rgb,
        png::BitDepth::Eight,
        false,
        |_, y| {
            if (1992..2008).contains(&y) { RED } else { BLUE }
        },
    );
    assert!(
        valid(&timed(Format::Png, &tall).unwrap())
            .chunks(4)
            .all(|p| p == RED)
    );
}
