# Profile pictures: dependency proposal and design

**Status: second version of 12.11.2026, with the owner's decisions of the
same day (below). The build stopped at its first step: the accepted AVIF
decoder cannot be called without first-party `unsafe` (§0.2). Nothing is
built.** No `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.cargo/audit.toml` or
`approved-packages.json` of the repository was changed. All measurements
ran in a copy of the tree, or in small programs outside the repository.

The owner must accept the exact packages, versions, features and gate
changes in §5 before adoption (AGENTS.md: "any required dependency
containing unsafe/assembly must be disclosed and explicitly accepted").
This is the design pass, the first of the three in [security
assurance](../security-assurance.md): assets, caller trust, failure modes and
invariants, before any code. §8 holds the questions the owner answered on
12.11.2026 (§0).

## 0. The owner's decisions (12.11.2026, forms), binding

Relayed by the project lead on 12.11.2026. The quoted words are the
owner's, as relayed; the rest is the relay's wording.

- **Formats:** "Alles was HTML5 fähig ist, darf hochgeladen werden. Der
  Server wandelt es in ein modernes Format um. (AVIF ...). (Und ...
  Fallback Format für Toaster Browser). So Bilduploads sind idr. immer
  sicherheitskritisch. Bitte besonders gut überprüfen und testen."
- **Where decoded:** **"Beides"**: the page decodes in the browser and
  uploads a normalised lossless square PNG (design A), **and** Central also
  accepts original files and decodes them itself (design B), in the
  sandboxed worker, for exactly these formats, whose packages the owner
  accepted:
  - **PNG** (`png` 0.18.1);
  - **JPEG, GIF, WebP** (`zune-jpeg` 0.5.15 scalar, without SIMD; `gif`
    0.14.2; `image-webp` 0.2.4);
  - **AVIF** (`rav1d` 1.1.0, the Rust port of dav1d, never the C
    libdav1d);
  - **SVG** (`resvg` 0.50.0 without default features: no text, no system
    fonts, no svgz, no embedded rasters, external references refused, a
    time limit; SVG is only ever rasterised, never served as SVG);
  - **BMP, ICO** (through `image` 0.25.10 with only the bmp and ico
    features).

  Each decoder runs only in the worker process.
- **Encoder:** "Freigeben, Assembler an": the newest `ravif` 0.13.0 and
  `rav1e` 0.8.1 **with rav1e's assembly on** (nasm on x86-64; documented
  for CI and the server), threads off. All the gate changes §5.4 lists:
  - the licence exceptions BSD-2-Clause for `rav1e`, `av1-grain` and
    `v_frame`, and whatever the decoders above need; NCSA for
    `libfuzzer-sys` if it appears;
  - the duplicate skips;
  - the `paste` advisory exception widened as §5.3 says;
  - the six web-sys features;
  - `blob:` in the profile page's image CSP.

  "Am liebsten sehr schnelle stabile und gepflegte Crates auf der neusten
  Version": the newest releases, unless a newer one breaks the gate in a way
  this proposal did not foresee.
- **Output:** AVIF at 256 px, fallback **PNG 256 px**.
- **Visibility:** **also signed-out visitors**:
  - public URLs with an unguessable key;
  - no enumeration by account id;
  - removal and account deletion make the URL 404 at once;
  - caching headers that do not keep a removed picture alive longer than
    this proposal's bound (one hour, §3.5).
- **Conversion:** **a fresh sandboxed process per upload** (systemd socket
  activation in production as §6.2 designs it, a child process in
  development; no secrets, no network, memory and time limits).
- **Reports and moderation:**
  - a reason from a fixed list plus an optional note of up to 1000
    characters;
  - administrators see the reporter; the reported person never does;
  - administrators remove with a reason the person is told, optionally
    blocking uploads for some days or until lifted;
  - the bytes are deleted at once, and only the digest stays with the
    report;
  - decided reports are kept **one year**.

The decisions answer §8's questions:

| Q | Answer |
| --- | --- |
| Q1 | both (a) and (b) |
| Q2 | (b): the newest releases, assembly on |
| Q3 | (a): PNG, 256 px |
| Q4 | (b): also signed-out visitors |
| Q5 | (a): a fresh process per upload |
| Q6 | (a) |
| Q7 | (a) |
| Q8 | (a): one year |

### 0.1 What the decisions change in this document

The sections below still describe design A alone. The build rewrites them
in its design pass; until then, these points take precedence over them:

- **§1, §2.3, §3.2:** Central decodes originals too. Caps per format apply
  before any decoder runs, and the type is recognised by its magic bytes,
  not by the declared type or the name. Only the first frame of an
  animation is used, and JPEG's EXIF orientation is applied by our own
  reader.
- **§3.5, §7.5:** pictures are public. The `GET` routes ask for no cookie,
  so a block between two accounts can only withhold the key from
  `SocialRequest::Pictures`, never the bytes. Keys are still learned only
  from answers that name an account already, and never from the account id
  or the tag. The visibility matrix becomes a matrix of who learns a key.
- **§4.2, §5:** `rav1e`'s assembly is on: nasm on x86-64 (CI's Linux job
  and the server's build) and the C compiler on aarch64.
- **§6.2, §7.1:** the worker also holds the decoders of design B. So the
  codecs live in a crate of their own, linked only by the worker binary,
  and never by the service or by anything `central-client` reaches. The
  service links only the protocol and the answer checks.
- **§8:** answered as in the table above.

### 0.2 Where the build stopped: AVIF originals cannot be decoded with the accepted package

**Finding (12.11.2026).** `rav1d` 1.1.0, the newest release, has no safe
Rust API.
- Its only public decoding functions are the C ABI:
  - `pub unsafe extern "C" fn dav1d_open`, `dav1d_send_data`,
    `dav1d_get_picture`, `dav1d_close` and the others, in `src/lib.rs`
    lines 138 to 931 of the packaged crate;
  - the Rust functions behind them (`rav1d_open`, `rav1d_send_data`,
    `rav1d_get_picture`) are `pub(crate)`.
- Calling it would need `unsafe` blocks in our own code. AGENTS.md forbids
  that, also in a helper crate.
- The accepted set also holds no parser for the AVIF container: `rav1d`
  decodes an AV1 bitstream, and the HEIF items (`iloc`, `pitm`, the alpha
  auxiliary, `irot`/`imir`, grids) must be read first.

§2.2 counted `rav1d`'s unsafe sites and assembly, but did not foresee that
its API is unsafe-only. The task's rule is to stop and report rather than
weaken a decision, so nothing was built: no package was added, and AVIF
originals were not silently dropped.

**Everything else in the accepted set is usable.** Checked in a copy of
the tree (lock at `ecaacfc` plus a probe crate under
`#![forbid(unsafe_code)]` and the workspace lints):
- **The probe**, built in release on this Mac (aarch64), decoded synthetic
  files (640 × 480; the ICO 128 × 128) through safe APIs only:
  - PNG, JPEG (EXIF found), GIF, WebP, BMP and ICO;
  - an SVG with a `<script>`, an `xlink:href="/etc/passwd"` and a remote
    `href`, rasterised at 256 × 256 with an `ImageHrefResolver` that
    resolves nothing (whether anything else in `usvg` touches the
    filesystem was not traced);
  - and it encoded a 256² AVIF and PNG with `ravif`. `rav1e`'s assembly
    was built by `cc` on aarch64 and is linked: its `_neon` symbols are in
    the binary. A flat grey square took 0.09 s of CPU (0.72 s wall, under
    a load average of 33) and peaked at 11.4 MB.
  - nasm is not installed here, so the x86-64 assembly was not built.
- **The exact manifest lines:**
  - `png = "=0.18.1"`;
  - `zune-jpeg = { version = "=0.5.15", default-features = false,
    features = ["std"] }` and `zune-core = { version = "=0.5.3",
    default-features = false, features = ["std"] }`;
  - `gif = { version = "=0.14.2", default-features = false, features =
    ["std"] }`;
  - `image-webp = "=0.2.4"`;
  - `resvg = { version = "=0.50.0", default-features = false }`;
  - `image = { version = "=0.25.10", default-features = false, features
    = ["bmp", "ico"] }`;
  - `ravif = { version = "=0.13.0", default-features = false, features =
    ["asm"] }`.
- **The lock** gains 74 packages and changes no existing version. One
  existing entry's reference to `nom` gains its version (`nom 7.1.3`),
  since two versions are then in the lock. The packages:
  - adler2 2.0.1, aligned 0.4.3, aligned-vec 0.6.4, arbitrary 1.5.0,
    arg_enum_proc_macro 0.3.4, arrayref 0.3.9, as-slice 0.2.1,
    av-scenechange 0.14.1, av1-grain 0.2.5, avif-serialize 0.8.9,
    bitstream-io 4.10.0, built 0.8.1, bytemuck 1.25.2, byteorder-lite
    0.1.0, crc32fast 1.5.5, crossbeam-deque 0.8.8, crossbeam-epoch 0.9.21,
    data-url 0.3.2, equator 0.4.2, equator-macro 0.4.2, euclid 0.22.14,
    fdeflate 0.3.7, flate2 1.1.11, float-cmp 0.9.0, gif 0.14.2, image
    0.25.10, image-webp 0.2.4, imagesize 0.15.0, imgref 1.12.3,
    interpolate_name 0.2.4, kurbo 0.13.1, libfuzzer-sys 0.4.13, loop9
    0.1.5, maybe-rayon 0.1.1, miniz_oxide 0.8.9 and 0.9.1, moxcms 0.8.1,
    nasm-rs 0.3.2, new_debug_unreachable 1.0.6, no_std_io2 0.9.4, nom
    8.0.0, noop_proc_macro 0.3.0, num-derive 0.4.2, num-rational 0.4.2,
    pastey 0.1.1, pico-args 0.5.0, png 0.18.1, polycool 0.4.0, profiling
    1.0.18, profiling-procmacros 1.0.18, pxfm 0.1.30, quick-error 2.0.1,
    rav1e 0.8.1, ravif 0.13.0, rayon 1.12.0, rayon-core 1.13.0, resvg
    0.50.0, rgb 0.8.53, roxmltree 0.21.2, simd-adler32 0.3.10,
    simd_helpers 0.1.0, simplecss 0.2.2, siphasher 1.0.4, strict-num
    0.1.1, svgtypes 0.16.2, tiny-skia 0.12.0, tiny-skia-path 0.12.0, usvg
    0.50.0, v_frame 0.3.9, weezl 0.1.12, y4m 0.8.0, zlib-rs 0.6.8,
    zune-core 0.5.3, zune-jpeg 0.5.15.
- **Features that matter:**
  - `zune-jpeg` and `zune-core` resolve with `std` only, so no SIMD;
  - `rav1e` with `asm`, `cc` and `nasm-rs`;
  - `resvg` and `usvg` with no features, so no text, fonts, svgz,
    rasters or writer;
  - `image` with `bmp`, `ico` and `png` (ICO entries may be PNGs; the same
    `png` 0.18.1);
  - SIMD on by default in `tiny-skia` (`simd`), `moxcms` (avx, sse and
    neon paths, through `image`) and `bytemuck` (`aarch64_simd`).
- **Approved packages that gain features (8).** These, and the 74 above,
  come from `cargo metadata` compared with `approved-packages.json`:
  `cargo xtask policy` stops at the first package it does not find
  approved (`adler2@2.0.1`), so it was not run to a pass.
  - `arrayvec` 0.7.8: default;
  - `crossbeam-utils` 0.8.23: default;
  - `log` 0.4.35: alloc, std;
  - `num-traits` 0.2.19: default;
  - `rand` 0.9.5: default, os_rng, small_rng, std_rng, thread_rng;
  - `rand_chacha` 0.9.0: default;
  - `rand_core` 0.9.5: os_rng;
  - `syn` 2.0.119: fold.
- **`cargo deny check`:** advisories and sources pass. It fails exactly
  where the decisions already accept an exception:
  - licences: BSD-2-Clause for `rav1e`, `av1-grain`, `v_frame` and
    **`arrayref`** (through `tiny-skia`, for `resvg`); NCSA for
    `libfuzzer-sys`;
  - duplicates: `getrandom`, `miniz_oxide`, `nom`, `r-efi`, `rand` and
    `rand_core`, which are §5.4's six skips.

  The existing `paste` ignore already covers `rav1e`'s use; only its
  reason changes.
- **`cargo audit --deny warnings`** with the repository's three ignores
  exits 0 (549 packages).
- **New since §5:** `nasm-rs` 0.3.2 (`rav1e`'s build dependency with
  `asm`, which runs the external nasm on x86-64), and the decoders'
  packages.

**What the owner can choose (one of):**

- (a) **No AVIF originals at Central.** An AVIF file is decoded by the
  browser, as design A does for every format the browser shows; Central
  refuses an original whose magic bytes say AVIF, with a refusal of its
  own. It needs no further package. Every browser that shows AVIF can
  convert it; a hardened browser that blanks the canvas cannot send an
  AVIF original instead.
- (b) **`avif-decode` 3.0.0** (kornelski, who also maintains `ravif`;
  BSD-3-Clause). It is a safe API (`Decoder::from_avif(&[u8])`,
  `to_image()`) over the accepted `rav1d` 1.1.0 and `avif-parse` 2.1.0.
  - **Checked in the same copy:** our probe called it under
    `#![forbid(unsafe_code)]` and decoded a 640 × 480 AVIF in 92 ms; a
    truncated file and a PNG were refused.
  - **It adds 16 packages:** assert_matches 1.5.0, atomig 0.4.3,
    atomig-macro 0.4.0, avif-decode 3.0.0, avif-parse 2.1.0, bitreader
    0.3.11, fallible_collections 0.5.2, leb128 0.2.7, rav1d 1.1.0,
    raw-cpuid 11.6.0, strum 0.26.3, strum_macros 0.26.4, to_method 1.1.0,
    yuv 0.8.20, zerocopy 0.7.35, zerocopy-derive 0.7.35.
  - **Unsafe markers in source:** `avif-decode` 11 (its calls into
    `rav1d`'s C ABI), `avif-parse` 4, `yuv` 874 (SIMD),
    `fallible_collections` 129, `zerocopy` 0.7 151, `rav1d` 510 in 36
    files with 91 assembly files.
  - **`rav1d`'s assembly is always on:** `avif-decode` asks for `rav1d`'s
    default features (asm, the arm64 variants, both bit depths), and
    features only add. So nasm is needed on x86-64 for `rav1d` too.
  - **Further gate exceptions:**
    - licences: MPL-2.0 for `avif-parse`, BSD-2-Clause for `rav1d`,
      CC0-1.0 for `to_method`;
    - duplicates: `strum` (0.26 and the workspace's) and `zerocopy` (0.7
      and 0.8);
    - `cargo audit` still exits 0 (565 packages).
  - It is a package the decisions did not name, so the owner must accept
    it.
- (c) **Wait** for a `rav1d` release with its Rust API. Upstream's main
  branch has one; `sqzer-rav1d` 0.1.0 republishes that branch under
  another name. A third party's republication is not recommended.

Not usable:
- `rav1d-safe` 0.6.0 is `AGPL-3.0-only OR` commercial, and an AGPL crate
  cannot enter the source-available crates (LICENSE.md §6);
- `re_rav1d` 0.1.3 and `rusty_av1d` 1.3.0 are forks of the same `rav1d`,
  and were not examined further.

**Recommended: (a).** It keeps strangers' AVIF bytes away from Central
altogether: the browser that can show an AVIF converts it. (b) adds a
second assembly decoder and about 1,700 unsafe markers in source for the
few people whose browser shows an AVIF but will not let the page read its
canvas.

## The owner's first answers (12.11.2026, form)

The owner's answers to the first version, verbatim:

- **Formats:** "Alles was HTML5 fähig ist, darf hochgeladen werden. Der
  Server wandelt es in ein modernes Format um. (AVIF glaube ich). (Und
  vielleicht noch ein gutes passendes Fallback Format für Toaster Browser).
  So Bilduploads sind idr. immer sicherheitskritisch. Bitte besonders gut
  überprüfen und testen."
- **Packages:** "Durch die vorherige Antwort ändern sich evtl. die
  Bedingungen. Am liebsten sehr schnelle stabile und gepflegte Crates auf
  der neusten Version."
- **Size:** "256px, aber nicht unbedingt PNG".
- **Who sees a picture:** "Alle".

The decision of the same day stands: pictures are uploaded, the server
decodes and re-encodes them, and there are size limits, reports and
moderation by administrators.

**What changed against the first version** (commit `cd98291`):
- the output is AVIF at 256 px, with a fallback;
- every package is at its newest release;
- every signed-in account sees pictures;
- decoding and encoding move into a separate, sandboxed process.

The terms already name the profile picture and the report function (§6), so
they need no new version.

## In short

- **Recommended: design A.** The person may pick any picture their browser
  can display: PNG, JPEG, GIF, WebP, AVIF, BMP, ICO, SVG, and in Safari also
  HEIC and JPEG XL.
  - The page decodes it in the browser, applies its orientation, lets the
    person choose a square, and uploads only a lossless square PNG.
  - Central decodes only that PNG and scales it to 256 × 256.
  - It writes an **AVIF**, plus a **PNG of the same 256 px** as the lossless
    master and the fallback.
  - The original file, with its EXIF, location and camera data, never
    leaves the device.
- **Design B**, where Central decodes every format itself, means 98 new
  packages, a C library (libdav1d) or a Rust port with 497 unsafe sites and
  assembly, an SVG renderer, and every decoder exposed to strangers' bytes
  (§2).
- **Packages for A, each at the newest release the graph allows.**
  - `png` 0.18.1 and `ravif` 0.13.0, which brings `rav1e` 0.8.1, the
    established pure-Rust AV1 encoder. The only other pure-Rust candidate,
    `gamut-avif`, is five months old and six to eight times slower (§4.1).
  - 46 new packages, of which 42 are compiled; four sit in the lock only.
  - Seven approved packages gain features.
- **Gate changes, all tested in the scratch copy.** With them, `cargo deny
  check` and `cargo xtask policy` pass and `cargo audit --deny warnings`
  exits 0:
  - four per-crate licence exceptions: BSD-2-Clause for `rav1e`,
    `av1-grain` and `v_frame`; NCSA for `libfuzzer-sys`, which is never
    built;
  - six duplicate skips;
  - widening the existing `paste` advisory exception (RUSTSEC-2024-0436) to
    cover `rav1e`.
- **Speed (Apple Silicon, one thread, sampled).** `rav1e`'s own assembly stays off:
  - a 256 px AVIF takes 0.17–0.25 s of CPU and is 6–16 KB;
  - the worst case (noise with alpha) takes 0.39 s and is 87 KB;
  - with the assembly on, it is about twice as fast and byte-identical.
- **Unsafe that remains:** the encoder (`rav1e`) and its helpers hold about
  1,100 unsafe sites in their source, of which at most about 870 are
  compiled.
  The PNG decoding path keeps only the two SIMD checksum crates.
  - The encoder works on pixels whose values the uploader chose, at a fixed
    256 × 256.
  - So both decoding and encoding run in **a fresh process per upload, with
    no secrets, no network and bounded memory and time** (§6.2). It needs
    no new crate.
- **Visibility.** Every signed-in account and administrators. Pictures are
  served same-origin under a random key that changes at every upload. Not
  to signed-out visitors, and not to gateways unless the owner says so (Q4).

## 1. What the answers mean for the design

**"Alles was HTML5 fähig ist."** Two readings are possible:
- the browser may open the file (design A);
- the server must parse it (design B).

Both reach the outcome the owner names: Central ends up with a modern format
it made itself.

They differ in mechanism. In A the original is never uploaded: the server
converts a lossless PNG of what the browser showed, not the original file.
If "the server converts it" is meant literally, that is the owner's call
(Q1). §2 compares the two honestly.

**"Besonders gut überprüfen und testen."**
- §2 to §5 report what was run.
- §9 separates the verified, the assumed and the unmeasured.
- §7.10 lists a hostile-file test for every input path.

**"Sehr schnelle stabile und gepflegte Crates auf der neusten Version."**
- Every package is at the newest release its dependents allow:
  `cargo update --dry-run --verbose` moves none of them. Where a newer
  major version exists, such as `rand` 0.10, a dependent's requirement holds
  it back.
- The cost of that in this workspace is spelled out in §5.4.
- One concrete reason against "newest" is reported rather than acted on:
  `crc32fast` 1.5.5 (Q2).

**"Alle."** I read it as every signed-in account. Signed-out visitors and
gateways are a further step that the answer does not settle (Q4).

## 2. Design A against design B

### 2.1 Design A: the browser converts, the server checks

1. The page opens the chosen file with `createImageBitmap(file, {
   imageOrientation: "from-image" })`. If that fails, as it does for SVG,
   it uses an `<img>` with a `blob:` address of the file.
2. The person places a square: by dragging, with the arrow keys, and with a
   zoom slider.
3. The page draws the square on a canvas of 512 px, or of the square's own
   size if smaller but at least 256, and uploads `canvas.toBlob("image/png")`.
4. Central checks that PNG (§3.2), decodes it, scales it to 256 × 256, and
   writes the AVIF and the PNG master (§3.3).

**Checked in a browser on 12.11.2026.** A loopback page in the desktop
app's browser pane (Chromium 156) showed:
- a JPEG with EXIF orientation 6 (300 × 200 stored) came out of
  `createImageBitmap` as 200 × 300, so the orientation was applied;
- `toBlob("image/png")` gave an 8-bit, RGBA (colour type 6),
  non-interlaced PNG for every input, the opaque JPEG included;
- an AVIF and an animated WebP decoded to a still;
- `createImageBitmap` refused an SVG, but the `<img>` and `blob:` path drew
  it, the canvas was not tainted, and the script inside the SVG did not run.

Firefox and WebKit were not checked. Their behaviour, and canvas read-back
in hardened browsers, are assumptions for the end-to-end tests (§9).

**What A costs:**
- **The site's CSP gains `blob:` in `img-src`**, sent on the profile page
  only. A CSP cannot limit it to SVG; the page uses it only for SVG. It makes a
  `blob:` address from the file the person chose, draws it, and revokes
  it. An SVG drawn through `<img>` runs no script and loads nothing from
  elsewhere.
- **Hardened browsers.** Firefox with `resistFingerprinting` and Tor
  Browser ask before a page may read a canvas, and give a blank picture
  without permission; Brave and Safari add faint noise. This comes from
  those browsers' documentation and was not run. The page says so, and the
  person sees the stored picture straight after saving.
- **The native client (M6) must produce the PNG itself.** Baylee already
  links `image` 0.25.10.
- **Colour.** The browser manages it (assumed for canvas drawing). Central
  treats the PNG as sRGB.

### 2.2 Design B: the server decodes every format

Each format's decoder was resolved against the repository's lock in the
scratch copy, newest versions, with `cargo deny` and `cargo audit` run on
each:

| Format | Decoder | New packages | Unsafe in source (sites) | Native code | Gate |
| --- | --- | ---: | --- | --- | --- |
| PNG | `png` 0.18.1 | 9 | `png` and `fdeflate` forbid; the SIMD checksums have 23 + 36 | none | `miniz_oxide` duplicate |
| JPEG | `zune-jpeg` 0.5.15 (scalar only if used directly; `image` turns SIMD on) | 2 | 79 sites, none compiled without `x86` and `neon` | none | `zune-core` 0.5.3's packaged commit is not on GitHub (checked 12.11.2026) |
| GIF | `gif` 0.14.2 | 3 (`gif`, `weezl`, `color_quant`) | 0; `gif` and `weezl` forbid | none | passes |
| WebP | `image-webp` 0.2.4 | 3 | 0, forbid | none | passes |
| BMP, ICO | `image` 0.25.10 (bmp, ico) | 14 | `image` 8, `bytemuck` 348, `moxcms` 344, `pxfm` 224 | none | `miniz_oxide` duplicate |
| AVIF (`image`'s `avif-native`) | `dav1d` 0.11.1, `dav1d-sys` 0.8.3, `mp4parse` 0.17.0 | 19 | `dav1d` 49, `fallible_collections` 127, `hashbrown` 0.13 232 | **the system's libdav1d (C and assembly)**, found through pkg-config and at least 1.3.0; CVE-2024-1580 (memory corruption, fixed in 1.4.0) | MPL-2.0 (`mp4parse`), `Apache-2.0 WITH LLVM-exception`, `hashbrown` duplicate |
| AVIF, the alternative | `rav1d` 1.1.0, a Rust port of dav1d | 11 | `rav1d` 497 in 33 files; `zerocopy` 0.7 114 | 91 assembly files (on by default; nasm on x86-64) | BSD-2-Clause, CC0, `strum` duplicate |
| SVG | `resvg` 0.50.0 without default features | 28 | `tiny-skia` 151 (SIMD), `euclid` 44, `rgb` 77 | none | BSD-2-Clause (`arrayref`), `roxmltree` duplicate |
| SVG with defaults (text, system fonts, svgz, embedded rasters) | `resvg` 0.50.0 | 48 | adds `memmap2` 142 and about 240,000 lines of font code (`read-fonts`, `skrifa`, `harfrust`) | none | as above |
| HEIC, JPEG XL, TIFF | Safari only | — | — | libheif and libde265 are C | not considered |

All of them together (`image` with png, jpeg, gif, webp, bmp, ico, avif and
avif-native, plus `resvg` with defaults):
- 98 new packages, and 12 approved packages gaining features;
- `cargo deny` fails on eight duplicates and seven licences;
- one system C library to install and keep patched.

`cargo audit` reports nothing for any of these graphs.

**SVG in B.**
- Central would never serve SVG; it would rasterise it.
- That is an XML parser, a CSS parser, path and filter rendering, and
  optionally fonts, all fed by strangers' bytes.
- Filters and blurs can cost seconds of CPU, so only a timeout bounds them.
- External references (images, fonts, `href`) must be refused, and gzip
  (`svgz`) brings decompression bombs.

In design A the browser does this, in its own sandbox, for the person's own
file.

### 2.3 Comparison and recommendation

| | A: the browser converts | B: the server decodes everything |
| --- | --- | --- |
| Formats a person can use | whatever their browser shows, Safari's HEIC and JPEG XL included | only what Central compiles in |
| Parsers on strangers' bytes at Central | PNG only (`png` and `fdeflate`, both forbid unsafe) | eight or more formats, including SVG and AV1 |
| Native code at Central | none | libdav1d (C, assembly), or `rav1d`'s assembly |
| New packages | 46 (42 built) | about 98 for decoding and encoding |
| Gate changes | 4 licence exceptions, 6 skips, `paste` widened | more, plus MPL-2.0 and a system library |
| Location and camera data reach Central | never | always, then stripped |
| Depends on the browser | yes (orientation, canvas, hardened browsers) | no |
| The literal "the server converts" | from a lossless PNG of what the browser showed | from the original |

**Recommendation: A.** It meets the owner's security demand best: the only
parser Central exposes is one that forbids unsafe, and the original's
metadata never arrives. It covers more formats than B could. Q1 asks the
owner to confirm.

## 3. Requirements (design A)

### 3.1 What a person can choose

- **The file:** any file the browser opens as a picture, up to 25 MiB. That
  bound only protects the person's own device.
  - The picker offers `image/*`.
  - The page tries `createImageBitmap`, then the `<img>` and `blob:` path.
  - Failing both, it says that this browser cannot open the file.
- **Animation:** an animated GIF, WebP, AVIF or APNG becomes its first
  frame (an animated WebP was checked).
- **Huge files:** where the page can read the dimensions from the header
  first (PNG, JPEG, GIF, WebP, BMP, with small parsers of our own in the
  islands' WebAssembly), it refuses sides above 16,384 before decoding.
  Elsewhere the browser's own limits apply.

### 3.2 What Central accepts

**The request.** A square PNG, 256 to 1024 px a side:
- `Content-Type: image/png`;
- `Content-Length` of at most 2 MiB;
- the whole body within 20 seconds.

**Checks of our own, before the decoder runs:** the signature; IHDR first,
13 bytes long; equal sides; a valid depth and colour pair; compression 0,
filter 0, interlace 0 or 1.

**The decoder** (`png` 0.18.1):
- `Limits { bytes: 16 MiB }`;
- text and ICC chunks skipped unread;
- `acTL` (an animated PNG) refused;
- the first frame only, normalised to 8 bits;
- CRCs and the Adler-32 checked.

**Possible tightening.** Every browser checked so far emits 8-bit RGBA
without interlacing, so Central could accept only that. It does not yet:
it keeps accepting every valid PNG until the end-to-end tests show what
Firefox and WebKit emit. The native client would have to emit the same
narrow form.

**Bounds, measured** with a probe of `png` 0.18.1 on this Mac, on synthetic
hostile files:

| File | Result | Peak memory |
| --- | --- | ---: |
| IHDR 1024², an IDAT inflating to about 400 MB (398 KB file) | decoded to the picture's size; the rest never read; 1.5 ms | 11.4 MB |
| a valid 256² with a zTXt inflating to about 400 MB | chunk skipped unread | 3.3 MB |
| IHDR 100,000 × 100,000 | refused before any pixel buffer | 1.8 MB |
| `acTL` before IDAT (an animated PNG) | refused | 1.9 MB |
| a valid PNG followed by an HTML document | output byte-identical to the same picture without it | 2.9 MB |
| `eXIf` and a `tEXt` holding `<script>` | output byte-identical to the same picture without them | 3.2 MB |
| one bit changed in IDAT's CRC | refused (CRC error) | 2.4 MB |
| cut in half | refused (unexpected end) | 2.1 MB |

Every output held exactly IHDR, IDAT and IEND. A macOS input carried
`sRGB`, `eXIf`, `pHYs` and `iTXt`, and none of them survived.

### 3.3 What Central makes

**Scaling.** Exact area averaging from the upload's side to 256 × 256,
weighted by alpha, in safe Rust of our own (about 50 lines). A 512 → 256
scale took 0.9 ms. A fully transparent pixel's colour becomes 0, so nothing
hides under transparency.

**The PNG master and fallback.**
- 256 × 256, 8-bit; RGB when opaque, RGBA otherwise.
- Only the chunks IHDR, IDAT and IEND.
- Written with `Compression::High`.
- It is deterministic: the same pixels always give the same bytes.
- **Its SHA-256 identifies the picture.** Reports and re-upload checks then
  survive a change of encoder or setting.

**The AVIF.**
- `ravif` at quality 80 and speed 8, 8-bit, with alpha as an auxiliary
  item.
- Measured output boxes: `ftyp`, `meta` (`hdlr`, `pitm`, `iloc`, `iinf` with
  `av01` items, `iprp` with `ispe`, `av1C`, `pixi`, and `auxC` for alpha),
  `mdat`.
- No Exif, XMP or ICC item, and no `colr` box.
- The speed and quality are constants with their reasons beside them; §4.2
  has the table they were chosen from.

**Colour.** Pixels are taken as sRGB, and Central does no colour
management.

**Sizes for six sample photos.**

| | AVIF at speed 8 | PNG master | together |
| --- | --- | --- | --- |
| photos | 6–16 KB | 100–143 KB | about 120–160 KB per picture |
| worst case (noise) | 87 KB | about 263 KB | |

10,000 pictures take about 1.4 GB.

### 3.4 Where it is stored

**In PostgreSQL**, like everything else Central keeps: one row per account.
- The AVIF, at most 160 KiB.
- The PNG master, at most 320 KiB, with `STORAGE EXTERNAL` on both columns.
- The master's SHA-256, the side, and when it was set.
- **A key of 16 random bytes** that names it in URLs. It comes from
  independent randomness, not a UUIDv7, and is new at every upload.

Nothing of the upload itself is kept.

### 3.5 Who sees it, and how it is served

**"Alle"** means:
- every signed-in account whose Central terms are accepted;
- administrators.

Recommended against (Q4):
- **signed-out visitors**, because no page shows a signed-out visitor a
  person, and public URLs would let anyone collect pictures without an
  account;
- **gateways**, because a gateway is a stranger's server and would learn
  the picture of everyone who signs in there; that is in-game avatars,
  later work in Baylee.

**Privacy.** A display name and tag are public within Central already, but
a picture is more personal: it may show a face. Registration is open, so
"every signed-in account" is close to "anyone who registers". The page says
so beside the upload button, and the privacy notice says so too (§7.9).

**Enumeration.**
- The URL names a random key, never the account id or the tag.
- Tags are dense (from 0x1000 up), so a URL built from the tag could be
  crawled in an afternoon.
- A key is learned only from answers that name an account already: one's
  contacts, groups and chats, a lookup by exact tag, and the
  administration.
- Every tag outside the caller's contacts and groups counts against the
  existing lookup budget (120 an hour per account). Collecting pictures is
  then exactly as hard as collecting names is today.

**The routes:** `GET /api/v1/pictures/{key}.avif` and
`GET /api/v1/pictures/{key}.png` on the browser listener. They check:
- `Host` exact;
- `Sec-Fetch-Site` `same-origin` when it is sent;
- no `Authorization` header;
- the `__Host-central` cookie of a live session with the terms accepted.

Every refusal is the same `404`, so a key's existence is never confirmed.
Their own budget is 16 requests in flight and 600 a minute per address. The
page draws:

```html
<picture>
  <source type="image/avif" srcset="/api/v1/pictures/{key}.avif">
  <img src="/api/v1/pictures/{key}.png" alt="" loading="lazy">
</picture>
```

So the browser picks the format and no `Vary: Accept` is needed.

**Answer headers:**
- `Content-Type` (`image/avif` or `image/png`) and `Content-Length`;
- `X-Content-Type-Options: nosniff`;
- `Content-Security-Policy: default-src 'none'; sandbox`;
- `Cross-Origin-Resource-Policy: same-origin`;
- `Content-Disposition: inline`;
- `Cache-Control: private, max-age=3600`, and `ETag` set to the digest;
- `Referrer-Policy: no-referrer`.

No cookie is set and no CORS header is sent.

**Removal and deletion take effect at once.**
- The row goes, so both URLs answer `404` to everyone at once.
- A browser that already showed the picture may keep it for up to an hour.
  There is no shared cache and no CDN.
- A new upload makes a new key, so old URLs never show the new picture.

**Fallback reach.** caniuse data, updated 11.11.2026:
- AVIF is supported by 96.69 % of tracked usage, against 97.57 % for PNG
  with alpha, so about one viewer in a hundred gets the PNG;
- browsers without AVIF include Edge before 121 (January 2024), Safari
  before 16.4 on macOS and 16.0 on iOS, Chrome before 85 and Firefox
  before 93.

### 3.6 Account deletion and export

**Removal.** `RemovePicture` deletes the row, and both URLs answer `404`
at once.

**Account deletion.** `ON DELETE CASCADE` takes the picture, the reports
about it, its notice and its budget row. Reports the account filed lose
their reporter (`ON DELETE SET NULL`), so no row names the deleted account,
which is [account deletion](../account-deletion.md)'s test.

**Export.** `AccountExport` is unchanged, because answers refuse unknown
fields. The export page adds:
- the PNG master;
- the facts from `MyPicture`;
- the reports this account filed, from `MyPictureReports`.

Reports about one's own picture are not exported with their reporters
(GDPR Art. 15(4)). The person was told of every removal and its reason
(§7.6). Whether that suffices is the owner's legal question.

## 4. The output encoders and the scaler

### 4.1 AVIF encoders that exist in Rust now

| Candidate | Newest | Licence | Native code | Verdict |
| --- | --- | --- | --- | --- |
| `ravif` 0.13.0 with `rav1e` 0.8.1 | 19.01.2026 and 16.06.2025 | BSD-3-Clause and BSD-2-Clause | none with `asm` off; with it, 94 assembly files (nasm on x86-64, `cc` on aarch64) | **chosen** |
| `gamut-avif` 2.0.0 | 16.10.2026; releases since June 2026 | MIT OR Apache-2.0 | none; forbids unsafe, but `gamut-codec-abi` has 84 sites (an FFI seam for foreign backends) | 8 packages and no gate change, but 1.3–1.5 s of CPU per picture at its fastest setting and 5–12 s at its default, against `rav1e`'s 0.17–0.25 s. 11–27 KB at its "quality 80", about 1.7× `rav1e`'s bytes (the two quality scales are not calibrated against each other). Five months of releases, one maintainer |
| `zenavif` 0.1.6 and `zenravif` 0.1.3 | April and May 2026 | **AGPL-3.0-only** or commercial | — | out: an AGPL crate cannot enter source-available crates (LICENSE.md §6) |
| `libavif` 0.14.0, `avif-rs`, `modernimage` | — | — | C (libavif with libaom, SVT-AV1 or dav1d) | out: native code |
| `image`'s `avif` feature | — | — | — | the same `ravif`, wrapped in `image` and its other packages |

`gamut-avif` was measured the same way: the six photos, quality 80, on
this Mac. It is the safest-looking newcomer, but not yet "very fast and
stable". It is worth a look again in a year.

### 4.2 `ravif` and `rav1e`, measured

**Conditions.** A probe outside the repository, `default-features =
false`, so `asm` and `threading` are off unless stated. Six 512² photos,
scaled to 256², quality 80. Times are CPU (user) seconds: the machine was
under a load average of about 20 while measuring, which inflates wall times.
The encoder ran on one thread: `user` ≤ `real` throughout, and a sampled
thread count never exceeded one (§6.2).

| Speed | Output bytes (6 photos) | CPU per picture | Noise with alpha: bytes, CPU, peak memory |
| ---: | --- | --- | --- |
| 10 | 8,109–18,205 | 0.05–0.06 s | 89,804 B, 0.10 s, 11.7 MB |
| 8 | 6,165–16,377 | 0.17–0.25 s | 86,741 B, 0.36–0.39 s, 14–15 MB |
| 6 | 5,829–16,052 | 0.22–0.35 s | 86,741 B, 0.40 s, 13.3 MB |
| 4 | 5,215–15,459 | about 0.5–0.8 s (wall) | not run |

**Other inputs** at speed 8:
- fully transparent: 707 B;
- white: 421 B;
- a 1 px checkerboard: 409 B.

**The output is valid.** macOS's ImageIO decoded the files, and so did the
browser in §2.1.

**`rav1e`'s assembly** (`asm`, aarch64) halves the wall time: 20–26 ms
instead of 47–57 ms at speed 10. The files were byte-identical (`cmp`, at
speeds 10 and 8). On x86-64 it needs nasm, which is not installed here;
x86-64 was not measured at all.

**Build cost** (release, this Mac):
- the probe with `png`, `ravif`, `jpeg-encoder` and `image-webp` built
  from clean in 28.7 s;
- rebuilding `ravif` and `rav1e` alone took 15.4 s;
- the binary grew from 0.69 MB (`png` only) to 2.81 MB, and to 3.13 MB with
  `asm`.

**Choice: speed 8.** It costs a quarter of a second of CPU and gives files
10–24 % smaller than speed 10 (about 13 % on average). Speed 6 gains 2–6 %
for about a third more time.

### 4.3 The fallback

| Format | Bytes (6 photos, 256 px) | Encode | Extra package | Transparency |
| --- | --- | --- | --- | --- |
| **PNG 256** (`png`) | 100,593–143,234 | 5–25 ms with the newest `flate2` | none | yes |
| PNG 128 | 27,122–36,565 | — | none | yes |
| JPEG 256, q85 (`jpeg-encoder` 0.7.1) | 13,323–23,420 | under 1 ms | 1, with an IJG licence exception | no; flattened |
| WebP lossless (`image-webp` 0.2.4) | 89,010–135,830 | under 1 ms | 3 | yes, but WebP support (97.24 %) is no wider than AVIF's |

**Recommended: PNG 256.**
- It is the lossless master anyway: a later encoder or size can be made
  from it without asking anyone to upload again.
- It needs no package and no licence exception, and keeps transparency.
- Its bytes go to about one viewer in a hundred.

**The JPEG option** (Q3), `jpeg-encoder` 0.7.1:
- no dependencies and no build script;
- forbids unsafe without its `simd` feature (6 sites, only with `simd`);
- packaged sources equal upstream at `baf40191ae25` (13 files);
- its licence is `(MIT OR Apache-2.0) AND IJG`.

### 4.4 The scaler

| Candidate | Unsafe in source | Verdict |
| --- | --- | --- |
| **Our own area average** | 0 | chosen: exact for any ratio, about 50 lines, 0.9 ms for 512 → 256 |
| `resize` 0.8.9 | 1 site, plus `rgb` (77) | no gain |
| `fast_image_resize` 6.1.0 | 695 sites in 114 files (SIMD) | not worth it: the work is one 1024² → 256² scale per upload |
| `pic-scale` 0.7.12 | 1,348 sites in 128 files | not worth it |

## 5. The packages, at their newest releases

### 5.1 What enters

**The placeholder's manifest:**

```toml
png = "=0.18.1"
ravif = { version = "=0.13.0", default-features = false }   # no asm, no threading
```

**What resolves against the lock at `5529fe4`:**
- **46 new packages.** 42 are compiled on every target checked:
  x86_64 and aarch64 Linux, both macOS targets, and Windows MSVC.
- **Four packages sit in the lock only**: `arbitrary`, `interpolate_name`
  and `libfuzzer-sys` (all under `rav1e`'s `cfg(fuzzing)`), and `zlib-rs`
  (`flate2`'s weak dependency).
- **Approved packages that gain features:**
  - `arrayvec` 0.7.8: `default`;
  - `crossbeam-utils` 0.8.23: `default`;
  - `num-traits` 0.2.19: `default`;
  - `syn` 2.0.119: `fold`;
  - `rand` 0.9.5: `default`, `os_rng`, `small_rng`, `std_rng`,
    `thread_rng`;
  - `rand_chacha` 0.9.0: `default`;
  - `rand_core` 0.9.5: `os_rng`.

  The three `rand` packages are `cfg(fuzzing)` only and never built.
- **`rav1e`'s `wasm` and `wasm-bindgen` features** appear because `ravif`
  asks for them on `wasm32-unknown-unknown` only.

**The lock** gains exactly these 46 entries, plus the new workspace member.
`rand` 0.9.5, `rand_core` 0.9.5, `getrandom` 0.3.4 and `r-efi` 5.3.0 were
in it already; `cargo deny` counts them as duplicates once `rav1e` names
them, hence the skips of §5.4.

**The full table**, with licence, compiled or not, build script, unsafe
sites in source and native files, is in Appendix A.

**Unsafe in the compiled packages:**
- `rav1e`: 319 sites in source, of which about 78 are compiled without
  `asm`. 173 belong to the assembly wrappers, 47 to the C API, and 10 to
  tests.
- About 1,100 in source across all compiled new packages. At most about
  870 of them are compiled: that subtracts only `rav1e`'s uncompiled
  sites, while per-architecture SIMD paths and test code are still counted.
  Most are in:
  - `crossbeam-epoch` (192), `rayon-core` (87) and `rayon` (67), compiled
    although `threading` is off;
  - `rgb` (77), `aligned-vec` (56), `av-scenechange` (48), `equator` (46)
    and `flate2` (37);
  - the checksum crates.
- `png`, `fdeflate`, `miniz_oxide`, `adler2` and `bitstream-io` forbid
  unsafe.

**Build scripts.**
- `rav1e`'s runs `built::write_built_file()` (build facts, without git,
  since the `git2` feature is off) and compiles assembly only with `asm`.
- `av-scenechange`'s and `rayon-core`'s are similar.
- `crc32fast`'s runs `$RUSTC --version`.
- No package compiles C or assembly with `asm` off.

### 5.2 Newest versions, and the one exception worth discussing

- `cargo update --dry-run --verbose` in the scratch copy would change none
  of the new packages, so each is at its newest compatible release.
- **`flate2` 1.1.11 and `crc32fast` 1.5.5 are now recommended**, where the
  first version pinned 1.1.9 and 1.5.0. The owner asked for newest
  releases, and the encoder's gate changes are needed anyway, so the older
  `flate2` no longer avoids an exception.
- `crc32fast` 1.5.5 still has a concrete counter-reason, which I report
  rather than act on:
  - five releases since August 2026 tripled its SIMD code (6 → 23 sites);
  - 1.5.5 is two days old;
  - the new code runs only inside the sandboxed process (§6.2).

  Q2 offers the older pair.

### 5.3 Advisories, maintenance, provenance

**Advisories.** RustSec (database of 12.11.2026):
- nothing for `rav1e`, `ravif`, `av1-grain`, `v_frame`, `av-scenechange`,
  `avif-serialize`, `aligned-vec`, `equator`, `imgref`, `loop9`,
  `bitstream-io`, `rayon`, `rayon-core`, `nom`, `built`, `pastey`,
  `profiling` or `y4m`;
- old advisories are fixed in the resolved versions:
  - `rgb` RUSTSEC-2020-0029 (before 0.5.4);
  - `crossbeam-deque` RUSTSEC-2021-0093;
  - `crossbeam-epoch` RUSTSEC-2026-0204 (fixed in 0.9.20; we resolve
    0.9.21).

`cargo audit --deny warnings` exits 0 with the repository's
`.cargo/audit.toml`.

**`paste` 1.0.15** (RUSTSEC-2024-0436, unmaintained) comes in through
`rav1e` 0.8.1.
- The existing ignore was accepted for Leptos only ("remove when Leptos
  drops it").
- `rav1e` has moved to `pastey` upstream, but has not released it.

**Maintenance:**

| Package | Releases | Repository activity | Owners on crates.io |
| --- | --- | --- | --- |
| `rav1e` | 0.8.1, 16.06.2025 | xiph/rav1e, last commit 31.08.2026 | lu-zero alone |
| `ravif` | 0.13.0, 19.01.2026 | cavif-rs, last commit 05.09.2026 | kornelski alone, who also owns `avif-serialize`, `imgref` and `loop9` |

- **Unreleased in `rav1e`:** `rand` 0.10 (which would end the `rand`
  duplicates), MSRV 1.85, `pastey`, assembly for scene detection, lint
  fixes.
- **Unreleased in `ravif`:** a commit titled "Moxcms" (a future release may
  add the colour-management crate), dependency bumps, an EXIF setter.

Read from the repositories' commit feeds.

**Fuzzing.** OSS-Fuzz has no project for `rav1e` or `ravif`. It has one for
dav1d, which only design B would use. `rav1e` keeps fuzz targets of its own
(its `cfg(fuzzing)` dependencies).

**Provenance.** Every packaged source file, build script and
`Cargo.toml.orig` was compared with upstream at the commit
`.cargo_vcs_info.json` names:

| Package | Files equal |
| --- | --- |
| `rav1e` 0.8.1 (`38806721f718`) | 121 |
| `ravif` 0.13.0 (`760314e5100e`) | 5 |
| `avif-serialize` 0.8.9 | 5 |
| `av1-grain` 0.2.5 | 11 |
| `v_frame` 0.3.9 | 6 |
| `av-scenechange` 0.14.1 | 34 |
| `crc32fast` 1.5.5 | 9 |
| `jpeg-encoder` 0.7.1 | 13 |
| `png` 0.18.1 (`2a3f980245e3`) | 22 |
| `fdeflate` 0.3.7 (`c365c7e6ffa8`, tag `v0.3.7`) | 7 |
| `flate2` 1.1.11 (`cd06529164c5`) | 22 |
| `miniz_oxide` 0.8.9 (`44e43c7786e3`) | 15 |
| `miniz_oxide` 0.9.1 (`4e582392df3a`) | 15 |
| `adler2` 2.0.1 (`89a031a0f42e`) | 3 |
| `simd-adler32` 0.3.10 (`d93164f98d7f`) | 11 |

The other, smaller helpers were not compared.

### 5.4 What the gate needs, exactly

Tested in the scratch copy: with these lines, `cargo deny check` reports
`advisories ok, bans ok, licenses ok, sources ok`.

```toml
# deny.toml [bans] skip, beside the existing entries
{ name = "miniz_oxide", version = "=0.8.9", reason = "png 0.18.1 declares it and calls nothing of it; flate2 1.1.11 uses 0.9" },
{ name = "nom", version = "=7.1.3", reason = "bindgen's cexpr (push) vs av1-grain's nom 8 (rav1e)" },
{ name = "rand", version = "=0.9.5", reason = "rav1e's cfg(fuzzing) dependency; in the lock, built by no target" },
{ name = "rand_core", version = "=0.9.5", reason = "rav1e's cfg(fuzzing) dependency; in the lock, built by no target" },
{ name = "getrandom", version = "=0.3.4", reason = "rand_core 0.9 under rav1e's cfg(fuzzing); built by no target" },
{ name = "r-efi", version = "=5.3.0", reason = "getrandom 0.3 under rav1e's cfg(fuzzing); built by no target" },

# deny.toml, beside the existing licence exceptions
[[licenses.exceptions]]
name = "rav1e"
version = "=0.8.1"
allow = ["BSD-2-Clause"]

[[licenses.exceptions]]
name = "av1-grain"
version = "=0.2.5"
allow = ["BSD-2-Clause"]

[[licenses.exceptions]]
name = "v_frame"
version = "=0.3.9"
allow = ["BSD-2-Clause"]

[[licenses.exceptions]]    # rav1e's cfg(fuzzing) dependency; never built
name = "libfuzzer-sys"
version = "=0.4.13"
allow = ["NCSA"]
```

**The `paste` ignore** in `deny.toml` and `.cargo/audit.toml` keeps its id.
Its reason becomes: "compile-time proc macro of Leptos and of `rav1e` 0.8.1;
remove when both drop it".

**`approved-packages.json`** gains the 46 packages with the features of
Appendix A, and the feature additions of §5.1. With exactly these,
`cargo xtask policy` passed in the scratch copy.

Each line is a gate exception that only the owner can accept (Q2).
- **BSD-2-Clause** is an OSI-approved permissive licence; per-crate
  exceptions keep it off the global allow list.
- **NCSA** belongs to LLVM's libFuzzer C++ sources, which no build here
  compiles.

## 6. Recommendation

### 6.1 Packages

**Design A, with:**
- `png` 0.18.1;
- `ravif` 0.13.0 with `default-features = false` (no `asm`, no
  `threading`);
- every transitive package at its newest release, as resolved;
- the six web-sys features: `CanvasRenderingContext2d`, `File`,
  `FileList`, `ImageBitmap`, `ImageBitmapOptions`, `ImageOrientation`;
- `blob:` in `img-src` of the site's CSP, on the profile page only, used
  only for SVG.

**Not used:** `image`, `fast_image_resize`, `jpeg-encoder`, `image-webp`.

### 6.2 Containment: a fresh, sandboxed process per upload

**Why it is now needed.** The encoder is 68,000 lines with about 78
compiled unsafe sites, plus helpers with several hundred more, all running
on pixel values the uploader chose. The service holds the database
password: systemd hands it the configuration on standard input. Running
the conversion inside the service would put that password within reach of
any memory-corruption bug. A process of its own, holding nothing, does not.
It needs **no new crate**.

**The worker** is a new binary, `central-picture-worker` (crate
`central-picture`, source-available, `#![forbid(unsafe_code)]`).
- **Protocol:** one length-prefixed request on standard input, the PNG of
  at most 2 MiB. One answer on standard output: the AVIF and the PNG
  master, or a closed `Refusal` code. Then it exits.
- **Threads:** the worker needs one. `rav1e` takes `rayon` as `maybe-rayon`
  without threads, so it runs on the calling thread. `av-scenechange` links
  the real `rayon` but did not start it for a still picture: sampled every
  50 ms through 18 encodes at speed 4, the probe never had more than one
  thread.
- **It holds:** no configuration, no network code, no database code.

**In production on Linux**, through systemd socket activation (designed,
not run):
- `central-picture.socket`:
  - `ListenStream=/run/baylee-central/picture.sock`;
  - `Accept=yes`, `MaxConnections=2`;
  - `SocketUser=central`, `SocketMode=0600`.
- `central-picture@.service`:
  - `StandardInput=socket`, `StandardOutput=socket`;
  - `DynamicUser=yes`;
  - `PrivateNetwork=yes`, `RestrictAddressFamilies=none`, `IPAddressDeny=any`;
  - `ProtectSystem=strict`, `ProtectHome=yes`, `PrivateTmp=yes`,
    `PrivateDevices=yes`;
  - `NoNewPrivileges=yes`, `CapabilityBoundingSet=`;
  - `SystemCallFilter=@system-service`, `MemoryDenyWriteExecute=yes`;
  - `InaccessiblePaths=/etc/baylee-central`;
  - `LimitCORE=0`;
  - `MemoryMax`, `TasksMax` and `RuntimeMaxSec`, starting from 64 MiB, 4
    and 10 s. These are to be set from measurement on the server: the probe
    peaked at 15 MB, one thread and 0.4 s of CPU on Apple Silicon, and
    x86-64 was not measured.

  Each upload gets a fresh process with no user of its own and nothing to
  read, which systemd ends when it runs too long or too large.
- The service connects with `std::os::unix::net::UnixStream` with read and
  write timeouts. Its unit already allows `AF_UNIX`.

**In development, tests and CI** on macOS, Windows and Linux, the service
starts the same binary as a child: `std::process::Command`, environment
cleared, standard input and output piped, killed after 10 s. Same protocol,
same refusals.

**What this gives:**
- a memory-corruption bug in the encoder, the checksum crates or the
  decoder reaches no secret;
- a hang or a runaway allocation ends with the process;
- the service checks the answer's framing and bounds (sizes, the PNG
  master's signature and IHDR, the AVIF's top-level boxes), **computes the
  master's SHA-256 itself**, and stores the answer. A worker therefore
  cannot choose the digest that reports and blocks key on.

**What it cannot stop.** A worker that an exploit took over still chooses
the bytes Central serves. The checks above validate structure, not content,
so such bytes would reach every signed-in browser. They reach it as
`image/avif` or `image/png` with `nosniff`, a sandbox CSP and CORP, where
only the browser's own image decoders read them. In-process conversion has
the same exposure, and the database password besides (Q5).

**What it costs:** a unit pair and its operations notes, the protocol and
its tests, and about a millisecond to start a process per upload.

**What it does not do:** on Windows and macOS it is only a process
boundary, without the systemd sandbox. That is acceptable, since production
is Linux.

### 6.3 What the owner accepts with this

- the packages of §5.1;
- the gate changes of §5.4;
- the web-sys features and the CSP change of §6.1;
- the worker units of §6.2.

## 7. Design for the build

### 7.1 Crates

**`central-picture`** (library and the `central-picture-worker` binary,
source-available) holds:
- `reencode(png: &[u8]) -> Result<Picture, Refusal>`, which runs §3.2's
  checks, the decode, our area average, the PNG master and the AVIF
  (`ravif`); the worker binary calls it;
- the protocol;
- the answer checks the service uses, and the master's SHA-256 (`sha2`,
  already approved), computed in the service, never taken from the
  worker.

**Callers.**
- `central-identity` calls the worker.
- `central-postgres` stores the result.
- `central-http` serves the routes.

**Licences.** The AGPL crates (`central-api` and the others) gain types
only and never build on `central-picture`.

### 7.2 Wire: new variants only

Every answer refuses unknown fields, so no existing request or answer gains
a field. Old gateways and clients never send the new requests and so never
receive the new answers.

**For the account:**
- `Request::MyPicture` → `Response::MyPicture`, which holds:
  - `picture: Option<Picture { key: [u8; 16], side: u16, avif_bytes: u32,
    png_bytes: u32, sha256: [u8; 32], set_at: i64 }>`, where `sha256` is
    the PNG master's;
  - `notice: Option<PictureNotice { at, reason, note }>`;
  - `blocked_until: Option<i64>`.
- `Request::RemovePicture` → `Done`.
- `Request::MyPictureReports` → `Response::MyPictureReports { reports }`:
  the reports this account filed, with reason, time and state, for the
  export.

**Social:**
- `SocialRequest::Pictures { tags }` (at most 300 tags) →
  `SocialResponse::Pictures { pictures: Vec<{ tag, key }> }`.
  - Every tag outside the caller's contacts and groups spends one unit of
    the existing lookup budget (§3.5).
  - Unknown tags, accounts without a picture and blocked pairs are simply
    absent.
- `SocialRequest::ReportPicture { tag, key, reason, note: Option<String> }`
  → `Done`.

**Administration:**
- `AdminRequest::PictureReports { state, before }`: the queue, 50 a page;
- `AdminRequest::PictureCase { account }`: the current picture, its open
  and decided reports, the notice and the block;
- `AdminRequest::DecidePicture { account, sha256, decision: Keep |
  Remove { reason, note }, block: None | Days(u16) | UntilLifted }`;
- `AdminRequest::LiftPictureBlock { account }`;
- with matching new `AdminAnswer` variants.

**On which listener:** the browser listener only. The native listener
refuses these, as it does `SetDisplayName`, until M6 (§7.8).

### 7.3 Routes and budgets

**`PUT /api/v1/picture`**, the body a raw PNG. The browser listener's
64 KiB body bound does not fit, so this is a route of its own, like the
report parts. The checks run in this order:

1. **Headers, before any byte is read:**
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
   - no upload block;
   - its budget: 10 uploads an hour and 30 a day, in one durable row per
     account (`picture_budgets`);
   - an upload slot: two at a time in all, one per account; otherwise
     `429`.
3. **The body**, read to `Content-Length` within 20 seconds, else `408`.
   From here the upload counts against the budget, whether it is stored or
   refused.
4. **The worker** (§6.2), at most two at a time, as the socket's
   `MaxConnections` says; otherwise `429` after two seconds.
   - **A refusal answers `422`**, with the `Refusal` code in a small JSON
     body, so the page can say which check failed. It must not be
     `401 Rejected`: the account island reads that as an ended session and
     signs the person out.
   - **The service checks the worker's answer** before storing it (§6.2):
     - the framing and the sizes;
     - the PNG master's signature and IHDR (256 × 256, 8-bit, RGB or RGBA,
       not interlaced);
     - the AVIF's top-level boxes.

     The service computes the master's SHA-256 itself.
5. **One transaction:**
   - lock the account row;
   - check the session and the block again;
   - replace the row with a new key;
   - mark open reports about the old digest `gone`.

The answer is the JSON of `Response::MyPicture`.

**The `GET` routes** are as §3.5 describes. Other methods on these paths
answer `405`. Nothing of a body and no key is ever logged.

### 7.4 Migration 000027

000027 is free: no branch has a migration file with that number, and
[postgresql.md](../postgresql.md) says it "was kept … and is not used".
000024 stays reserved for the session-token line of work. If another line
lands first, this takes the number after it.

```sql
CREATE TABLE central_identity.pictures (
    account_id uuid PRIMARY KEY REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    key bytea NOT NULL UNIQUE CHECK (octet_length(key) = 16),         -- independent randomness
    avif bytea NOT NULL CHECK (octet_length(avif) BETWEEN 32 AND 163840),
    png bytea NOT NULL CHECK (octet_length(png) BETWEEN 67 AND 327680),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),          -- of the PNG master
    side smallint NOT NULL CHECK (side = 256),
    set_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);
ALTER TABLE central_identity.pictures ALTER COLUMN avif SET STORAGE EXTERNAL;
ALTER TABLE central_identity.pictures ALTER COLUMN png SET STORAGE EXTERNAL;

CREATE TABLE central_identity.picture_reports (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) = 7),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),          -- the PNG master reported
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
| `pictures` | `SELECT, INSERT, UPDATE (key, avif, png, sha256, side, set_at), DELETE` |
| `picture_reports` | `SELECT, INSERT, UPDATE (state, decided_at, decided_by)`, no `DELETE`: the moderation record is append-only for the runtime, as the accounts-admin design's step 8 has it |
| `picture_notices` | `SELECT, INSERT, UPDATE, DELETE` |
| `picture_budgets` | `SELECT, INSERT, UPDATE` |

**The sweep.** The operator's `--sweep-stdin` deletes decided reports past
their retention (Q8). The runtime cannot.

**Earlier binaries.** The previous binary names none of these tables, so it
runs unchanged over the migration.

### 7.5 Store and service

**Visibility** is one SQL predicate, used both by the `GET` routes and by
`SocialRequest::Pictures`. A picture is visible to:
- a signed-in account whose Central terms are accepted;
- an administrator;

unless a block exists in either direction between the viewer and the
owner. A block hides the picture between those two accounts only.

It is tested as a matrix (§7.10).

**Errors.** Every answer keeps the existing vocabulary: `Rejected`,
`Conflict`, `Busy`, `Unavailable`, `TermsRequired`. The upload route adds
its `422` refusals.

### 7.6 Reports and moderation

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

**Filing.** Since every signed-in account sees pictures, every signed-in
account can report one.
- "Report picture" sits in the menu (⋮) of a contact row, a conversation's
  head and a group member, and beside a picture shown after a lookup.
- The report holds a reason from the closed list and an optional note.
- The server records the picture's **digest** (the PNG master's), so a
  decision applies to exactly the picture that was reported, whatever the
  encoder settings.
- Refusals:
  - one's own picture: `Rejected`;
  - a picture one cannot see, or a key that is not the account's current
    one: the same `Rejected`;
  - a second open report on the same picture: `Conflict`;
  - more than 20 reports an hour, or 100 open, from one reporter: `Busy`.
- The reported person is not told of a report, only of a decision.

**Deciding (administrators, `/admin/reports`).** The queue shows the
picture through the same `GET` routes (administrators may see every
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

Q6 to Q8 settle what a report holds, what removal does and how long
decided reports are kept.

### 7.7 Website

**The profile page** (`islands/account.rs`, a new `islands/picture.rs`)
gains a "Profile picture" card:
- the picture at 128 CSS pixels, or the initials avatar;
- "Choose a picture…": a file input accepting `image/*`, then the cropper
  of §2.1, working on the decoded bitmap, with a live preview in a
  `<canvas>`;
- "Save", which sends the `PUT` with `fetch`;
- "Remove picture", behind a confirmation;
- the notice and the block of §7.6, when there are any;
- a line: "Every signed-in player can see your picture. Your browser sends
  a square copy without the photo's details (place, camera, time)."

**Errors** are named: too large, cannot be opened in this browser, each
`422` refusal, `Busy` ("try again in a moment" or "later today"), blocked
until a date.

**Behaviour:**
- keyboard: arrows move the square, Shift takes larger steps, a range
  input zooms;
- a live region announces each step;
- no hover that moves or resizes anything (owner, 11.11.2026).

**Where pictures are drawn.** `islands/chat/talk.rs`'s `avatar()` draws
every avatar today: the signed-in person on the chat's side, contact rows,
conversation heads and group members. It gains an optional key and draws
§3.5's `<picture>` inside the same circle. It falls back to the initials
when there is no picture or the image fails. The chat fetches keys with
`SocialRequest::Pictures` beside its contact snapshot.

Also:
- a lookup by exact tag shows the picture;
- the administration's account page and the report queue show it;
- the export section offers the PNG master and the report list (§3.6).

**CSP.** The `Content-Security-Policy` header gains `blob:` in `img-src`.
A CSP cannot limit `blob:` to SVG.
- The site sends it on the profile page only.
- The page uses it only to decode an SVG the person chose: a `blob:`
  address made from that file, revoked once it is drawn.
- No inline script, no `data:`.

**What is added:**
- web-sys features `CanvasRenderingContext2d`, `File`, `FileList`,
  `ImageBitmap`, `ImageBitmapOptions` and `ImageOrientation`, to be
  approved with the packages;
- small header readers in the islands' WebAssembly for PNG, JPEG, GIF, WebP
  and BMP dimensions (§3.1);
- texts in every language the site has.

### 7.8 Later, not in this step

**The native client (M6).** The native wire carries 1 MiB and writes bytes
as JSON numbers, so the client needs a raw route on the native listener:
`PUT /v1/picture`, with the bearer and the same checks. The client converts
the person's file to the PNG with its own `image` 0.25.10, which Baylee
already links. That is Baylee's graph, not Central's.

**Gateways.** Pictures for gateways would be a new request,
`GatewayPicture { lease, purpose }`, under `GatewayProfile`'s lease check.
Today a gateway learns the name and tag of whoever signs in there; it would
learn their picture too, and a gateway is a stranger's server. Q4 asks
whether that is wanted.

### 7.9 Privacy notice and threat model (drafts)

The privacy notice, in English; the binding German is the owner's:

> **§3** – **Profile picture (optional):** a square picture you choose.
> Your browser makes a copy of it without the photo's details (such as
> place, camera or time); we turn that copy into a 256 × 256 pixel AVIF
> and PNG picture that holds nothing else, and keep it with its fingerprint
> and the time you set it – to show your account. Every signed-in user of
> Baylee Central and our administrators can see it. *Art. 6(1)(b) GDPR*
>
> **§3** (added to "Reports about content or accounts, moderation
> decisions") – A report about a profile picture holds the reporting
> account, the picture's fingerprint, the reason and the note written; the
> reported person is not told who reported it.
>
> **§6** – Other users: everyone signed in to Baylee Central can see your
> profile picture.
>
> **§7** – A profile picture is kept until you replace or remove it, an
> administrator removes it, or you delete your account; it is then deleted
> at once.

The privacy policy needs a new version before this goes live. Its words
"(a profile picture will follow)" go.

A new row for the [threat model](../threat-model.md):

| Threat | Required control | Evidence before release |
| --- | --- | --- |
| A profile picture: a hostile file, a decompression bomb, a polyglot, hidden metadata, a bug in the decoder or the AVIF encoder, offensive content | The browser converts to a square PNG; Central takes only that, up to 2 MiB and 256–1024 px a side, with its header checked by our code before decoding. Decoding (`png`, which forbids unsafe) and encoding (`rav1e` without assembly) run in a fresh process per upload with no secrets, no network, and bounded memory and time. The service checks the answer's framing and headers and computes the digest itself. The output is made from pixels only. Pictures are served same-origin under random keys with `nosniff`, a sandbox CSP and CORP, to signed-in accounts only. Reports, and administrators' removal with a reason | The hostile-file tests and mutation tests of §7.10, the worker's crash, hang and oversized-answer tests, the unit's sandbox check, the visibility matrix, the routes' headers |

What remains, stated for Q5: a worker that an exploit took over still
chooses the bytes Central serves to every signed-in browser. The service
checks framing and headers, not the content. Those bytes reach browsers as
`image/avif` or `image/png` with `nosniff`, where only the browser's own
image decoders read them. The in-process alternative has the same exposure,
and the database password besides.

### 7.10 Tests, with hostile files on every input path

**The page** (unit tests; Playwright on desktop and Pixel 7, and on WebKit
and Firefox where the test setup allows):
- a JPEG with EXIF orientation;
- CMYK and progressive JPEG;
- 16-bit and palette PNGs with `tRNS`;
- animated GIF, WebP, AVIF and APNG, giving their first frame;
- SVG with a script, an external `href`, a huge `viewBox`, and an entity
  bomb;
- a 30,000² PNG and JPEG in a small file, refused from the header;
- HTML renamed to `.png`;
- an empty file, and one of 26 MiB;
- HEIC on WebKit;
- the hardened-browser text;
- no CSP violation;
- the stored AVIF and PNG decode in the browser, at 256 × 256. Central
  links no AVIF decoder, so this is where the AVIF is decoded again.

**`central-picture`, the PNG path:**
- *Accepted:*
  - every colour type and bit depth: grey 1, 2, 4, 8 and 16; grey with
    alpha 8 and 16; RGB 8 and 16; RGBA 8 and 16; palette 1, 2, 4 and 8,
    with and without `tRNS`;
  - interlaced input;
  - sides 256, 257, 511, 512 and 1024.
- *Refused before decoding:* not a PNG; IHDR not first or not 13 bytes
  long; a side of 0, 255 or 1025; not square; an invalid depth and colour
  pair; compression or filter other than 0; interlace above 1; `acTL`.
- *Refused while decoding:*
  - a CRC error in any chunk;
  - every prefix of a valid file;
  - a bad zlib header or Adler-32;
  - too little image data;
  - a palette image without PLTE, or a `tRNS` of the wrong length.
- *Bounded, each within a time bound:* the IDAT and zTXt bombs of §3.2;
  `iCCP` bombs; `eXIf`, script text and bytes after IEND absent from the
  output.

**`central-picture`, the outputs:**
- the PNG master: 256 × 256, decodes again to the same pixels (the round
  trip), holds only IHDR, IDAT and IEND, is RGB when opaque and RGBA
  otherwise, has colour 0 under alpha 0, is the same bytes twice, and gives
  itself back when processed again;
- the scaler averages exactly: a checkerboard becomes even grey, and halves
  stay halves;
- the AVIF, for noise, transparent, white, checkerboards and extreme alpha
  at 256²: within its column bound, with the boxes of §3.3 and no Exif,
  XMP or ICC.

**Mutation testing without a new dependency:**
- seeded mutations of valid and hostile PNGs in `cargo test`: fixed seeds
  and an xorshift in the test code; byte flips, insertions, deletions and
  chunk-length edits, with CRCs corrected so the mutations reach the
  decoder;
- each case ends in a refusal or a valid answer, never a panic, within a
  time bound;
- `cargo xtask picture-fuzz --seconds N` runs the same loop for longer.

Coverage-guided fuzzing would need `cargo-fuzz` and libFuzzer, which is a
tool decision. The worker reads standard input, so an external fuzzer (such
as AFL++ in its binary-only mode) can drive the release binary in a
separate environment without changing the build.

**The worker and its protocol:**
- a length prefix that lies, a truncated frame, an oversized frame;
- a worker that crashes, hangs, or answers too much or malformed (a test
  stand-in), each giving a refusal and a killed process;
- the environment cleared;
- the service's own digest, and its answer checks;
- on Linux CI, `systemd-analyze security` on the unit, as a check that it
  says what §6.2 says.

**`central-http`:**
- every header refusal happens before the body is read and before any slot
  is taken;
- `Content-Length` missing or too large; a slow body (`408`); a body of
  the wrong length; another content type;
- the `422` refusals;
- `GET` without a cookie, blocked, and with an unknown key: all the same
  `404`;
- the answer's headers byte for byte;
- `HEAD` and other methods `405`;
- the `GET` budget, apart from `POST /api/v1`'s places.

**`central-postgres` fixture:**
- set, replace and remove; a new key each time, and the old key `404`;
- the visibility matrix: self, any signed-in account, blocked either way,
  administrator, signed out;
- `Pictures`: the lookup budget spent per tag outside contacts and groups;
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

**Gates:** `cargo xtask gate`, `policy`, `postgres`, `smoke` and
`secrets`, `cargo deny check` and `cargo audit --deny warnings`.

### 7.11 Order


1. The owner's answers and acceptance.
2. The next design round: the exact routes, schema, grants, wire, worker
   protocol and units. Then the implementation and adversarial passes.
3. `central-picture` with the worker and its tests.
4. The migration, store, service, routes and administration.
5. The website.
6. The units and the operations notes.
7. The privacy policy's new version.
8. M6: the native route and the client's own conversion to PNG.

Then the docs: web accounts, administration, the threat model's
"Upload parser compromise" row, `postgresql.md`, `operations.md`,
`status.md` and the roadmap.

## 8. Questions for the owner

Each question lists the recommended option first. Q2 and Q3 apply only if
Q1 is answered (a).

**Q1. Where are the uploaded formats decoded?**
- (a) **In the browser.** The page uploads a lossless square PNG, and
  Central converts that to AVIF and decodes nothing else. Any format the
  browser shows works, SVG included; that adds `blob:` to `img-src` in the
  CSP of the profile page. *Recommended.*
- (b) On the server, from the original. About 98 packages, libdav1d (C) or
  `rav1d`'s assembly for AVIF, an SVG renderer, and more gate exceptions.

**Q2. Accept the package set and its gate changes?**
- (a) **The newest releases:** `png` 0.18.1 and `ravif` 0.13.0 (`rav1e`
  0.8.1, assembly and threading off). 46 packages, 42 built; the four
  licence exceptions, six duplicate skips, the `paste` exception widened to
  `rav1e`; the six web-sys features. *Recommended.*
- (b) The same with `rav1e`'s assembly on: about twice as fast, identical
  output. It adds 94 hand-written assembly files to the build, and nasm on
  x86-64 servers.
- (c) The same with `flate2` 1.1.9 and `crc32fast` 1.5.0. One skip fewer
  and smaller SIMD in `crc32fast`, but not the newest.
- (d) Not now.

**Q3. Which fallback for browsers without AVIF?**
- (a) **PNG, 256 px**, which is also the lossless master: about 100–145 KB,
  no new package, transparency kept. *Recommended.*
- (b) PNG, 128 px: about 27–37 KB, no new package, softer.
- (c) JPEG, 256 px, quality 85: about 13–23 KB. Needs `jpeg-encoder` 0.7.1
  with an IJG licence exception, and transparency is flattened.

**Q4. "Alle": beyond every signed-in account and administrators?**
- (a) **No: signed-in accounts and administrators only.** *Recommended.*
- (b) Also signed-out visitors, through public URLs.
- (c) Also the gateways a person signs in to, later, for in-game avatars
  (Baylee work).

**Q5. How is a picture decoded and encoded?**
- (a) **In a fresh process per upload with no secrets, no network and
  bounded memory and time** (systemd socket activation in production, a
  child process in development). *Recommended.*
- (b) Inside the service, on a blocking thread. Simpler, but a bug in the
  encoder or the checksum crates could reach the database password, and a
  hang holds the slot until the service restarts.

In both, a conversion an exploit took over chooses the bytes served to
signed-in browsers (§6.2).

**Q6. What does a report hold?**
- (a) **A reason from the list and an optional note of up to 1000
  characters. Administrators see the reporter; the reported person never
  does.** *Recommended.*
- (b) A reason only.
- (c) As (a), but the reporter is not stored.

**Q7. What can an administrator do, and what happens to a removed
picture?**
- (a) **Remove it with a reason the person is told, and optionally block
  uploads for some days or until lifted. The picture's bytes are deleted at
  once; only the digest stays with the reports.** *Recommended.*
- (b) Remove only; the bytes are deleted at once.
- (c) As (a), but the bytes are kept hidden for 90 days in case an
  authority asks. That needs a legal check.

**Q8. How long are decided picture reports kept?**
- (a) **One year.** *Recommended.*
- (b) Three years, the privacy notice's maximum.
- (c) 90 days.

## 9. Verified, assumed, not verified

**Verified, by commands, probes and one browser:**
- the graphs of design A and of B's decoders, format by format;
- the new packages, their features and which are built, on five targets;
- `cargo deny`, `cargo audit`, and `cargo xtask policy` with the proposed
  entries;
- `cargo update --dry-run`;
- marker counts and native files;
- provenance of the packages named in §5.3;
- the encoder's sizes and CPU times on Apple Silicon, its worst cases and
  peak memory;
- one thread: `user` ≤ `real`, and a thread count sampled every 50 ms
  through 18 encodes never above one;
- the lock's growth: exactly the 46 entries, plus the new member;
- assembly on and off giving identical bytes;
- the AVIF's boxes;
- the fallbacks' sizes, and the scalers' unsafe counts;
- `gamut-avif`'s sizes and times;
- caniuse figures (data of 11.11.2026);
- Chromium 156: EXIF orientation applied, canvas PNG as 8-bit RGBA and not
  interlaced, AVIF and animated WebP drawn as stills, SVG through `<img>`
  and `blob:` without running its script and without tainting the canvas;
- that no branch has migration 000027.

**Checked in only one browser.** Design A's intake was checked in Chromium
156 alone, nothing was measured on x86-64, and the worker's sandbox is
designed, not run. These come first among the open points.

**Assumed, to be checked in the end-to-end tests.** Design A rests on
these:
- Firefox's and WebKit's orientation handling and canvas PNG output;
- canvas colour management to sRGB;
- canvas read-back in hardened browsers (Firefox `resistFingerprinting`,
  Tor, Brave, Safari);
- Safari's HEIC;
- canvas taint for SVG in WebKit.

If one fails, the person sees a turned, dull, noisy or blank picture, or an
error. Central's safety does not depend on any of them.

**Not measured:**
- anything on x86-64: timings, the assembly path, nasm;
- the worker's systemd sandbox, which is designed, not run;
- the server's real load.

**Not verified:**
- `rav1e`'s compiled-unsafe count beyond the source reading in §5.1;
- the provenance of the smaller helpers;
- release notes beyond the commit feeds;
- whether the notice, the export and the moderation text meet the GDPR's
  and the DSA's duties, which is the owner's legal check;
- a third-party audit of any package.

## Appendix A: the new packages of design A

Resolved on 12.11.2026 against the lock at `5529fe4`, with `png` 0.18.1 and
`ravif` 0.13.0 without default features.
- **"Built"** means compiled for x86_64 Linux. The same holds for aarch64
  Linux, both macOS targets and Windows MSVC.
- **"Unsafe sites"** counts textual markers in all of the package's Rust
  files, tests and cfg-inactive code included.
- **"Native files"** counts C, C++ and assembly sources in the package.
- **"Features"** are those the graph enables.

| Package | Licence | Built | Build script | Unsafe sites (files) | Native files | Crate root | Features |
| --- | --- | --- | --- | ---: | ---: | --- | --- |
| adler2 2.0.1 | 0BSD OR MIT OR Apache-2.0 | yes | no | 0 (0) | 0 | forbid | – |
| aligned 0.4.3 | MIT OR Apache-2.0 | yes | no | 12 (1) | 0 | – | – |
| aligned-vec 0.6.4 | MIT | yes | no | 56 (2) | 0 | – | default, std |
| arbitrary 1.5.0 | MIT OR Apache-2.0 | lock only | no | 5 (3) | 0 | – | – |
| arg_enum_proc_macro 0.3.4 | MIT | yes | no | 0 (0) | 0 | – | – |
| as-slice 0.2.1 | MIT OR Apache-2.0 | yes | no | 0 (0) | 0 | – | – |
| av-scenechange 0.14.1 | MIT | yes | yes | 48 (15) | 27 (only with `asm`) | – | – |
| av1-grain 0.2.5 | BSD-2-Clause | yes | no | 16 (4) | 0 | – | create, default, diff, estimate, nom, num-rational, parse, v_frame |
| avif-serialize 0.8.9 | BSD-3-Clause | yes | no | 0 (0) | 0 | – | – |
| bitstream-io 4.10.0 | MIT/Apache-2.0 | yes | no | 0 (0) | 0 | forbid | alloc, default, std |
| built 0.8.1 | MIT | yes (build-time) | no | 1 (1) | 0 | – | – |
| crc32fast 1.5.5 | MIT OR Apache-2.0 | yes | yes | 23 (2) | 0 | deny | default, std |
| crossbeam-deque 0.8.8 | MIT OR Apache-2.0 | yes | yes | 42 (1) | 0 | – | default, std |
| crossbeam-epoch 0.9.21 | MIT OR Apache-2.0 | yes | yes | 192 (11) | 0 | – | alloc, std |
| equator 0.4.2 | MIT | yes | no | 46 (5) | 0 | – | – |
| equator-macro 0.4.2 | MIT | yes | no | 3 (1) | 0 | – | – |
| fdeflate 0.3.7 | MIT OR Apache-2.0 | yes | no | 0 (0) | 0 | forbid | – |
| flate2 1.1.11 | MIT OR Apache-2.0 | yes | no | 37 (4) | 0 | – | any_impl, default, miniz_oxide, runtime_detection, rust_backend |
| imgref 1.12.3 | CC0-1.0 OR Apache-2.0 | yes | no | 12 (1) | 0 | – | default, deprecated |
| interpolate_name 0.2.4 | MIT | lock only | no | 0 (0) | 0 | – | – |
| libfuzzer-sys 0.4.13 | (MIT OR Apache-2.0) AND NCSA | lock only | yes | 9 (1) | 53 | – | default, link_libfuzzer |
| loop9 0.1.5 | MIT | yes | no | 1 (1) | 0 | – | – |
| maybe-rayon 0.1.1 | MIT | yes | no | 0 (0) | 0 | – | – |
| miniz_oxide 0.8.9 | MIT OR Zlib OR Apache-2.0 | yes | no | 0 (0) | 0 | forbid | default, simd, simd-adler32, with-alloc |
| miniz_oxide 0.9.1 | MIT OR Zlib OR Apache-2.0 | yes | no | 0 (0) | 0 | forbid | default, simd, simd-adler32, with-alloc |
| new_debug_unreachable 1.0.6 | MIT | yes | no | 3 (3) | 0 | – | – |
| no_std_io2 0.9.4 | Apache-2.0 OR MIT | yes | no | 11 (3) | 0 | – | alloc, default, std |
| nom 8.0.0 | MIT | yes | no | 4 (1) | 0 | – | alloc, default, std |
| noop_proc_macro 0.3.0 | MIT | yes | no | 0 (0) | 0 | – | – |
| num-derive 0.4.2 | MIT OR Apache-2.0 | yes | no | 0 (0) | 0 | – | – |
| num-rational 0.4.2 | MIT OR Apache-2.0 | yes | no | 0 (0) | 0 | – | default, num-bigint, std |
| pastey 0.1.1 | MIT OR Apache-2.0 | yes | no | 0 (0) | 0 | – | – |
| png 0.18.1 | MIT OR Apache-2.0 | yes | no | 0 (0) | 0 | forbid | – |
| profiling 1.0.18 | MIT OR Apache-2.0 | yes | no | 0 (0) | 0 | – | default, procmacros, profiling-procmacros |
| profiling-procmacros 1.0.18 | MIT OR Apache-2.0 | yes | no | 0 (0) | 0 | – | – |
| quick-error 2.0.1 | MIT/Apache-2.0 | yes | no | 0 (0) | 0 | – | – |
| rav1e 0.8.1 | BSD-2-Clause | yes | yes | 319 (50) | 94 (only with `asm`) | – | wasm, wasm-bindgen (wasm32 only) |
| ravif 0.13.0 | BSD-3-Clause | yes | no | 0 (0) | 0 | – | – |
| rayon 1.12.0 | MIT OR Apache-2.0 | yes | no | 67 (11) | 0 | – | – |
| rayon-core 1.13.0 | MIT OR Apache-2.0 | yes | yes | 87 (9) | 0 | – | – |
| rgb 0.8.53 | MIT | yes | no | 77 (9) | 0 | – | – |
| simd-adler32 0.3.10 | MIT | yes | no | 36 (6) | 0 | – | const-generics, default, std |
| simd_helpers 0.1.0 | MIT | yes | no | 0 (0) | 0 | – | – |
| v_frame 0.3.9 | BSD-2-Clause | yes | no | 8 (1) | 0 | – | – |
| y4m 0.8.0 | MIT | yes | no | 3 (1) | 0 | – | – |
| zlib-rs 0.6.8 | Zlib | lock only | no | 463 (30) | 0 | – | rust-allocator, std |
