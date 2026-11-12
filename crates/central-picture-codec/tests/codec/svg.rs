//! SVG: rasterised only, without scripts, without loading anything, within
//! bounds for entities, nesting, element counts and filters.
use crate::support::{assemble, at, chunk, ihdr, timed, valid, zlib_stored};
use central_picture::{Format, Refusal};

const GREEN: [u8; 4] = [0, 128, 0, 255];

fn svg(body: &str) -> Vec<u8> {
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' \
         viewBox='0 0 100 100' width='100' height='100'>{body}</svg>"
    )
    .into_bytes()
}

fn green() -> Vec<u8> {
    timed(
        Format::Svg,
        &svg("<rect width='100' height='100' fill='green'/>"),
    )
    .unwrap()
    .png
}

#[test]
fn an_svg_is_drawn_as_its_centred_square() {
    let wide = b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 300 100'>\
        <rect width='300' height='100' fill='red'/><rect x='100' width='100' height='100' fill='green'/></svg>";
    let rgba = valid(&timed(Format::Svg, wide).unwrap());
    assert!(rgba.chunks(4).all(|p| p == GREEN), "{:?}", at(&rgba, 0, 0));
    // a tiny or a huge view box is drawn at 256 all the same
    for view in ["0 0 0.001 0.001", "0 0 1000000000 1000000000"] {
        let file = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='{view}'><rect width='100%' height='100%' fill='green'/></svg>"
        );
        let rgba = valid(&timed(Format::Svg, file.as_bytes()).unwrap());
        assert_eq!(at(&rgba, 128, 128), GREEN, "{view}");
    }
    for bad in [
        "<svg xmlns='http://www.w3.org/2000/svg' width='0' height='0'/>",
        "<svg><rect",
    ] {
        assert!(timed(Format::Svg, bad.as_bytes()).is_err(), "{bad}");
    }
}

#[test]
fn scripts_events_and_foreign_content_change_nothing() {
    let plain = green();
    for extra in [
        "<script>alert(1)</script>",
        "<script xlink:href='https://example.invalid/x.js'/>",
        "<rect width='1' height='1' fill='none' onload='alert(1)'/>",
        "<foreignObject width='100' height='100'><div xmlns='http://www.w3.org/1999/xhtml'>x</div></foreignObject>",
        "<text x='10' y='50' font-size='40'>Hello</text>",
        "<style>@import url('https://example.invalid/a.css'); rect { fill: green }</style>",
        "<a xlink:href='javascript:alert(1)'><rect width='1' height='1' fill='none'/></a>",
    ] {
        let file = svg(&format!(
            "<rect width='100' height='100' fill='green'/>{extra}"
        ));
        assert_eq!(timed(Format::Svg, &file).unwrap().png, plain, "{extra}");
    }
}

#[test]
fn no_reference_is_loaded_from_files_or_the_network() {
    // A real file beside the test, which a resolver that read files would
    // draw: a PNG of solid red.
    let dir = std::env::temp_dir().join(format!("central-picture-svg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let red = dir.join("red.png");
    let rows: Vec<u8> = (0..16)
        .flat_map(|_| [vec![0], [255, 0, 0].repeat(16)].concat())
        .collect();
    std::fs::write(
        &red,
        assemble(&[
            chunk(b"IHDR", &ihdr(16, 16, 8, 2, 0)),
            chunk(b"IDAT", &zlib_stored(&rows)),
            chunk(b"IEND", b""),
        ]),
    )
    .unwrap();
    let path = red.to_string_lossy().into_owned();
    let plain = green();
    for href in [
        format!("file://{path}"),
        path.clone(),
        "red.png".to_owned(),
        "https://example.invalid/red.png".to_owned(),
        "data:image/png;base64,iVBORw0KGgo=".to_owned(),
        "#self".to_owned(),
    ] {
        let file = svg(&format!(
            "<rect width='100' height='100' fill='green'/>\
             <image id='self' width='100' height='100' href='{href}'/>\
             <image width='100' height='100' xlink:href='{href}'/>\
             <use href='{href}#frag'/>"
        ));
        assert_eq!(timed(Format::Svg, &file).unwrap().png, plain, "{href}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn entity_bombs_deep_nesting_and_crowds_are_bounded() {
    let laughs = format!(
        "<?xml version='1.0'?><!DOCTYPE svg [<!ENTITY a 'lollollollollollollollollollol'>{}]>\
         <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10'><text>&j;</text></svg>",
        (b'b'..=b'j').fold(String::new(), |mut out, c| {
            let prev = char::from(c - 1);
            let refs = format!("&{prev};").repeat(10);
            out.push_str("<!ENTITY ");
            out.push(char::from(c));
            out.push_str(" '");
            out.push_str(&refs);
            out.push_str("'>");
            out
        })
    );
    assert!(timed(Format::Svg, laughs.as_bytes()).is_err());
    let deep = svg(&format!(
        "{}<rect width='1' height='1'/>{}",
        "<g>".repeat(10_000),
        "</g>".repeat(10_000)
    ));
    assert_eq!(timed(Format::Svg, &deep), Err(Refusal::TooManyPixels));
    let crowd = svg(&"<rect width='1' height='1'/>".repeat(30_000));
    assert_eq!(timed(Format::Svg, &crowd), Err(Refusal::TooManyPixels));
    // nesting hidden from the scan in an entity: the parser's own bound
    let hidden = format!(
        "<!DOCTYPE svg [<!ENTITY g '{}'>]><svg xmlns='http://www.w3.org/2000/svg'>&g;&g;&g;</svg>",
        "<g>".repeat(200) + &"</g>".repeat(200)
    );
    let _ = timed(Format::Svg, hidden.as_bytes());
}
