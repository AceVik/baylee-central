//! The formats Central accepts as an upload, by declared type and by their
//! own first bytes. A file is taken only when both agree: the declared type
//! alone (or a file name) never chooses a decoder.

/// Largest upload of any format. A raster file may be this large; an SVG or
/// an ICO much less ([`Format::max_bytes`]).
pub const MAX_UPLOAD: usize = 8 * 1024 * 1024;

/// An accepted picture format. The browser's own PNG (design A) is
/// [`Format::Png`] like any other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// PNG, APNG's default image included.
    Png,
    /// JPEG (JFIF or EXIF), baseline or progressive.
    Jpeg,
    /// GIF 87a or 89a; an animation's first frame.
    Gif,
    /// WebP, lossy, lossless or extended; an animation's first frame.
    Webp,
    /// AVIF (HEIF with AV1); the primary item.
    Avif,
    /// SVG, rasterised; never served as SVG.
    Svg,
    /// Windows bitmap.
    Bmp,
    /// Windows icon (not a cursor); its largest entry.
    Ico,
}

impl Format {
    /// Every format, in protocol order.
    pub const ALL: [Self; 8] = [
        Self::Png,
        Self::Jpeg,
        Self::Gif,
        Self::Webp,
        Self::Avif,
        Self::Svg,
        Self::Bmp,
        Self::Ico,
    ];

    /// The format a request's `Content-Type` names: exactly one of the
    /// media types, without parameters.
    #[must_use]
    pub fn from_content_type(value: &str) -> Option<Self> {
        Some(match value {
            "image/png" => Self::Png,
            "image/jpeg" => Self::Jpeg,
            "image/gif" => Self::Gif,
            "image/webp" => Self::Webp,
            "image/avif" => Self::Avif,
            "image/svg+xml" => Self::Svg,
            "image/bmp" => Self::Bmp,
            "image/x-icon" | "image/vnd.microsoft.icon" => Self::Ico,
            _ => return None,
        })
    }

    /// The format's media type.
    #[must_use]
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Avif => "image/avif",
            Self::Svg => "image/svg+xml",
            Self::Bmp => "image/bmp",
            Self::Ico => "image/x-icon",
        }
    }

    /// Its byte in the worker protocol.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Png => 1,
            Self::Jpeg => 2,
            Self::Gif => 3,
            Self::Webp => 4,
            Self::Avif => 5,
            Self::Svg => 6,
            Self::Bmp => 7,
            Self::Ico => 8,
        }
    }

    /// The format with protocol byte `code`.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|format| format.code() == code)
    }

    /// The largest file of this format Central reads. An SVG is text that a
    /// renderer works through and an ICO holds pictures of at most 256 px,
    /// so both are kept small; a photo may be large.
    #[must_use]
    pub const fn max_bytes(self) -> usize {
        match self {
            Self::Svg | Self::Ico => 1024 * 1024,
            _ => MAX_UPLOAD,
        }
    }

    /// The format `bytes` begin as, from their own first bytes alone.
    #[must_use]
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            Some(Self::Jpeg)
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some(Self::Gif)
        } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(Self::Webp)
        } else if avif_brand(bytes) {
            Some(Self::Avif)
        } else if bytes.starts_with(b"BM") && bytes.len() >= 26 {
            Some(Self::Bmp)
        } else if icon_directory(bytes) {
            Some(Self::Ico)
        } else if svg_root(bytes) {
            Some(Self::Svg)
        } else {
            None
        }
    }
}

/// An ISO-BMFF file whose `ftyp` names AVIF, as its major brand or among
/// the compatible ones.
fn avif_brand(bytes: &[u8]) -> bool {
    let Some(size) = bytes
        .get(..4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_be_bytes)
    else {
        return false;
    };
    let size = usize::try_from(size).unwrap_or(usize::MAX);
    if bytes.get(4..8) != Some(b"ftyp") || !(16..=256).contains(&size) || size > bytes.len() {
        return false;
    }
    let major = &bytes[8..12];
    major == b"avif"
        || major == b"avis"
        || bytes[16..size]
            .as_chunks::<4>()
            .0
            .iter()
            .any(|brand| brand == b"avif")
}

/// An icon directory (type 1, not a cursor) of 1 to 255 entries, all of
/// whose 16-byte records are there.
fn icon_directory(bytes: &[u8]) -> bool {
    let (Some(head), Some(count)) = (bytes.get(..4), bytes.get(4..6)) else {
        return false;
    };
    let count = usize::from(u16::from_le_bytes([count[0], count[1]]));
    head == [0, 0, 1, 0] && (1..=255).contains(&count) && bytes.len() >= 6 + 16 * count
}

/// Text whose first element, after an optional byte order mark, white
/// space, an XML declaration, comments, processing instructions and a
/// document type, is `<svg`. Only the first 64 KiB are looked at.
fn svg_root(bytes: &[u8]) -> bool {
    let mut rest = &bytes[..bytes.len().min(64 * 1024)];
    rest = rest.strip_prefix(b"\xef\xbb\xbf").unwrap_or(rest);
    loop {
        rest = trim_start(rest);
        if rest.starts_with(b"<?") {
            rest = after(rest, b"?>");
        } else if rest.starts_with(b"<!--") {
            rest = after(rest, b"-->");
        } else if rest.starts_with(b"<!DOCTYPE") {
            // An internal subset in brackets ends with `]>`.
            let close = rest.iter().position(|&b| b == b'>');
            let open = rest.iter().position(|&b| b == b'[');
            rest = match (open, close) {
                (Some(open), Some(close)) if open < close => after(rest, b"]>"),
                _ => after(rest, b">"),
            };
        } else {
            break;
        }
        if rest.is_empty() {
            return false;
        }
    }
    rest.starts_with(b"<svg")
        && rest
            .get(4)
            .is_some_and(|b| b.is_ascii_whitespace() || matches!(b, b'>' | b'/'))
}

fn trim_start(bytes: &[u8]) -> &[u8] {
    let skip = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[skip..]
}

/// What follows the first `end` in `bytes`, or nothing.
fn after<'a>(bytes: &'a [u8], end: &[u8]) -> &'a [u8] {
    bytes
        .windows(end.len())
        .position(|window| window == end)
        .map_or(&[], |at| &bytes[at + end.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_types_and_codes_round_trip() {
        for format in Format::ALL {
            assert_eq!(
                Format::from_content_type(format.content_type()),
                Some(format)
            );
            assert_eq!(Format::from_code(format.code()), Some(format));
        }
        assert_eq!(
            Format::from_content_type("image/vnd.microsoft.icon"),
            Some(Format::Ico)
        );
        for other in [
            "image/png; charset=utf-8",
            "IMAGE/PNG",
            "image/heic",
            "image/svg",
            "text/html",
            "",
        ] {
            assert_eq!(Format::from_content_type(other), None, "{other}");
        }
        assert_eq!(Format::from_code(0), None);
        assert_eq!(Format::from_code(9), None);
    }

    #[test]
    fn the_first_bytes_decide() {
        const ICON: [u8; 22] = [
            0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let avif = [
            &[0, 0, 0, 0x1c][..],
            b"ftypavif",
            &[0, 0, 0, 0],
            b"avifmif1miaf",
        ]
        .concat();
        let cases: [(&[u8], Option<Format>); 17] = [
            (b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR", Some(Format::Png)),
            (&[0xff, 0xd8, 0xff, 0xe0], Some(Format::Jpeg)),
            (b"GIF89a\x01\0\x01\0", Some(Format::Gif)),
            (b"GIF87a", Some(Format::Gif)),
            (b"RIFF\0\0\0\0WEBPVP8 ", Some(Format::Webp)),
            (&avif, Some(Format::Avif)),
            (b"BM\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", Some(Format::Bmp)),
            (&ICON, Some(Format::Ico)),
            (b"<svg xmlns='http://www.w3.org/2000/svg'/>", Some(Format::Svg)),
            (
                b"\xef\xbb\xbf <?xml version='1.0'?>\n<!-- a -->\n<!DOCTYPE svg [<!ENTITY a 'b'>]>\n<svg>",
                Some(Format::Svg),
            ),
            // a cursor is not an icon, nor a directory without its records
            (&[0, 0, 2, 0, 1, 0], None),
            (&[0, 0, 1, 0, 1, 0], None),
            (b"<html><body><svg></svg></body></html>", None),
            (b"<svgx/>", None),
            (b"<!-- never closed <svg>", None),
            (b"%PDF-1.7", None),
            (b"", None),
        ];
        for (bytes, expected) in cases {
            assert_eq!(Format::sniff(bytes), expected, "{bytes:?}");
        }
    }

    #[test]
    fn an_iso_file_that_is_not_avif_is_nothing() {
        let heic = [
            &[0, 0, 0, 0x18][..],
            b"ftypheic",
            &[0, 0, 0, 0],
            b"mif1heic",
        ]
        .concat();
        assert_eq!(Format::sniff(&heic), None);
        // a size beyond the file, and one too small for a brand
        let long = [&[0, 0, 1, 0][..], b"ftypavif", &[0, 0, 0, 0]].concat();
        assert_eq!(Format::sniff(&long), None);
        let short = [&[0, 0, 0, 8][..], b"ftypavif"].concat();
        assert_eq!(Format::sniff(&short), None);
    }

    #[test]
    fn small_formats_have_small_caps() {
        assert_eq!(Format::Svg.max_bytes(), 1 << 20);
        assert_eq!(Format::Ico.max_bytes(), 1 << 20);
        assert_eq!(Format::Jpeg.max_bytes(), MAX_UPLOAD);
    }

    #[test]
    fn a_vector_s_root_is_found_only_as_its_first_element_and_early() {
        let svg = |text: &str| Format::sniff(text.as_bytes()) == Some(Format::Svg);
        for text in [
            "<svg/>",
            "<svg>",
            "<svg\n>",
            "\t\r\n <svg >",
            "<?xml version='1.0'?><?x y?><!-- <html> --><svg>",
            "<!-- a --><!-- b --><svg>",
            "<!DOCTYPE svg><svg>",
            "<!DOCTYPE svg PUBLIC '-//W3C//DTD SVG 1.1//EN' 'x'><svg>",
            "<!DOCTYPE svg [<!ENTITY a 'b'><!ENTITY c 'd'>]><svg>",
        ] {
            assert!(svg(text), "{text}");
        }
        for text in [
            "<svg",
            "<SVG>",
            "<svg:svg>",
            "<svg\0>",
            "x<svg>",
            "<html><svg>",
            "<script/><svg>",
            "<!-- <svg>",
            "<?xml <svg>",
            "<!DOCTYPE svg [<!ENTITY a 'b'> <svg>",
            "<!DOCTYPE html",
            "<!doctype svg><svg>",
            "\u{feff}\u{feff}<svg>",
        ] {
            assert!(!svg(text), "{text}");
        }
        // The root is looked for in the first 64 KiB, and not behind them.
        let padded = |length: usize| format!("{}<svg>", " ".repeat(length));
        assert!(svg(&padded(64 * 1024 - 5)));
        assert!(!svg(&padded(64 * 1024 - 4)));
        assert!(!svg(&padded(64 * 1024)));
    }

    #[test]
    fn an_icon_has_all_the_records_it_counts() {
        let icon = |count: u16, records: usize| {
            let mut bytes = vec![0, 0, 1, 0];
            bytes.extend(count.to_le_bytes());
            bytes.resize(6 + 16 * records, 0);
            Format::sniff(&bytes) == Some(Format::Ico)
        };
        assert!(icon(1, 1) && icon(255, 255));
        assert!(!icon(0, 0) && !icon(0, 1));
        assert!(!icon(2, 1) && !icon(255, 254));
        assert!(!icon(256, 256) && !icon(u16::MAX, 255));
    }

    #[test]
    fn an_avif_brand_stands_inside_its_own_box() {
        let file = |size: u32, major: &[u8], brands: &[u8], tail: &[u8]| {
            [
                &size.to_be_bytes()[..],
                b"ftyp",
                major,
                &[0; 4],
                brands,
                tail,
            ]
            .concat()
        };
        let avif = |bytes: &[u8]| Format::sniff(bytes) == Some(Format::Avif);
        assert!(avif(&file(16, b"avif", b"", b"")));
        assert!(avif(&file(16, b"avis", b"", b"")));
        assert!(avif(&file(24, b"mif1", b"miafavif", b"")));
        assert!(!avif(&file(24, b"mif1", b"miafmif1", b"")));
        // Behind the box, or across two brands, it is not one of them.
        assert!(!avif(&file(20, b"mif1", b"miaf", b"avif")));
        assert!(!avif(&file(24, b"mif1", b"xxavifxx", b"")));
        // The box is sixteen to 256 octets and wholly there.
        let brands = b"miaf".repeat(60);
        assert!(avif(&file(256, b"avif", &brands, b"")));
        assert!(!avif(&file(260, b"avif", &brands, &[0; 64])));
        assert!(!avif(&file(15, b"avif", b"", b"")));
        assert!(!avif(&file(17, b"avif", b"", b"")));
        for size in [0, 1, u32::MAX] {
            assert!(!avif(&file(size, b"avif", &brands, b"")), "{size}");
        }
    }
}
