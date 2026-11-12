# Profile pictures: dependency proposal and design

Status: **proposal of 12.11.2026, nothing built.** No `Cargo.toml`,
`Cargo.lock`, `deny.toml`, `.cargo/audit.toml` or `approved-packages.json`
was changed. Before adoption, the owner must accept the exact packages,
versions and features in §3 (AGENTS.md: "any required dependency containing
unsafe/assembly must be disclosed and explicitly accepted"). This document is
the design pass, the first of the three in [security
assurance](../security-assurance.md): assets, caller trust, failure modes and
invariants, before any code. §5 holds the questions only the owner can answer.

The owner's decision of 12.11.2026:
- profile pictures are uploaded;
- the server decodes and re-encodes them;
- a decoder is a new dependency and needs explicit acceptance of the exact
  crates, versions and features;
- there are size limits, a way to report a picture, and moderation by
  administrators.

This answers the roadmap's open question "moderation and reporting, or picture
choice from a fixed set first?" ([roadmap](../roadmap-accounts-web.md), M1:
"profile picture (bounded, re-encoded server side, size and type checked)").
The terms already name the profile picture and the report function (§6), so
they need no new version for this.

## In short

- **The browser converts, the server checks.** The website (and later the
  native client) turns the chosen picture into a square PNG on the person's
  own device. Central accepts only that: a square PNG of at most 2 MiB, 256 to
  1024 pixels a side. It checks the header with code of its own, decodes with
  `png`, scales to 256 × 256, and writes a new PNG that holds pixels and
  nothing else. The original photo, with its place, camera and time, never
  leaves the device.
- **Seven new packages:** `png` 0.18.1, `fdeflate` 0.3.7, `flate2` 1.1.9,
  `miniz_oxide` 0.8.9, `adler2` 2.0.1, `crc32fast` 1.5.0 and `simd-adler32`
  0.3.10.
  - No gate exception is needed: `cargo deny check` and `cargo audit --deny
    warnings` pass on the resolved graph.
  - The page needs six more web-sys features (§3).
- **Unsafe that remains:**
  - SIMD checksum code in `crc32fast` (CRC-32) and `simd-adler32` (Adler-32),
    which read the uploaded bytes but do not parse them;
  - four small sites in `flate2`, used only to compress the scaled pixels.
  - Both packages that parse the uploaded bytes, `png` and `fdeflate`,
    declare `#![forbid(unsafe_code)]`, and so do `miniz_oxide` and `adler2`.
- **Not recommended:**
  - the `image` crate: 18 packages, SIMD JPEG decoding of strangers' files,
    and a colour-management library with SIMD that is compiled in and never
    called;
  - JPEG and WebP decoding on the server: five more packages, an EXIF reader
    of our own, and `zune-core` 0.5.3's packaged commit is missing from
    GitHub.

## 1. Requirements

### 1.1 What a person can choose (the page)

**Types.** The file picker offers PNG, JPEG and WebP
(`accept="image/png,image/jpeg,image/webp"`), and the page checks the
file's type again. The original may be up to 25 MiB; that bound only
protects the person's own device.
- GIF animates.
- AVIF and HEIC decoders are C or very large.
- SVG is a document that can carry script.
- **Assumed:** Safari on an iPhone hands a page that does not accept HEIC a
  JPEG instead (Apple's documented behaviour; to be checked in the WebKit
  end-to-end test).

**What the page does with the file:**
1. The browser decodes it with `createImageBitmap(file, { imageOrientation:
   "from-image" })`, so a phone photo's EXIF orientation is applied by the
   browser, not by Central.
2. The person places a square on the picture: by dragging, with the arrow
   keys, and with a zoom slider.
3. The page draws that square onto a canvas whose side is 512 pixels, or
   the square's own size when it is smaller (at least 256).
4. The page uploads `canvas.toBlob("image/png")`, and nothing else.

**Hardened browsers (assumed, from those browsers' documentation as I know
it; not tested here).**
- Firefox with `resistFingerprinting` and Tor Browser ask before a page may
  read a canvas back, and hand back a blank picture without permission.
- Brave, and Safari's advanced fingerprinting protection, add faint noise.

The page says this beside the button. The person sees the stored picture
straight after saving and can remove it.

### 1.2 What the server accepts

**The request.** `Content-Type: image/png`, a `Content-Length` of at most
2 MiB (2,097,152 bytes), and the whole body within 20 seconds.

**Checks of our own, before any decoder runs:**
- the PNG signature, then IHDR as the first chunk, of length 13;
- width equal to height, 256 to 1024;
- a valid pair of bit depth and colour type;
- compression 0, filter 0, interlace 0 or 1.

**The decoder** (`png` 0.18.1):
- `Limits { bytes: 16 MiB }`;
- text and ICC chunks skipped unread (`set_ignore_text_chunk(true)`,
  `set_ignore_iccp_chunk(true)`);
- an `acTL` chunk (an animated PNG) refused;
- the first frame only, with `Transformations::normalize_to_color8()`
  (palette and grey expanded, 16-bit cut to 8 bits);
- CRCs and the Adler-32 checked.

**Nothing after the first frame is used:** neither further image data, nor
chunks after it, nor bytes after IEND. The output is made from pixels, so a
polyglot's tail cannot reach it (verified, §2.6).

**Decompression bombs.** The decoder produces at most what IHDR announces:
side² × 4 bytes plus one filter byte a row, so 4 MiB at 1024 pixels.
Verified in the probe (§2.6):
- an IDAT that would inflate to about 400 MB was cut off at the picture's
  size, with 11 MB peak memory;
- a 400 MB zTXt chunk was skipped.

**One upload is processed at a time.** The pixel bound is also the time
bound: a 512-pixel picture took about 3 ms to decode on the probe machine,
and 10–54 ms to encode (§2.6).

### 1.3 What the server makes

**The file.** A PNG of 256 × 256 pixels: 8-bit RGB, or RGBA when any pixel
is not opaque. It holds the chunks IHDR, IDAT and IEND, and no other: no
gAMA, cHRM, sRGB, iCCP, eXIf, tEXt, iTXt, zTXt, pHYs or tIME.

**Scaling.** An exact area average (a box filter) from the upload's side to
256, weighted by alpha, in safe Rust of our own (about 50 lines in the
probe). A fully transparent pixel's colour becomes 0, so nothing can hide
under transparency.

**Deterministic.** The same pixels always give the same bytes, so the
SHA-256 names a picture. Verified twice: the same input gave the same
output, and our own output, processed again, gave itself back.

**Colour.** Central does no colour management and treats pixels as sRGB.
- Browsers convert to sRGB when they draw into a canvas (assumed for
  current Chrome, Firefox and Safari; to be checked in the end-to-end
  tests).
- A PNG from elsewhere with an ICC profile, gAMA or cICP is read as if it
  were sRGB, so a Display P3 picture looks slightly duller.
- That is the price of not linking a colour-management library (`moxcms`,
  §2.2).

**Size**, measured with `png` 0.18.1 at `Compression::High` on six sample
photos (§2.6):

| side | output |
| ---: | ---: |
| 256 px | 101–143 KB |
| 192 px | 57–80 KB |
| 128 px | 27–37 KB |

### 1.4 What is stored and where

**In PostgreSQL**, like everything else Central keeps (report parts are
`bytea` already). One row per account holds:
- the PNG (`bytea`, at most 320 KiB, `STORAGE EXTERNAL` so PostgreSQL does
  not try to compress compressed data);
- its SHA-256, its side and when it was set;
- a key of 16 random bytes that names it in the URL.

The key comes from independent randomness, not a UUIDv7 (AGENTS.md: IDs are
not secrets), and is new at every upload. Nothing of the upload itself is
kept: no original, no file name, no metadata.

**Not as files.** Files would be one more store to back up, path and
permission handling, and a second place to clear on account deletion.

**Cost:** 10,000 pictures at 256 pixels are about 1.2 GB in the database.

### 1.5 Who sees it and how it is served

**Who (Q5).** Recommended:
- the account itself;
- its contacts;
- both sides of an open contact request;
- members of a group it is in;
- administrators.

Not:
- an account that only knows the handle;
- gateways (later, §4.8);
- anyone signed out.

A block in either direction hides the picture.

**The route: `GET /api/v1/pictures/{key}`** on the browser listener, with
`{key}` as 32 lowercase hex digits. An `<img>` request can carry neither
`X-Baylee-Central` nor a JSON content type, so this route has checks of its
own:
- `Host` exactly the site's;
- `Sec-Fetch-Site` `same-origin` when it is sent;
- no `Authorization` header;
- the `__Host-central` cookie of a live browser session whose account has
  accepted Central's terms;
- the visibility rule above, checked in the statement that reads the
  picture.

Every refusal is the same `404`, whether there is no session, the key is
unknown, or the picture is not visible. The route never says whether a key
exists.

**Answer headers:**
- `Content-Type: image/png` and `Content-Length`;
- `X-Content-Type-Options: nosniff`;
- `Content-Security-Policy: default-src 'none'; sandbox`;
- `Cross-Origin-Resource-Policy: same-origin`;
- `Content-Disposition: inline; filename="picture.png"`;
- `Cache-Control: private, max-age=3600`, and `ETag` set to the digest;
- `Referrer-Policy: no-referrer`.

No cookie is set and no CORS header is sent. The bytes are always a PNG
that Central wrote, declared as one, so no browser needs to guess.

**Caching.** The key changes with every upload, so one URL never names two
pictures. After a removal, new viewers get `404` at once. A browser that
already showed the picture may keep it for up to an hour.

**Budget.** Limits of its own, apart from `POST /api/v1`'s four places per
address: 16 in flight and 600 a minute per address. One indexed `SELECT`
answers each request. Pages load avatars with `loading="lazy"`.

**CSP unchanged.** `img-src 'self'` already covers the route. The crop
preview is drawn in a `<canvas>`, so `blob:` is not needed.

### 1.6 Removal, account deletion and export

**Removal.** `RemovePicture` deletes the row, and the key answers `404` at
once.

**Account deletion.** `ON DELETE CASCADE` takes the picture, the reports
about it and its notice. Reports the account filed lose their reporter
(`ON DELETE SET NULL`), so no row names the deleted account. That is
[account deletion](../account-deletion.md)'s test.

**Export.** `AccountExport` refuses unknown fields, so it gains none.
Instead, the export page adds:
- `picture.png`, from the route;
- the picture's facts, from `MyPicture`;
- the reports this account filed (reason, time, state), from
  `MyPictureReports`.

Reports about one's own picture are not exported with their reporters
(GDPR Art. 15(4)). The person was told of every removal and its reason
(§4.6). Whether that suffices is the owner's legal question.

## 2. Candidates and evidence

### 2.1 How this was measured

**Resolution.** The repository tree at `5529fe4` was copied (`git archive`)
into a temporary directory. A placeholder crate there held exactly the
candidate dependencies. Then, in that copy:
- `cargo metadata --all-features` against the repository's `Cargo.lock`
  (Cargo adds only what is new);
- `cargo tree --target all`;
- `cargo deny check` 0.20.2 with the repository's `deny.toml`;
- `cargo audit --deny warnings` 0.22.2 with its `.cargo/audit.toml`, on an
  advisory database fetched on 12.11.2026 (1,471 advisories);
- `cargo check` of the placeholder.

The repository's own manifests and lock file were not touched.

**Marker counts.** As in [leptos.md](leptos.md): files containing an
`unsafe` block, function, impl, trait or extern, or an assembly macro,
tests and cfg-inactive code included. "Compiled" is read from the cfgs and
features this graph enables. This is not a soundness audit.

**Other sources:**
- the crates.io API: versions, owners, publishers;
- GitHub: activity and fuzz folders through the API, until its
  unauthenticated quota ran out, then raw files;
- a local copy of the RustSec database, for history;
- OSS-Fuzz's `projects/` folder.

**Probes.** Two small programs in a scratch directory, not project code:
- one linking `png`;
- one linking `zune-jpeg` (with only `std`) and `image-webp`.

They ran as release builds with overflow checks on this Mac (Apple
Silicon). Timings are this machine's, not the server's.

**Inputs.**
- Photos: macOS's account pictures in `/Library/User Pictures`, converted
  with `sips`.
- Hostile files: written by a short script.

### 2.2 Option A: `image` 0.25.10 with `png`, `jpeg` and `webp`

**18 new packages:**
- `image` 0.25.10, `bytemuck` 1.25.2, `byteorder-lite` 0.1.0;
- `moxcms` 0.8.1, `pxfm` 0.1.30;
- `zune-core` 0.5.3, `zune-jpeg` 0.5.15, `image-webp` 0.2.4, `quick-error`
  2.0.1;
- the `png` stack of Option C with `flate2` 1.1.11, so also `miniz_oxide`
  0.9.1 and `zlib-rs` 0.6.8.

`num-traits` 0.2.19 also gains its `default` feature.

**The packages A adds beyond B and C:**

| Package | Licence | Owners on crates.io | Published | Build script | Rust lines |
| --- | --- | --- | --- | --- | ---: |
| image 0.25.10 | MIT OR Apache-2.0 | theotherphil, fintelia, 197g, image-rs's team | 10.03.2026 | no | 37,189 |
| bytemuck 1.25.2 | Zlib OR Apache-2.0 OR MIT | Lokathor, rust-bus maintainers | 19.07.2026 | no | 4,162 |
| moxcms 0.8.1 | BSD-3-Clause OR Apache-2.0 | awxkee (alone) | 07.03.2026 | no | 42,813 |
| pxfm 0.1.30 | BSD-3-Clause OR Apache-2.0 | awxkee (alone) | 06.07.2026 | no | 102,970 |

The licences pass `deny.toml`. `cargo deny check` on this graph fails only
`bans`, on the `miniz_oxide` duplicate; `cargo audit` exits 0 (both
verified).

**Unsafe in source.** Sites and files, as counted:

| package | sites | files | notes |
| --- | ---: | ---: | --- |
| `bytemuck` | 348 | 19 | |
| `moxcms` | 344 | 30 | SIMD; it forbids unsafe only with `avx`, `sse` and `neon` off, and `image` turns them on through default features |
| `pxfm` | 224 | 72 | FMA paths; its `#![forbid(unsafe_code)]` is commented out |
| `zune-jpeg` | 79 | 10 | AVX2 and NEON compiled, because `image` asks for default features |
| `image` | 8 | 4 | |

**`moxcms` is unconditional, but our pipeline never calls it.** Reading
`image`'s source, it is reached only through `CicpTransform`
(colour-space conversion), which our steps would not call. It and `pxfm`
are still compiled in.

**A licence note found on the way.** `image`'s JPEG encoder
(`src/codecs/jpeg/transform.rs`) carries the Independent JPEG Group's
licence in its header, while the crate declares only `MIT OR Apache-2.0`.

**What it would give:** decoders for all three formats, scaling, EXIF
orientation and a JPEG encoder.

**Rejected.** It has the largest graph and the most unsafe code, and it
decodes strangers' JPEGs with SIMD.

### 2.3 Option B: the format crates directly

**The graph.** `png` + `zune-jpeg` 0.5.15 (default features off, `std`) +
`zune-core` 0.5.3 + `image-webp` 0.2.4: 14 new packages. These are the nine
of `png` with `flate2` 1.1.11 (seven with 1.1.9), plus `zune-jpeg`,
`zune-core`, `image-webp`, `byteorder-lite` and `quick-error`.

| Package | Licence | Owners on crates.io | Published | Build script | Rust lines | Crate root |
| --- | --- | --- | --- | --- | ---: | --- |
| zune-jpeg 0.5.15 | MIT OR Apache-2.0 OR Zlib | etemesi254 (alone) | 26.03.2026 | no | 9,737 | `forbid` without `x86` and `neon` |
| zune-core 0.5.3 | MIT OR Apache-2.0 OR Zlib | etemesi254 (alone) | 07.08.2026 | no | 3,206 | none; 0 markers |
| image-webp 0.2.4 | MIT OR Apache-2.0 | fintelia | 27.08.2025 | no | 8,670 | `forbid` |
| byteorder-lite 0.1.0 | Unlicense OR MIT | fintelia, image-rs's team | 07.04.2024 | no | small | `forbid` |
| quick-error 2.0.1 | MIT/Apache-2.0 | tailhook | 07.05.2021 | no | small | none; 0 markers |

`zune-jpeg` 0.5.16 is at release candidate 3 (19.10.2026); 0.5.15 is its
newest release.

**`jpeg-decoder`, the older JPEG crate, is out.** Version 0.3.2 dates from
21.06.2025, the repository's last push was the same day, and it has 45 open
issues. Its default feature pulls in `rayon`. `image` moved from it to
`zune-jpeg`.

**Unsafe.** With `x86` and `neon` off, `zune-jpeg` forbids unsafe
(`#![cfg_attr(not(any(feature = "x86", feature = "neon")),
forbid(unsafe_code))]`). `cargo check` passed that way, so none of its 79
sites is compiled. `image-webp`, `byteorder-lite` and `adler2` forbid unsafe
too. `cargo deny` fails only on the `miniz_oxide` duplicate (§2.4), and
`cargo audit` exits 0.

**Costs:**
- an EXIF orientation reader of our own, on hostile bytes;
- server-side cropping;
- the scalar JPEG path gets less upstream fuzzing, because OSS-Fuzz builds
  `image-rs` with `zune-jpeg`'s default SIMD on x86-64;
- `zune-core` 0.5.3's packaged commit `de5dfc800cfa` is not on GitHub: the
  raw host and the commit page both answer 404. Version 0.5.2 was yanked
  the same day;
- the zune crates have a single maintainer.

**Probe (verified):**

| input | result |
| --- | --- |
| baseline JPEG, 512² | 3.3 ms |
| baseline JPEG, 4096² | 140 ms, 55 MB peak |
| CMYK JPEG | decoded to RGB |
| lossy WebP, 512² | 11 ms |
| lossless WebP, 4096² | 387 ms, 129 MB peak |
| animated WebP | refused from its header, before any pixel |

**Read in source, not run:**
- a progressive JPEG with very many scans (`zune-core` has
  `jpeg_set_max_scans`);
- arithmetic coding (feature off).

### 2.4 Option C, recommended: `png` only

**Graph and checks.**
- Seven new packages, with nothing else changed in the lock (table in
  §2.7).
- `cargo deny check`: advisories, bans, licences and sources all `ok`.
- `cargo audit --deny warnings`: exits 0.

**Two packages are pinned below their newest release, on purpose.**

**`flate2` 1.1.9 (03.02.2026) instead of 1.1.11 (06.11.2026).**
- *Why it matters.* Since 1.1.10, `flate2` depends on `miniz_oxide` 0.9,
  while `png` 0.18.1 also declares `miniz_oxide ^0.8`. Both versions would
  be built, and `cargo deny check bans` fails on the duplicate (verified).
- *Unused.* `png` names `miniz_oxide` only in a doc comment; nothing of it
  is called.
- *zlib-rs.* With 1.1.11, `zlib-rs` 0.6.8 also enters the lock and
  `cargo metadata` through `flate2`'s weak `zlib-rs?/std`. `cargo tree -i
  zlib-rs` prints nothing: it is compiled nowhere. The policy check would
  still need it approved, as `fiat-crypto` was.
- *The fixes 1.1.9 lacks.* The 1.1.10 and 1.1.11 release notes fix:
  - gzip header and footer handling;
  - deflate streams incomplete at EOF;
  - partial writes and sync flushes in the writer;
  - a panic when an unfinished compressor is dropped.
- *Why that is acceptable here.* We use `flate2` only through `png`'s zlib
  encoder. Decoding goes through `fdeflate`.
  - The encoder compresses the scaled pixels. Their values come from the
    uploader; their length and layout are ours (256 rows of fixed size).
  - `png`'s `write_image_data` (`encoder.rs`, from line 818) drives
    `ZlibEncoder` over a `Vec` with `write_all` for each row, then
    `finish()`. A `Vec` never takes a partial write, the encoder never
    flushes with `Sync`, nothing is written after `finish`, and `finish` is
    always reached.
  - So the fixed bugs (a write that hangs after `try_finish`, partial
    writes, sync flushes across buffer boundaries, dropping an unfinished
    compressor) are on paths this one does not take. That was read in the
    source, not proved.
  - §4.11's round-trip test decodes every output we write.
  - A hang would hold the single processing slot until the service
    restarts, because a blocking thread cannot be stopped (§3, Q6).
- *The choice.* With 1.1.9 there is one `miniz_oxide`, no `zlib-rs` and no
  gate exception. That is Q2.

**`crc32fast` 1.5.0 (13.07.2025) instead of 1.5.5 (10.11.2026).**
- Five releases between 22.08. and 10.11.2026 grew the SIMD modules from
  307 to 929 lines and the marker sites from 6 to 23: new VPCLMULQDQ
  AVX2/AVX-512 folds on x86, and four- to sixteen-stream folds on aarch64.
- 1.5.5 was two days old at review time.
- 1.5.0 has the older, smaller paths and has had 16 months of use (385
  million downloads).
- I read the release-to-release diffs from 1.5.0 to 1.5.5, the SIMD
  modules included: 551, 269, 330 and 290 changed lines.
  - They add the wide paths and restructure `baseline.rs` and `lib.rs` for
    speed.
  - 1.5.4 bounds-checks the SIMD loads (`a[..16]` before the load, "a
    miscounted length panic[s] instead of reading out of bounds").
    1.5.0's load helper relies on its callers instead: under 128 bytes it
    returns to the table path, and its loops run at 64 and then 16 bytes or
    more. So that change hardens new code and corrects no line of 1.5.0.
  - No change corrects a line that exists in 1.5.0.
- Upstream has no changelog file, and the GitHub API quota was spent, so
  release notes were not read.

**Consequence.** Both pins are held by the approved inventory. A later
`cargo update` that moves them fails `cargo xtask policy` until they are
reviewed again.

### 2.5 Encoders for the output

- **PNG (chosen).** Lossless, keeps transparency, no extra package, and
  the native client can draw it (Baylee's `Cargo.lock` already has `png`
  0.18.1 and `image` 0.25.10; read, not changed).
- **JPEG.** About a quarter of the bytes, but transparency must be
  flattened onto a colour.
  - macOS's own encoder at quality 85 gave 23–40 KB at 256 px; that is
    not a Rust encoder.
  - The encoder would be `image`'s own (Option A only) or `jpeg-encoder`
    0.7.1, whose licence is `(MIT OR Apache-2.0) AND IJG`. IJG is not on
    `deny.toml`'s allow list, so that would need a licence exception.
- **WebP lossless** (`image-webp`'s encoder): about a quarter to a third
  smaller than PNG, for three more packages.
  - Measured with `cwebp`, not Rust: 70–111 KB against 107–143 KB for
    macOS's PNG.
- **WebP lossy.** No pure-Rust encoder exists; libwebp is C. Out.

### 2.6 Probe results for Option C

**Pipeline.** PNG in → our IHDR checks → `png` decode → area average to 256
→ `png` encode at `Compression::High`. Built against `png` 0.18.1, `flate2`
1.1.9, `crc32fast` 1.5.0, `miniz_oxide` 0.8.9 and `simd-adler32` 0.3.10.

**Sample photos**, 512² PNGs of macOS's account pictures. Each time is the
median of three runs:

| input | bytes in | out at 256 | at 192 | at 128 | decode | scale | encode |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Eagle | 405,874 | 104,829 | 59,289 | 27,845 | 3.2 ms | 0.9 ms | 51 ms |
| Owl | 475,736 | 123,327 | 69,501 | 32,778 | 2.8 ms | 1.0 ms | 14 ms |
| Parrot | 426,819 | 114,776 | 66,292 | 31,576 | 3.1 ms | 0.9 ms | 28 ms |
| Zebra | 481,869 | 127,867 | 73,409 | 35,053 | 2.9 ms | 1.0 ms | 25 ms |
| Dahlia | 547,027 | 143,234 | 80,089 | 36,565 | 3.3 ms | 0.9 ms | 11 ms |
| Chalk | 393,595 | 100,593 | 56,903 | 27,122 | 3.2 ms | 0.9 ms | 44 ms |

An earlier run with `flate2` 1.1.11, `miniz_oxide` 0.9.1 and `crc32fast`
1.5.5 gave byte-identical outputs and encoded about twice as fast (5–25 ms).

**Hostile files** (synthetic):

| file | result | peak memory |
| --- | --- | ---: |
| IHDR 1024², IDAT inflating to ~400 MB (398 KB file) | decoded to the picture's size, rest never read; 1.5 ms | 11.4 MB |
| valid 256² with a zTXt inflating to ~400 MB | chunk skipped unread | 3.3 MB |
| IHDR 100000 × 100000 | refused before any pixel buffer | 1.8 MB |
| `acTL` before IDAT (APNG) | refused | 1.9 MB |
| valid PNG followed by an HTML document | output byte-identical to the same picture without it | 2.9 MB |
| `eXIf` and a `tEXt` holding `<script>` | output byte-identical to the same picture without them | 3.2 MB |
| one bit changed in IDAT's CRC | refused (CRC error) | 2.4 MB |
| cut in half | refused (unexpected end) | 2.1 MB |

Every output held exactly IHDR, IDAT and IEND. The macOS input had `sRGB`,
`eXIf`, `pHYs` and `iTXt`; none of them survived.

### 2.7 Inventory of the recommendation

"Sites" count markers in all source, tests included. "Compiled" is what
this graph builds on the server (x86-64 Linux) and on Apple Silicon.

| Package | Licence | Role | Rust lines | Sites (files) | Compiled unsafe | Build script | Crate root |
| --- | --- | --- | ---: | ---: | --- | --- | --- |
| png 0.18.1 | MIT OR Apache-2.0 | decodes the upload, encodes the result | 12,674 | 0 | none | no | `forbid(unsafe_code)` |
| fdeflate 0.3.7 | MIT OR Apache-2.0 | `png`'s inflater; hostile input | 2,276 | 0 | none | no | `forbid` |
| miniz_oxide 0.8.9 | MIT OR Zlib OR Apache-2.0 | `flate2`'s deflate backend | 6,948 | 0 | none | no | `forbid` |
| adler2 2.0.1 | 0BSD OR MIT OR Apache-2.0 | `miniz_oxide`'s scalar Adler-32 | 442 | 0 | none | no | `forbid` |
| flate2 1.1.9 | MIT OR Apache-2.0 | `png`'s zlib encoder (output only) | 7,317 | 36 (3) | 4 sites: `mem.rs`'s `write_to_spare_capacity_of_vec` and its two callers, and `ffi/mod.rs`'s zero-initialising cast; the other 32 belong to the C-zlib backends, which are off | no | none |
| crc32fast 1.5.0 | MIT OR Apache-2.0 | chunk CRCs (hostile input), zlib | 1,335 | 6 (2) | `pclmulqdq.rs` on x86-64 (runtime check for `pclmulqdq`, `sse2`, `sse4.1`); `aarch64.rs` on aarch64 (runtime check for `crc`) | yes: runs `$RUSTC --version`, sets `stable_arm_crc32_intrinsics` | none |
| simd-adler32 0.3.10 | MIT | Adler-32 for `fdeflate` and `miniz_oxide` (hostile input) | 1,958 | 36 (6) | `sse2`, `ssse3`, `avx2` on x86-64, by runtime detection; `neon` on aarch64; `avx512` only with its `nightly` feature (off) | no | none |

None of the seven has C, C++ or assembly sources (`find` for `.c`, `.h`,
`.cc`, `.cpp`, `.S`, `.s`, `.asm`: none).

**Maintainers and activity:**

| Package | Owners on crates.io | Version published | Repository |
| --- | --- | --- | --- |
| png | fintelia, 197g, image-rs's publish team | 14.02.2026 | image-rs/image-png, last push 23.09.2026, 69 open issues |
| fdeflate | fintelia, image-rs's team | 06.12.2024 | image-rs/fdeflate, last push 31.05.2026 |
| flate2 | rust-lang-owner | 03.02.2026 | rust-lang/flate2-rs, last push 06.11.2026 |
| miniz_oxide | oyvindln, Frommi | 09.06.2025 | Frommi/miniz_oxide, last push 02.11.2026 |
| adler2 | oyvindln | 09.06.2025 | oyvindln/adler2 |
| crc32fast | srijs | 13.07.2025 | srijs/rust-crc32fast, last push 10.11.2026 |
| simd-adler32 | mcountryman (alone) | 14.07.2026 | mcountryman/simd-adler32, last push 14.07.2026 |

None of these repositories is archived.

**Features, exactly as resolved:**

| Package | Features |
| --- | --- |
| png | none |
| fdeflate | none |
| flate2 | `any_impl`, `default`, `miniz_oxide`, `rust_backend` |
| miniz_oxide | `default`, `simd`, `simd-adler32`, `with-alloc` |
| adler2 | none |
| crc32fast | `default`, `std` |
| simd-adler32 | `const-generics`, `default`, `std` |

Already-approved packages in the graph (`bitflags` 2.13.2, `cfg-if` 1.0.5)
gain no feature.

### 2.8 Advisories and licences

**RustSec database of 12.11.2026.**
- No advisory, of any date, names `png`, `fdeflate`, `flate2`,
  `miniz_oxide`, `adler2`, `crc32fast` or `simd-adler32`.
- None names Option B's or A's other packages either (`zune-jpeg`,
  `zune-core`, `image-webp`, `moxcms`, `pxfm`, `bytemuck`).
- `image` has two from 2019 and 2020, both fixed since 0.23.12.
- `zlib-rs` had RUSTSEC-2024-0401, fixed in 0.4.0.

**GitHub.** No published repository advisory for image, image-png,
image-webp, fdeflate, zune-image or moxcms (API, 12.11.2026).

**Tools.**
- `cargo audit --deny warnings` exits 0 on all three graphs.
- `cargo deny check` passes everything on Option C with `flate2` 1.1.9.
- Options A and B, and C with `flate2` 1.1.11, fail only `bans`, on the
  `miniz_oxide` duplicate.

**Licences.** All seven are on `deny.toml`'s allow list: MIT, Apache-2.0,
Zlib, and `0BSD OR MIT OR Apache-2.0` through MIT. No licence exception is
needed.

### 2.9 Provenance

For each of the seven, every packaged `src/**/*.rs`, the build script and
`Cargo.toml.orig` were compared byte for byte with the upstream repository
at the commit `.cargo_vcs_info.json` names:

| Package | Commit | Files equal |
| --- | --- | ---: |
| png 0.18.1 | `2a3f980245e3`, the merge of its release pull request #676 | 22 of 22 |
| fdeflate 0.3.7 | `c365c7e6ffa8`, tag `v0.3.7` | 7 of 7 |
| flate2 1.1.9 | `19ddb18bf111` | 22 of 22 |
| miniz_oxide 0.8.9 | `44e43c7786e3` | 15 of 15 |
| adler2 2.0.1 | `89a031a0f42e` | 3 of 3 |
| crc32fast 1.5.0 | `dbf4f76cd71c` | 9 of 9 |
| simd-adler32 0.3.10 | `d93164f98d7f` | 11 of 11 |

Registry checksums are pinned in `Cargo.lock` as usual. Tags were checked
only for `png` and `fdeflate` before the API quota ran out.

### 2.10 Fuzzing

**Upstream.**
- `image-png` is an OSS-Fuzz project (targets `decode`, `buf_independent`,
  `roundtrip`). Its decoding runs through `fdeflate`, `crc32fast` and
  `simd-adler32`.
- `miniz_oxide` is an OSS-Fuzz project of its own.
- `fdeflate` keeps eight cargo-fuzz targets in its repository.
- For Option B: `zune-jpeg` has four and `image-webp` two. `image-rs` on
  OSS-Fuzz covers JPEG and WebP through `image`'s default, SIMD build.

**What that fuzzing does not cover.** OSS-Fuzz builds upstream's newest
versions. So it exercises `crc32fast`'s and `flate2`'s current code, not the
pinned 1.5.0 and 1.1.9. For those, the evidence is their earlier time on
OSS-Fuzz and their use since.

**Here.** `cargo-fuzz` needs a nightly compiler and libFuzzer's C++, which
would be a new tool decision. Instead, §4.11 proposes seeded mutation tests
inside `cargo test`, which add no dependency.

## 3. Recommendation

**The manifest.** A new crate, `central-picture` (source-available, server
only):

```toml
[dependencies]
png = "=0.18.1"
```

**The lock**, pinned with `cargo update --precise` and held by the approved
inventory:
- `fdeflate` 0.3.7, `flate2` 1.1.9, `miniz_oxide` 0.8.9, `adler2` 2.0.1,
  `crc32fast` 1.5.0, `simd-adler32` 0.3.10;
- with the features in §2.7.

`png` has no default features; its optional ones (`benchmarks`, `unstable`,
`zlib-rs`) stay off.

**Why this set:**
- **Only PNG parsing on hostile input.** The server parses only PNG, and
  every parser of those bytes forbids unsafe.
- **Smaller graph.** Seven packages instead of 14 or 18.
- **No gate exception.**
- **Privacy.** The person's original, with its EXIF, never reaches
  Central.
- **What it costs:**
  - the page's conversion (§1.1);
  - colour management (§1.3);
  - the native client's own conversion at M6 (§4.8).

**Unsafe that remains, and why it is acceptable:**

| Where | What | Reached by hostile bytes | Why acceptable |
| --- | --- | --- | --- |
| `crc32fast` 1.5.0 | carry-less-multiply CRC-32 (x86-64), CRC instructions (aarch64) | yes: each chunk's CRC | Small, old paths (307 lines). Runtime-detected, with a safe table fallback. The pin avoids the newer, larger paths. `image-png`'s OSS-Fuzz targets ran this code while it was the newest release; today they run 1.5.5. |
| `simd-adler32` 0.3.10 | SSE2, SSSE3, AVX2 and NEON Adler-32 | yes: the zlib checksum of the image data | A sum over a byte slice, with a scalar fallback. Exercised through `fdeflate` and `miniz_oxide` fuzzing. A single maintainer, which is a risk to watch. |
| `flate2` 1.1.9 | `MaybeUninit` buffer helpers | indirectly: it compresses the scaled pixels, whose values the uploader chose and whose length and layout are ours | The sites only set a `Vec`'s length after compression and zero a buffer. The path `png` takes is §2.4's. Every test decodes the output again and compares it (§4.11). |
| Rust's standard library | as everywhere | yes | Part of the trusted base already (README.md). |

**How decoding is contained (Q6 asks the owner).** Recommended: in the
service's process, in a blocking task, one picture at a time.
- **Before decoding:** the upload has passed the header checks of §1.2 and
  the session, budget and block checks of §4.3.
- **Panics:** a panic becomes a refused upload, because the release
  profile unwinds; only the `wasm` and `islands` profiles abort.
- **Memory:** about 12 MiB at the largest input, against 11 MB measured.
- **No database work and no secret are involved:** the function receives
  bytes and returns bytes.
- **What it cannot contain is a hang.** A blocking thread cannot be
  stopped. If decoding or encoding never returned, the one processing slot
  would stay taken, and every later upload would get `429` until the
  service restarts.
  - Nothing found points to such a loop: the inputs are bounded, and §2.4
    covers `flate2`'s writer.
  - Still, the service should count a slot held for longer than ten seconds
    and report it in its health output, so a stuck slot is seen.

**The alternative is a child process** of the same binary for every
upload, with its environment cleared and killed after two seconds.
- *What it adds:* it would contain a memory-corruption bug in the two
  checksum crates, and it would end a hang.
- *What it costs:* a process start per upload, a pipe protocol, and tests
  across three operating systems.
- *What it cannot do without new dependencies:* limit memory or system
  calls, which needs `setrlimit`, seccomp or Landlock, all of which reach
  unsafe.

Its added safety is small while no parser of the upload uses unsafe.

**Threat model.** For [threat model](../threat-model.md)'s "Upload parser
compromise" row, "isolated" therefore means three things:
- a function on its own thread;
- given only the upload's bytes;
- written in crates that forbid unsafe.

**Entries that follow in `approved-packages.json`**, when accepted:

```json
"packages": [
  "adler2@2.0.1", "crc32fast@1.5.0", "fdeflate@0.3.7", "flate2@1.1.9",
  "miniz_oxide@0.8.9", "png@0.18.1", "simd-adler32@0.3.10"
],
"features": {
  "adler2@2.0.1": [],
  "crc32fast@1.5.0": ["default", "std"],
  "fdeflate@0.3.7": [],
  "flate2@1.1.9": ["any_impl", "default", "miniz_oxide", "rust_backend"],
  "miniz_oxide@0.8.9": ["default", "simd", "simd-adler32", "with-alloc"],
  "png@0.18.1": [],
  "simd-adler32@0.3.10": ["const-generics", "default", "std"],
  "web-sys@0.3.107": ["…the approved list…", "CanvasRenderingContext2d",
    "File", "FileList", "ImageBitmap", "ImageBitmapOptions", "ImageOrientation"]
},
"approvals": [{
  "approved_on": "<the owner's date>",
  "reason": "<the owner's words>",
  "for": "profile pictures: central-picture decodes the uploaded PNG and writes a new one (docs/dependency-proposal/profile-picture.md)",
  "entries": [
    "png@0.18.1 as a direct dependency of central-picture (it has no default features)",
    "fdeflate@0.3.7, flate2@1.1.9, miniz_oxide@0.8.9, adler2@2.0.1, crc32fast@1.5.0 and simd-adler32@0.3.10 with the features of profile-picture.md §2.7",
    "web-sys@0.3.107 gains CanvasRenderingContext2d, File, FileList, ImageBitmap, ImageBitmapOptions and ImageOrientation for the page's conversion"
  ],
  "implied": "Dependency-internal unsafe (SIMD CRC-32 and Adler-32, flate2's buffer helpers) was disclosed; every parser of the upload forbids unsafe; first-party unsafe stays forbidden. Not a third-party source audit."
}]
```

**What else changes, and what does not:**
- **No exceptions** in `deny.toml` or `.cargo/audit.toml`.
- **If the owner prefers `flate2` 1.1.11:**
  - `deny.toml` gains a skip for `miniz_oxide` 0.8.9, with this reason:
    "png 0.18.1 declares it and calls nothing of it";
  - the inventory gains `miniz_oxide@0.9.1` and `zlib-rs@0.6.8`
    (`rust-allocator`, `std`), the latter never compiled;
  - a guard like `cargo xtask sqlite-scope` should then fail once
    `zlib-rs` is built.
- **Unaffected:** `cargo xtask crypto-scope`, because `central-crypto`'s
  graph does not change.
- **Licences (LICENSE.md §6).** `central-api`, which stays AGPL, gains
  types only and no dependency. The AGPL crates never build on
  `central-picture`, as the policy check enforces.

## 4. Design for the build

### 4.1 Crates

**`central-picture`** (new, source-available) holds one function:
`fn reencode(png: &[u8]) -> Result<Picture, Refusal>`. It:
- runs the header checks of §1.2;
- decodes with the bounds there;
- scales by area average;
- writes the PNG and its SHA-256 (`sha2`, already approved).

`Refusal` is a closed enum (`NotPng`, `Size`, `Animated`, `Damaged`,
`Limit`). The crate root forbids unsafe and nothing in it does I/O.

**Callers.**
- `central-identity` calls it inside `tokio::task::spawn_blocking`.
- `central-postgres` stores the result.
- `central-http` adds the two routes.

### 4.2 Wire: new variants only

Every answer refuses unknown fields, so no existing request or answer gains
a field. Old gateways and clients never send the new requests and so never
receive the new answers.

**For the account:**
- `Request::MyPicture` → `Response::MyPicture`, which holds:
  - `picture: Option<Picture { key: [u8; 16], side: u16, bytes: u32,
    sha256: [u8; 32], set_at: i64 }>`;
  - `notice: Option<PictureNotice { at, reason, note }>`;
  - `blocked_until: Option<i64>`.
- `Request::RemovePicture` → `Done`.
- `Request::MyPictureReports` → `Response::MyPictureReports { reports }`,
  for the export.

**Social:**
- `SocialRequest::Pictures { tags }` (at most 300 tags) →
  `SocialResponse::Pictures { pictures: Vec<{ tag, key }> }`. It lists only
  the visible pictures; absent means none, or not visible.
- `SocialRequest::ReportPicture { tag, key, reason, note: Option<String> }`
  → `Done`.

**Administration:**
- `AdminRequest::PictureReports { state, before }`;
- `AdminRequest::PictureCase { account }`: the current picture, its open
  and decided reports, the notice and the block;
- `AdminRequest::DecidePicture { account, sha256, decision: Keep |
  Remove { reason, note }, block: None | Days(u16) | UntilLifted }`;
- `AdminRequest::LiftPictureBlock { account }`;
- with matching new `AdminAnswer` variants.

**On which listener.** The browser listener only, for now. The native
listener refuses them, as it does `SetDisplayName`, until M6.

### 4.3 Routes and budgets

**`PUT /api/v1/picture`**, the body a raw PNG. The browser listener's
64 KiB body bound does not fit, so this is a route of its own, like the
report parts. It answers the JSON of `Response::MyPicture`.

The checks run in this order:

1. **Headers, before anything else:**
   - `Host` and `Origin` exact;
   - `Sec-Fetch-Site` `same-origin` when it is sent;
   - `Content-Type: image/png` exactly;
   - `X-Baylee-Central: 1`;
   - no `Authorization`;
   - the loopback proxy and the client address, as for `POST /api/v1`,
     including its place in flight;
   - `Content-Length` present and at most 2 MiB.

   `image/png` and the custom header both force a CORS preflight, which is
   never granted.
2. **The account, before any byte is read:**
   - the session cookie, with Central's terms accepted;
   - not blocked from uploading;
   - within its budget: 10 uploads an hour and 30 a day, in one durable row
     per account, like `social_budgets`;
   - an upload slot: two at a time in all, one per account; otherwise
     `429`.
3. **The body**, read to `Content-Length` within 20 seconds, else `408`.
   From here the upload counts against the budget, whether it is stored or
   refused.
4. **Processing:** one at a time. The upload waits at most two seconds for
   its turn, else `429`. Then `reencode` runs on a blocking thread.

   **Open for the next design round.** A refused picture must not be
   answered `401 Rejected`. The account island reads `Rejected` as an ended
   session and signs the person out, and it could not tell a refused picture
   from that. This raw route can use a status of its own instead: `422`,
   with the `Refusal` code in a small JSON body, so the page can say which
   check failed.
5. **One transaction:**
   - lock the account row;
   - check the session and the block again;
   - replace the row with a new key;
   - mark open reports about the old digest `gone`.

**`GET /api/v1/pictures/{key}`**, as §1.5 describes.

**Other methods** on both paths answer `405`. Nothing of a body and no key
is ever logged.

### 4.4 Migration

**The number.** It takes the next free number, **000027**:
- `central-migration/src/lib.rs` ends at 000026;
- 000024 is reserved for the session-token line;
- [postgresql.md](../postgresql.md) says 000027 "was kept" for the report
  levels "and is not used".

If another line of work lands first, this takes the number after it.

```sql
CREATE TABLE central_identity.pictures (
    account_id uuid PRIMARY KEY REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    key bytea NOT NULL UNIQUE CHECK (octet_length(key) = 16),      -- independent randomness
    png bytea NOT NULL CHECK (octet_length(png) BETWEEN 67 AND 327680),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    side smallint NOT NULL CHECK (side = 256),
    set_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);
ALTER TABLE central_identity.pictures ALTER COLUMN png SET STORAGE EXTERNAL;

CREATE TABLE central_identity.picture_reports (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) = 7),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),       -- the picture reported
    reporter_id uuid REFERENCES central_identity.accounts(id) ON DELETE SET NULL,
    reason text NOT NULL CHECK (reason IN
        ('sexual', 'violence', 'hate', 'harassment', 'impersonation', 'private', 'other')),
    note text CHECK (char_length(note) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    state text NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'kept', 'removed', 'gone')),
    decided_at timestamptz,
    decided_by uuid,
    CHECK ((state = 'open') = (decided_at IS NULL)),
    CHECK (reporter_id IS DISTINCT FROM account_id)
);
CREATE UNIQUE INDEX picture_reports_open
    ON central_identity.picture_reports(account_id, sha256, reporter_id) WHERE state = 'open';
CREATE INDEX picture_reports_queue ON central_identity.picture_reports(state, created_at);

CREATE TABLE central_identity.picture_notices (     -- what the person is told; one per account
    account_id uuid PRIMARY KEY REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    removed_at timestamptz NOT NULL,
    reason text NOT NULL,                            -- the same closed list
    note text CHECK (char_length(note) BETWEEN 1 AND 500),
    blocked_until timestamptz                        -- NULL none, 'infinity' until lifted
);

CREATE TABLE central_identity.picture_budgets (     -- one fixed row per account
    account_id uuid PRIMARY KEY REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    hour_start timestamptz NOT NULL, hour_count smallint NOT NULL,
    day_start timestamptz NOT NULL, day_count smallint NOT NULL
);
```

**Runtime grants.** The startup check refuses any grant wider or narrower
than these:

| Table | Runtime role |
| --- | --- |
| `pictures` | `SELECT, INSERT, UPDATE (key, png, sha256, side, set_at), DELETE` |
| `picture_reports` | `SELECT, INSERT, UPDATE (state, decided_at, decided_by)`, no `DELETE`: the moderation record is append-only for the runtime, as the accounts-admin design's step 8 has it |
| `picture_notices` | `SELECT, INSERT, UPDATE, DELETE` |
| `picture_budgets` | `SELECT, INSERT, UPDATE` |

**The sweep.** The operator's `--sweep-stdin` deletes decided reports past
their retention (Q10).

**Earlier binaries.** The previous binary names none of these tables, so it
runs unchanged over the migration.

### 4.5 Store and service

**Visibility** is one SQL predicate, used both by `GET` and by
`SocialRequest::Pictures`. A picture is visible to:
- its owner;
- an administrator;
- an account with a `friend`, `incoming` or `outgoing` relation to it,
  unless a block exists in either direction;
- a member of a group the owner is in.

It is tested as a matrix (§4.11).

**Errors.** Every answer keeps the existing vocabulary: `Rejected`,
`Conflict`, `Busy`, `Unavailable`, `TermsRequired`.

### 4.6 Reports and moderation

**Why a table of its own.** [Reports](../reports.md)' bug-report table does
not fit:
- a gateway files every report there;
- the reporter is a gateway's pseudonym;
- its privacy invariant forbids joining a report to an account.

The model is the accounts-admin design's unbuilt step 8 (`account_reports`,
branch `c41/accounts-admin-design`): a category, a note of 1 to 1000
characters, open or decided, decided by an administrator, append-only for
the runtime, with the reported person told of a decision but never of who
reported. `picture_reports` has its shape, so one `/admin/reports` queue
can show both later.

What is reused:
- the administration's rules: the role checked in the acting transaction,
  a password sign-in under 30 minutes for a change, the audit written in
  the same transaction;
- the website's list-and-drawer components.

**Filing (any viewer, §1.5).**
- The menu (⋮) on a contact row, on a conversation's head and on a group
  member offers "Report picture".
- The report holds a reason from the closed list and an optional note.
- The server records the picture's **digest**, so a decision applies to
  exactly the picture that was reported.
- Refusals:
  - one's own picture: `Rejected`;
  - a picture one cannot see, or a key that is not the account's current
    one: the same `Rejected`;
  - a second open report on the same picture: `Conflict`;
  - more than 20 reports an hour, or 100 open, from one reporter: `Busy`.
- The reported person is not told of a report, only of a decision.

**Deciding (administrators, `/admin/reports`).** The queue shows the
picture through the same `GET` route (administrators may see every
picture), the reasons with their counts, and the notes as text. For each
picture:
- **Keep:** the open reports on that digest become `kept`, audited as
  `picture.kept`.
- **Remove:**
  - the picture is deleted;
  - its reports become `removed`;
  - a notice is written with a reason from the list and an optional
    sentence of at most 500 characters, shown to the person;
  - optionally, uploads are blocked for N days or until lifted;
  - the audit records `picture.removed`, plus `picture.blocked` when a
    block was set.
- **Lift a block:** audited as `picture.unblocked`.

Every decision needs the fresh password, and `audit_events.detail` holds
reason codes only.

**If the person replaced the picture before a decision**, its reports read
`gone`. The administrator then judges the current picture, or blocks if the
pattern repeats.

**What the person sees** on the profile page: "An administrator removed
your profile picture on {date}. Reason: {reason}. {note} If you think this
is wrong, write to us: {the legal notice's contact}." When a block is set:
"You can choose a new picture from {date}." Terms §6 promises the "why"
and the "how to object". Whether these sentences meet any legal duty (the
DSA's statement of reasons, for instance) is the owner's question; this
document makes no such claim.

### 4.7 Website

**The profile page** (`islands/account.rs`, a new `islands/picture.rs`)
gains a "Profile picture" card:
- the picture at 128 CSS pixels, or the initials avatar;
- "Choose a picture…", then the cropper of §1.1, with a live preview in a
  `<canvas>`;
- "Save", which sends the `PUT` with `fetch`;
- "Remove picture", behind a confirmation;
- the notice and the block of §4.6, when there are any;
- a line: "Your browser makes a square copy without the photo's details
  (place, camera, time); only that copy is sent."

**Errors** are named: too large, cannot be opened here, `Busy` ("try again
in a moment" or "later today"), blocked until a date.

**Behaviour:**
- keyboard: arrows move the square, Shift takes larger steps, a range
  input zooms;
- a live region announces each step;
- no hover that moves or resizes anything (owner, 11.11.2026).

**Where pictures are drawn.** `islands/chat/talk.rs`'s `avatar()` draws
every avatar today: the signed-in person on the chat's side, contact rows,
conversation heads and group members. It gains an optional key, draws
`<img src="/api/v1/pictures/{key}" alt="" loading="lazy">` inside the same
circle, and falls back to the initials when there is no picture or the
image fails. The chat fetches keys with `SocialRequest::Pictures` beside
its contact snapshot.

Also:
- the administration's account page and the report queue show the picture;
- the export section gets its two downloads (§1.6).

**What does not change.** The CSP stays as it is. No inline script, no
`blob:`, no `data:`.

**What is added:**
- web-sys features `CanvasRenderingContext2d`, `File`, `FileList`,
  `ImageBitmap`, `ImageBitmapOptions` and `ImageOrientation`, to be
  approved with this proposal (§3);
- texts in every language the site has.

### 4.8 Later, not in this step

**The native client (M6).** The native wire carries 1 MiB and writes bytes
as JSON numbers, so the client needs a raw route on the native listener,
`PUT /v1/picture` with the bearer and the same checks. The client converts
with its own `image` 0.25.10, which Baylee already links. That is Baylee's
graph, not Central's.

**Gateways.** Pictures for gateways would be a new request,
`GatewayPicture { lease, purpose }`, under `GatewayProfile`'s lease check.
Today a gateway learns the name and tag of whoever signs in there. It would
learn their picture too, and a gateway is a stranger's server. Q5 asks
whether that is ever wanted.

### 4.9 Privacy notice (draft for the owner)

English, to sit beside the binding German that the owner writes:

> **§3** – **Profile picture (optional):** a square picture you choose.
> Your browser makes a copy of it without the photo's details (such as
> place, camera or time), and we turn that copy into a 256 × 256 pixel
> picture that holds nothing else. We keep it with its fingerprint and the
> time you set it, to show your account to your contacts, to people who
> share a group with you, and to our administrators. Your original file
> never leaves your device. *Art. 6(1)(b) GDPR*
>
> **§3** (added to "Reports about content or accounts, moderation
> decisions") – A report about a profile picture holds the reporting
> account, the picture's fingerprint, the reason and the note you write;
> the reported person is not told who reported it.
>
> **§6** – Other users: your contacts, people with whom you have an open
> contact request, and members of groups you are in see your profile
> picture.
>
> **§7** – A profile picture is kept until you replace or remove it, an
> administrator removes it, or you delete your account; it is then deleted
> at once.

The privacy policy needs a new version before this goes live. Its words
"(a profile picture will follow)" go.

### 4.10 Threat model (draft row)

| Threat | Required control | Evidence before release |
| --- | --- | --- |
| A profile picture: a hostile file, a decompression bomb, a polyglot, hidden metadata, offensive content | The page converts to a square PNG; Central takes only that, up to 2 MiB and 256–1024 pixels a side, with its header checked by our code before decoding; `png` within its bounds, one at a time on its own thread, in crates that forbid unsafe; the output made from pixels only (IHDR, IDAT, IEND); served same-origin with `nosniff`, a sandbox CSP and CORP, to related accounts only; reports and administrators' removal with a reason | The hostile-file table, mutation tests, header tests, the visibility matrix across accounts, the route's headers |

### 4.11 Tests

**`central-picture`.**
- *Accepted:*
  - every colour type and bit depth: grey 1, 2, 4, 8 and 16; grey with
    alpha 8 and 16; RGB 8 and 16; RGBA 8 and 16; palette 1, 2, 4 and 8,
    with and without `tRNS`;
  - interlaced input;
  - sides 256, 257, 511, 512 and 1024.
- *Every output:*
  - is 256 × 256, decodes again to the same pixels (the round trip), and
    holds only IHDR, IDAT and IEND;
  - is RGB when opaque and RGBA otherwise;
  - has colour 0 under alpha 0;
  - is the same bytes twice, and gives itself back when processed again;
  - averages exactly: a checkerboard becomes even grey, and halves stay
    halves.
- *Refused before decoding:* not a PNG; IHDR not first or not 13 bytes
  long; a side of 0, 255 or 1025; not square; an invalid depth and colour
  pair; compression or filter other than 0; interlace above 1; `acTL`.
- *Refused while decoding:*
  - a CRC error in any chunk;
  - every prefix of a valid file;
  - a bad zlib header or Adler-32;
  - too little image data;
  - a palette image without PLTE, or a `tRNS` of the wrong length.
- *Bounded, each within a time bound:* the IDAT and zTXt bombs; `iCCP`
  bombs; `eXIf`, script text and bytes after IEND absent from the output.
- *Seeded mutations in `cargo test`* (no new dependency):
  - fixed seeds and a small xorshift in the test code;
  - byte flips, insertions, deletions and chunk-length edits, with CRCs
    corrected so that mutations reach the decoder;
  - thousands of cases from valid and hostile seeds;
  - each case is either a refusal or a valid 256-pixel PNG, never a panic,
    within a time bound.

**`central-http`.**
- every header refusal happens before the body is read and before any slot
  is taken;
- `Content-Length` missing or too large; a slow body (`408`); a body of
  the wrong length; another content type;
- `GET` without a cookie, with another account's cookie, and with an
  unknown key: all the same `404`;
- the answer's headers byte for byte;
- `HEAD` and other methods `405`;
- the `GET` budget, apart from `POST /api/v1`'s places.

**`central-postgres` fixture.**
- set, replace and remove; a new key each time, and the old key `404`;
- the visibility matrix: self; friend; incoming; outgoing; group member;
  blocked either way; stranger; administrator; signed out;
- reports: one's own refused; not visible refused with the same answer as
  unknown; a duplicate open report `Conflict`; the rate `Busy`;
- decisions: only with a fresh password; the exact audit rows; `gone` on
  replacement;
- a block that refuses uploads until its end, and lifting it;
- deletion: the cascade, and `SET NULL` so no row names the account;
- the export answers;
- the grants, both ways;
- the sweep;
- the previous binary over the migrated schema.

**Website.**
- crop arithmetic (unit tests);
- Playwright on desktop and Pixel 7: PNG, JPEG and WebP fixtures generated
  in the test; cropping by keyboard and by pointer; save; the picture in
  the chat and on the profile; remove; every refusal text; no CSP
  violation;
- `web/e2e-real` against a real Central on loopback.

**Gates.** `cargo xtask gate`, `policy`, `postgres`, `smoke` and `secrets`,
`cargo deny check` and `cargo audit --deny warnings`.

### 4.12 Order

1. The owner's answers and acceptance.
2. The next design round, as [reports](../reports.md)' "Pass 2" was: the
   exact routes, schema, grants and wire, including the refusal status of
   §4.3. Then the implementation and adversarial passes of [security
   assurance](../security-assurance.md).
3. `central-picture` with its tests.
4. The migration, store, service, routes and administration requests.
5. The website.
6. The privacy policy's new version.
7. M6: the native route and the client.

Then the docs: [web accounts](../web-accounts.md),
[administration](../administration.md), the threat model, `postgresql.md`,
`operations.md` (grants, sweep), `status.md` and the roadmap.

## 5. Questions for the owner

Q2 and Q4 apply only if Q1 is answered (a). For (b) or (c), this proposal
gets a second inventory and other bounds (originals of many megapixels)
before anything is accepted.

**Q1. Which formats does the server decode?**
- (a) **PNG only; the page, and later the client, converts any picture
  first.** *Recommended.*
- (b) PNG, JPEG and WebP on the server, with scalar `zune-jpeg` and
  `image-webp`. Five more packages, an EXIF reader of our own, and
  `zune-core`'s commit is missing upstream.
- (c) The `image` crate. Eighteen packages, SIMD JPEG decoding, and an
  unused SIMD colour library.

**Q2 (only if Q1 is a). Accept these exact packages?**
- (a) **The seven packages with `flate2` 1.1.9 and `crc32fast` 1.5.0, with
  no gate exception, plus the six web-sys features.** *Recommended.*
- (b) The same with `flate2` 1.1.11. That adds `miniz_oxide` 0.9.1, a
  `deny.toml` duplicate skip, and `zlib-rs` 0.6.8 approved though never
  built.
- (c) Not now.

**Q3. The stored picture?**
- (a) **PNG, 256 × 256, about 100–145 KB.** *Recommended.*
- (b) PNG, 192 × 192, about 57–80 KB.
- (c) PNG, 128 × 128, about 27–37 KB; soft on the profile page.
- (d) JPEG, 256 × 256, about a quarter of (a)'s bytes. It loses
  transparency and needs `jpeg-encoder` with an IJG licence exception.

**Q4 (only if Q1 is a). Upload bounds?**
- (a) **2 MiB, square, 256–1024 pixels a side.** *Recommended.*
- (b) 1 MiB, 256–512 pixels.
- (c) 4 MiB, 256–2048 pixels.

**Q5. Who sees a picture?**
- (a) **The account, its contacts, both sides of open requests, members of
  its groups, and administrators.** *Recommended.*
- (b) Every signed-in account that knows the handle, as for the display
  name.
- (c) As (a), and later the gateways the account signs in to.

**Q6. Where is the picture decoded?**
- (a) **In the service, on a blocking thread, one at a time, within the
  bounds.** *Recommended.*
- (b) In a child process of the same binary for each upload, killed after
  two seconds. It also contains a memory-corruption bug in the checksum
  crates and ends a hang, at the cost of more code and a process start per
  upload.

**Q7. What does a report hold?**
- (a) **A reason from the list and an optional note of up to 1000
  characters. Administrators see the reporter; the reported person never
  does.** *Recommended.*
- (b) A reason only, no note.
- (c) As (a), but the reporter is not stored. That leaves no defence
  against one account flooding reports.

**Q8. What can an administrator do?**
- (a) **Remove the picture with a reason the person is told, and
  optionally block new uploads for some days or until lifted.**
  *Recommended.*
- (b) Remove only.

**Q9. What happens to a removed picture's bytes?**
- (a) **They are deleted at once; only the digest stays with the
  reports.** *Recommended (data minimisation).*
- (b) They are kept hidden for 90 days in case an authority asks, then
  deleted. That needs the owner's legal check.

**Q10. How long are decided picture reports kept?**
- (a) **One year after the decision.** *Recommended.*
- (b) Three years, the privacy notice's maximum, as step 8 planned.
- (c) 90 days after the decision.

## 6. Verified, assumed, not verified

**Verified, with commands and probes:**
- the resolved graphs of A, B and C, with their features;
- `cargo deny` and `cargo audit` on all three graphs;
- the source diffs of `crc32fast` 1.5.0 to 1.5.5, and the path `png`'s
  encoder takes through `flate2`, both read in source;
- the RustSec history, and the GitHub advisories for six repositories;
- the marker counts;
- the compiled-unsafe reasoning for `zune-jpeg` (`cargo check` with only
  `std`) and for `flate2`'s and the checksum crates' cfgs, read in source;
- provenance of the seven packages, file by file, and of Option B's
  `zune-jpeg`, `image-webp`, `byteorder-lite` and `quick-error`;
- `zune-core` 0.5.3's missing commit;
- `png`'s API for limits, ignored chunks, APNG and normalisation;
- the pipeline's timings, sizes and hostile-file results on this Mac;
- the JPEG and WebP probe;
- OSS-Fuzz's `image-rs`, `image-png` and `miniz_oxide` projects;
- `moxcms`'s reach inside `image`;
- the IJG licence text in `image`'s encoder;
- that Baylee's lock has `image` 0.25.10 and `png` 0.18.1.

**Assumed, to be checked while building.** The recommendation rests most on
the first four, and none of them was run in a browser here:
- `createImageBitmap`'s orientation handling;
- browsers' canvas colour management (conversion to sRGB);
- canvas read-back in hardened browsers (Firefox `resistFingerprinting`,
  Tor, Brave, Safari);
- Safari's HEIC-to-JPEG hand-over.

If one of them fails, the page can still upload, but the person sees a
turned, dull, noisy or blank picture; the end-to-end tests decide.

Also assumed:
- the server's timings, which differ from this Mac's;
- the 12 MiB memory estimate per upload: probe RSS plus reading, not
  measured inside the service.

**Not verified:**
- tags for five of the seven packages, and the release notes of `crc32fast`
  1.5.1–1.5.5 (the GitHub API quota was spent, and there is no changelog
  file);
- many-scan progressive JPEGs and large-canvas lossless WebPs below the
  bounds (Option B only);
- whether §1.6's export and §4.6's notice meet the GDPR's and the DSA's
  duties, which is the owner's legal check;
- a third-party audit of any package.
