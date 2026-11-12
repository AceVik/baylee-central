# Fuzzing the picture worker's decoders

`picture-fuzz` mutates seed pictures and hands every mutation to one
decoder entry point of the picture worker (`docs/profile-pictures.md`):
`central_picture_codec::convert` for PNG, JPEG, GIF, WebP, SVG, BMP and
ICO, and the worker binary itself for AVIF, since `rav1d` may abort the
process on damaged AV1.

## What it is, and what it is not

- **Seeded mutation, not coverage-guided.** Each case flips, sets, inserts,
  deletes or copies bytes, writes edge values (0, 1, 2³¹ − 1, 2³² − 1) or
  cuts the file. A mutation that converts joins the seeds, up to 256, so
  the search wanders deeper than the seeds alone. PNG chunks get their
  CRCs made right, so damage reaches the inflater and the decoders.
- **Why not libFuzzer.** `cargo fuzz` needs a foreign-function entry point
  and macros that expand to `unsafe` code, which AGENTS.md forbids in
  first-party code. This crate has `#![forbid(unsafe_code)]` and links
  nothing new.
- **Outside the workspace.** Its own `[workspace]`, its own lock and only
  path dependencies on the workspace's crates, so the workspace's graph
  and the gate are unchanged. Its seeds come from the encoders already in
  the graph and from the codec's committed fixtures.
- **What counts as a finding:**
  - a panic that escapes `convert`;
  - a case that takes longer than 9 s;
  - a pair that does not pass the service's own checks (`png_master`,
    `avif_framing`).

  Each one is written to `PICTURE_FINDINGS` (default `findings/`). A
  refusal is no finding; the run counts them by name.

## Running it

```sh
cargo build --release -p central-picture-codec --bin central-picture-worker
cargo build --release --manifest-path fuzz/Cargo.toml --target-dir target/fuzz
PICTURE_WORKER=target/release/central-picture-worker \
  target/fuzz/release/picture-fuzz png 120   # FORMAT SECONDS [SEED]
```

The release profile keeps overflow checks and debug assertions on. The
seed defaults to a fixed number per format, so a run can be repeated.

## What ran

All on this repository's development Mac (Apple Silicon), release build,
at load averages of about 11 to 30. No case found anything.

| Run, 12.11.2026 | PNG | JPEG | GIF | WebP | AVIF | SVG | BMP | ICO |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Before animations, 120 s each | 3,257 | 8,799 | 4,190 | 9,908 | 9,388 | 43,483 | 704 | 3,303 |
| With animation decoding, 60 s each | 1,303 | 3,194 | 2,703 | 4,418 | 4,212 | 19,654 | 741 | 5,837 |
| Animated seeds (APNG, animated WebP), 120 s each | 4,155 | | 4,998 | 6,031 | | | | |
| After the encoder became 8-bit and AVIF sequences refused, 30 s each | 574 | 1,279 | 754 | 1,735 | 2,687 | 5,062 | 262 | 2,794 |

- Cases per run; findings in every run: 0.
- AVIF ran through the worker binary (`--no-ffmpeg`: the decoders are
  what is fuzzed). Of its cases 34 (first run), 18 (second) and 12 (last)
  ended in `failed`: `rav1d` aborting the worker, which the
  service sees as a worker without an answer.
- The third run added a two-frame APNG and the codec's animated WebP
  fixture to the seeds; the GIF seed was already two frames.
