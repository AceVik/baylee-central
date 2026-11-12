//! The worker binary behind the service's own client (`central_picture::
//! Worker`): a fresh process per picture, the same refusals as the codec,
//! the digest computed by the client, and a broken request refused.
use crate::formats::samples;
use crate::support::{gif_of, timed};
use central_picture::{Format, Refusal, Worker};
use std::io::Write;
use std::process::{Command, Stdio};

fn worker() -> Worker {
    crate::support::worker_without_ffmpeg()
}

#[test]
fn the_worker_converts_every_format_as_the_codec_does() {
    for (format, bytes) in samples() {
        let converted = worker()
            .convert(format, &bytes)
            .unwrap_or_else(|r| panic!("{format:?}: {r}"));
        let direct = timed(format, &bytes).unwrap();
        assert_eq!(
            (converted.avif.clone(), converted.png.clone()),
            (direct.avif, direct.png)
        );
        assert_eq!(
            converted.sha256,
            central_picture::digest(&converted.png, converted.sequence())
        );
    }
}

#[test]
fn the_worker_refuses_what_the_codec_refuses() {
    let mut huge = gif_of(64, 64, 1);
    huge[6..10].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
    assert_eq!(
        worker().convert(Format::Gif, &huge),
        Err(Refusal::TooManyPixels)
    );
    let cut = gif_of(64, 64, 1);
    assert!(
        worker()
            .convert(Format::Gif, &cut[..cut.len() / 2])
            .is_err()
    );
}

/// The binary itself, with a raw request: what it answers and how it exits.
fn raw(request: &[u8]) -> (Vec<u8>, bool) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_central-picture-worker"))
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let request = request.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&request);
    });
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    // it writes nothing but its answer: no message of a decoder or a panic
    assert!(
        output.stderr.is_empty(),
        "{:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    (output.stdout, output.status.success())
}

#[test]
fn a_broken_request_is_answered_with_a_refusal_and_nothing_else() {
    let good = central_picture::request(Format::Gif, &gif_of(32, 32, 1)).unwrap();
    let (answer, ok) = raw(&good);
    assert!(ok);
    assert!(matches!(central_picture::read_answer(&answer), Some(Ok(_))));
    let cases: [(&[u8], Refusal); 4] = [
        (b"", Refusal::Malformed),
        (&good[..good.len() - 1], Refusal::Malformed),
        (&[good.as_slice(), b"more"].concat(), Refusal::Malformed),
        (b"BCPQ\x02\x06\xff\xff\xff\xff", Refusal::TooLarge),
    ];
    for (request, refusal) in cases {
        let (answer, ok) = raw(request);
        assert!(ok);
        assert_eq!(central_picture::read_answer(&answer), Some(Err(refusal)));
    }
    // a body of another format than declared is refused by the worker too
    let declared_png = central_picture::request(Format::Png, &gif_of(32, 32, 1)).unwrap();
    assert_eq!(
        central_picture::read_answer(&raw(&declared_png).0),
        Some(Err(Refusal::WrongType))
    );
}

#[test]
fn heavy_svgs_end_within_the_worker_s_time_and_the_next_picture_converts() {
    // Filters and patterns that cost a renderer minutes, a self-referring
    // use and mask: the worker answers or is ended at its deadline, and
    // either way the next conversion works.
    let bodies = [
        "<filter id='f' x='-1000000%' y='-1000000%' width='2000000%' height='2000000%'>\
         <feGaussianBlur stdDeviation='1000000'/></filter><rect width='100' height='100' filter='url(#f)'/>",
        "<filter id='f'><feMorphology radius='100000'/><feTurbulence baseFrequency='0.9' numOctaves='100000'/></filter>\
         <rect width='100' height='100' filter='url(#f)'/>",
        "<g id='a'><use href='#a'/></g>",
        "<pattern id='p' width='0.0001' height='0.0001' patternUnits='userSpaceOnUse'><rect width='1' height='1'/></pattern>\
         <rect width='100' height='100' fill='url(#p)'/>",
        "<mask id='m'><rect width='100' height='100' fill='white' mask='url(#m)'/></mask><rect width='9' height='9' mask='url(#m)'/>",
    ];
    let runs: Vec<_> = bodies
        .iter()
        .map(|body| {
            let file = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>{body}</svg>"
            );
            std::thread::spawn(move || {
                let started = std::time::Instant::now();
                let result = worker().convert(Format::Svg, file.as_bytes());
                (result.map(|_| ()), started.elapsed())
            })
        })
        .collect();
    for (run, body) in runs.into_iter().zip(bodies) {
        let (result, took) = run.join().unwrap();
        assert!(
            took < central_picture::TIME + std::time::Duration::from_secs(3),
            "{body}: {took:?}"
        );
        assert!(
            matches!(result, Ok(()) | Err(Refusal::Failed | Refusal::Malformed)),
            "{body}: {result:?}"
        );
    }
    assert!(worker().convert(Format::Gif, &gif_of(32, 32, 1)).is_ok());
}
