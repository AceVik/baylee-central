//! PNG: every colour type and depth, interlacing, APNG, and the hostile
//! files of the proposal (§7.10): bombs in IDAT, zTXt and iCCP, huge
//! headers, broken checksums, every cut, metadata and trailing documents.
use crate::support::{
    assemble, at, chunk, chunks, ihdr, png_of, timed, valid, zero_bomb, zlib_stored,
};
use central_picture::{Format, Refusal};

const RED: [u8; 4] = [255, 0, 0, 255];

fn red_square() -> Vec<u8> {
    png_of(
        64,
        64,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        false,
        |_, _| RED,
    )
}

#[test]
fn every_colour_type_and_depth_is_taken() {
    use png::{BitDepth::*, ColorType::*};
    let cases = [
        (Grayscale, One),
        (Grayscale, Two),
        (Grayscale, Four),
        (Grayscale, Eight),
        (Grayscale, Sixteen),
        (GrayscaleAlpha, Eight),
        (GrayscaleAlpha, Sixteen),
        (Rgb, Eight),
        (Rgb, Sixteen),
        (Rgba, Eight),
        (Rgba, Sixteen),
        (Indexed, One),
        (Indexed, Two),
        (Indexed, Four),
        (Indexed, Eight),
    ];
    for (colour, depth) in cases {
        let file = png_of(40, 40, colour, depth, false, |x, _| {
            if x < 20 {
                [255, 255, 255, 255]
            } else {
                [0, 0, 0, 255]
            }
        });
        let rgba = valid(
            &timed(Format::Png, &file).unwrap_or_else(|r| panic!("{colour:?} {depth:?}: {r}")),
        );
        assert_eq!(rgba.len(), 256 * 256 * 4);
    }
}

#[test]
fn palettes_with_transparency_and_interlaced_files_are_taken() {
    let mut with_trns = Vec::new();
    for (kind, data) in chunks(&png_of(
        512,
        512,
        png::ColorType::Indexed,
        png::BitDepth::Eight,
        false,
        |x, _| [u8::try_from(x % 2).unwrap(), 0, 0, 255],
    )) {
        with_trns.push(chunk(&kind, &data));
        if &kind == b"PLTE" {
            // entry 0 fully transparent
            with_trns.push(chunk(b"tRNS", &[0]));
        }
    }
    let rgba = valid(&timed(Format::Png, &assemble(&with_trns)).unwrap());
    // half the pixels transparent: alpha about one half everywhere
    assert!(
        rgba.chunks(4).all(|p| (126..=130).contains(&p[3])),
        "{:?}",
        &rgba[..8]
    );
    // a tRNS longer than the palette: png 0.18.1 uses the entries the
    // palette has (measured); either way nothing panics and a picture is valid
    let mut bad_trns = with_trns.clone();
    let at_trns = bad_trns.iter().position(|c| &c[4..8] == b"tRNS").unwrap();
    bad_trns[at_trns] = chunk(b"tRNS", &[0; 300]);
    if let Ok(output) = timed(Format::Png, &assemble(&bad_trns)) {
        valid(&output);
    }
    // a palette image without PLTE
    let no_plte: Vec<Vec<u8>> = with_trns
        .iter()
        .filter(|c| &c[4..8] != b"PLTE" && &c[4..8] != b"tRNS")
        .cloned()
        .collect();
    assert_eq!(
        timed(Format::Png, &assemble(&no_plte)),
        Err(Refusal::Malformed)
    );
    for colour in [
        png::ColorType::Rgb,
        png::ColorType::Rgba,
        png::ColorType::Grayscale,
    ] {
        let interlaced = png_of(
            33,
            33,
            colour,
            png::BitDepth::Eight,
            true,
            crate::support::quadrant(33, 33),
        );
        assert_eq!(interlaced[28], 1, "interlace flag");
        let rgba = valid(&timed(Format::Png, &interlaced).unwrap());
        let plain = valid(
            &timed(
                Format::Png,
                &png_of(
                    33,
                    33,
                    colour,
                    png::BitDepth::Eight,
                    false,
                    crate::support::quadrant(33, 33),
                ),
            )
            .unwrap(),
        );
        assert_eq!(rgba, plain, "{colour:?}");
    }
}

#[test]
fn headers_that_claim_too_much_are_refused_before_decoding() {
    for (width, height) in [
        (100_000, 100_000),
        (8_193, 16),
        (4_097, 4_097),
        (u32::MAX, 1),
    ] {
        let file = assemble(&[
            chunk(b"IHDR", &ihdr(width, height, 8, 6, 0)),
            chunk(b"IDAT", &zlib_stored(&[0; 10])),
            chunk(b"IEND", b""),
        ]);
        assert_eq!(
            timed(Format::Png, &file),
            Err(Refusal::TooManyPixels),
            "{width}x{height}"
        );
    }
    // IHDR not first, or not 13 bytes long
    let mut text_first = vec![chunk(b"tEXt", b"a\0b")];
    text_first.extend(
        chunks(&red_square())
            .into_iter()
            .map(|(k, d)| chunk(&k, &d)),
    );
    assert_eq!(
        timed(Format::Png, &assemble(&text_first)),
        Err(Refusal::Malformed)
    );
    let long = assemble(&[
        chunk(b"IHDR", &[ihdr(64, 64, 8, 6, 0), vec![0]].concat()),
        chunk(b"IEND", b""),
    ]);
    assert_eq!(timed(Format::Png, &long), Err(Refusal::Malformed));
    // an invalid depth and colour pair, compression or filter other than 0, interlace above 1
    for header in [
        ihdr(64, 64, 3, 2, 0),
        ihdr(64, 64, 16, 3, 0),
        ihdr(64, 64, 8, 6, 2),
    ] {
        let file = assemble(&[
            chunk(b"IHDR", &header),
            chunk(b"IDAT", &zlib_stored(&[0; 10])),
            chunk(b"IEND", b""),
        ]);
        assert_eq!(
            timed(Format::Png, &file),
            Err(Refusal::Malformed),
            "{header:?}"
        );
    }
    let mut compression = ihdr(64, 64, 8, 6, 0);
    compression[10] = 1;
    let file = assemble(&[chunk(b"IHDR", &compression), chunk(b"IEND", b"")]);
    assert_eq!(timed(Format::Png, &file), Err(Refusal::Malformed));
}

#[test]
fn bombs_in_idat_ztxt_and_iccp_stay_bounded() {
    // About 67 MB of zeros in a stream of a few tens of kilobytes.
    let bomb = zero_bomb(8_192, 8_192);
    assert!(bomb.len() < 200_000, "{}", bomb.len());
    // IDAT that inflates far past a 1024² picture's size
    let idat = assemble(&[
        chunk(b"IHDR", &ihdr(1_024, 1_024, 8, 0, 0)),
        chunk(b"IDAT", &bomb),
        chunk(b"IEND", b""),
    ]);
    let result = timed(Format::Png, &idat);
    assert!(
        matches!(result, Ok(_) | Err(Refusal::Malformed)),
        "{result:?}"
    );
    // the same stream as a text chunk and as a colour profile: skipped unread
    let plain = timed(Format::Png, &red_square()).unwrap();
    for (kind, prefix) in [
        (b"zTXt", b"bomb\0\0".as_slice()),
        (b"iCCP", b"bomb\0\0".as_slice()),
    ] {
        let mut parts: Vec<Vec<u8>> = chunks(&red_square())
            .into_iter()
            .map(|(k, d)| chunk(&k, &d))
            .collect();
        parts.insert(1, chunk(kind, &[prefix, &bomb].concat()));
        assert_eq!(
            timed(Format::Png, &assemble(&parts)),
            Ok(plain.clone()),
            "{kind:?}"
        );
    }
}

#[test]
fn metadata_scripts_and_trailing_documents_never_reach_the_output() {
    let plain = timed(Format::Png, &red_square()).unwrap();
    let mut parts: Vec<Vec<u8>> = chunks(&red_square())
        .into_iter()
        .map(|(k, d)| chunk(&k, &d))
        .collect();
    parts.insert(1, chunk(b"eXIf", b"MM\0*\0\0\0\x08\0\0"));
    parts.insert(1, chunk(b"tEXt", b"Comment\0<script>alert(1)</script>"));
    parts.insert(
        1,
        chunk(
            b"iTXt",
            b"XML:com.adobe.xmp\0\0\0\0\0<x:xmpmeta>GPS</x:xmpmeta>",
        ),
    );
    parts.insert(1, chunk(b"sRGB", &[0]));
    parts.insert(1, chunk(b"pHYs", &[0, 0, 11, 19, 0, 0, 11, 19, 1]));
    let decorated = assemble(&parts);
    assert_eq!(timed(Format::Png, &decorated), Ok(plain.clone()));
    // a polyglot: a PNG followed by an HTML document and a ZIP's end
    let polyglot = [
        red_square(),
        b"<html><script>alert(1)</script></html>PK\x05\x06".to_vec(),
    ]
    .concat();
    assert_eq!(timed(Format::Png, &polyglot), Ok(plain.clone()));
    let output = timed(Format::Png, &polyglot).unwrap();
    for needle in [b"script".as_slice(), b"html", b"GPS", b"MM\0*"] {
        assert!(!output.png.windows(needle.len()).any(|w| w == needle));
        assert!(!output.avif.windows(needle.len()).any(|w| w == needle));
    }
}

#[test]
fn broken_checksums_and_every_cut_are_refused() {
    let file = red_square();
    let at_idat = file.windows(4).position(|w| w == b"IDAT").unwrap();
    let mut crc = file.clone();
    let crc_at = at_idat
        + 4
        + usize::try_from(u32::from_be_bytes(
            file[at_idat - 4..at_idat].try_into().unwrap(),
        ))
        .unwrap();
    crc[crc_at] ^= 1;
    assert_eq!(timed(Format::Png, &crc), Err(Refusal::Malformed));
    // a damaged zlib header and a damaged Adler-32 (CRC corrected, so the
    // damage reaches the inflater)
    let parts = chunks(&file);
    let idat = parts.iter().find(|(k, _)| k == b"IDAT").unwrap().1.clone();
    for damage in [0, idat.len() - 1] {
        let mut broken = idat.clone();
        broken[damage] ^= 0x55;
        let rebuilt: Vec<Vec<u8>> = parts
            .iter()
            .map(|(k, d)| {
                if k == b"IDAT" {
                    chunk(k, &broken)
                } else {
                    chunk(k, d)
                }
            })
            .collect();
        assert_eq!(
            timed(Format::Png, &assemble(&rebuilt)),
            Err(Refusal::Malformed),
            "byte {damage}"
        );
    }
    // too little image data
    let short = assemble(&[
        chunk(b"IHDR", &ihdr(64, 64, 8, 6, 0)),
        chunk(b"IDAT", &zlib_stored(&[0; 100])),
        chunk(b"IEND", b""),
    ]);
    assert_eq!(timed(Format::Png, &short), Err(Refusal::Malformed));
    // Every cut before the image data is whole is refused; a cut in the
    // last CRC or IEND may still give the picture, since the decoder stops
    // once the zlib stream (with its Adler-32) is complete.
    let data_end = at_idat + 4 + idat.len();
    for end in 0..data_end {
        assert!(timed(Format::Png, &file[..end]).is_err(), "cut at {end}");
    }
    for end in data_end..file.len() {
        if let Ok(output) = timed(Format::Png, &file[..end]) {
            assert_eq!(output, timed(Format::Png, &file).unwrap());
        }
    }
}

#[test]
fn an_animated_png_gives_its_default_image() {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 32, 32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_animated(2, 0).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&RED.repeat(32 * 32)).unwrap();
        writer
            .write_image_data(&[0, 0, 255, 255].repeat(32 * 32))
            .unwrap();
        writer.finish().unwrap();
    }
    assert!(out.windows(4).any(|w| w == b"acTL"));
    let rgba = valid(&timed(Format::Png, &out).unwrap());
    assert_eq!(at(&rgba, 128, 128), RED);
}
