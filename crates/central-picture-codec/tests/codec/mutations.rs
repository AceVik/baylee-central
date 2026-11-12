//! Seeded mutations of a valid file of every format (§7.10, "mutation
//! testing without a new dependency"): bit flips, insertions, deletions,
//! overwritten lengths and spliced runs, with PNG CRCs corrected so the
//! damage reaches the decoder. Each case ends in a refusal or a valid pair
//! within the time bound, never a panic. `CENTRAL_PICTURE_MUTATIONS` sets
//! the cases per format (default 120); the fuzz targets in `fuzz/` go
//! further.
use crate::formats::samples;
use crate::support::{Xorshift, chunk, chunks, through_worker, timed, valid};
use central_picture::Format;

fn cases() -> usize {
    std::env::var("CENTRAL_PICTURE_MUTATIONS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(120)
}

fn mutate(rng: &mut Xorshift, seed: &[u8]) -> Vec<u8> {
    let mut bytes = seed.to_vec();
    for _ in 0..=rng.below(4) {
        let at = rng.below(bytes.len());
        match rng.below(6) {
            0 => bytes[at] ^= 1 << rng.below(8),
            1 => bytes[at] = u8::try_from(rng.below(256)).unwrap(),
            2 => bytes.insert(at, u8::try_from(rng.below(256)).unwrap()),
            3 if bytes.len() > 16 => {
                let end = (at + 1 + rng.below(16)).min(bytes.len());
                bytes.drain(at..end);
            }
            4 => {
                // a length-like field: four bytes of extremes
                let value = [0_u32, 1, 0x7fff_ffff, u32::MAX, 0x0001_0000][rng.below(5)];
                let end = (at + 4).min(bytes.len());
                let len = end - at;
                bytes[at..end].copy_from_slice(&value.to_be_bytes()[..len]);
            }
            _ => {
                // repeat a run of the file inside itself
                let len = rng.below(64).min(bytes.len() - at);
                let run = bytes[at..at + len].to_vec();
                let to = rng.below(bytes.len());
                bytes.splice(to..to, run);
            }
        }
        if bytes.is_empty() {
            bytes.push(0);
        }
    }
    bytes
}

/// A PNG's chunks with their CRCs made right again.
fn recrc(png: &[u8]) -> Vec<u8> {
    if png.len() < 8 || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return png.to_vec();
    }
    let mut out = png[..8].to_vec();
    let mut rest = &png[8..];
    while rest.len() >= 12 {
        let length = usize::try_from(u32::from_be_bytes(rest[..4].try_into().unwrap()))
            .unwrap_or(usize::MAX);
        if length > rest.len() - 12 {
            break;
        }
        out.extend_from_slice(&chunk(&rest[4..8], &rest[8..8 + length]));
        rest = &rest[12 + length..];
    }
    out.extend_from_slice(rest);
    out
}

#[test]
fn mutated_files_of_every_format_never_panic_and_stay_bounded() {
    let mut accepted = 0;
    let mut refused = 0;
    for (n, (format, seed)) in samples().into_iter().enumerate() {
        let mut rng = Xorshift(0x9e37_79b9_7f4a_7c15 ^ u64::try_from(n + 1).unwrap());
        for _ in 0..cases() {
            let mut bytes = mutate(&mut rng, &seed);
            if format == Format::Png {
                bytes = recrc(&bytes);
            }
            // AVIF through the worker, since rav1d may abort the process
            let result = if format == Format::Avif {
                through_worker(format, &bytes)
            } else {
                timed(format, &bytes)
            };
            match result {
                Ok(output) => {
                    valid(&output);
                    accepted += 1;
                }
                Err(_) => refused += 1,
            }
        }
    }
    // both outcomes occur: the mutations reach the decoders and are not all
    // stopped at the first byte
    assert!(
        accepted > 0 && refused > 0,
        "{accepted} accepted, {refused} refused"
    );
    assert!(chunks(&samples()[0].1).len() >= 3);
}
