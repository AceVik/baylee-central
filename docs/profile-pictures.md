# Profile pictures

*Built 12.11.2026 on the owner's decisions of the same day
([proposal §0](dependency-proposal/profile-picture.md)). Not deployed.*

An account may have one profile picture. Everyone can see it, signed in or
not, wherever its display name is shown. Central keeps it as a 256 × 256
AVIF and a 256 × 256 PNG, both made by Central from pixels alone, and a
picture that moves also as an AVIF image sequence.

## In short

- **The browser converts first.** The profile page opens the chosen file
  in the browser, turns it upright, lets the person place a square, and
  sends a lossless square PNG of it (256 to 512 px). The photo's details
  (place, camera, time) never leave the device.
- **Central also takes originals.** Where the browser cannot open a file,
  or will not let the page read its own canvas (a hardened browser), the
  page sends the file itself. Central then decodes it and keeps its
  centred square. The page says so first.
- **Pictures that move go as they are.** A canvas holds one frame, so
  the page sends an animated GIF, WebP or APNG itself, says so, and does
  not offer to place the square. The worker composites its frames within
  bounds and hands them, as raw pixels, to ffmpeg with `libaom-av1`,
  which writes an AVIF image sequence ([Animations](#animations)). The
  first frame is the still. Where ffmpeg is missing or fails, Central
  keeps the still alone, the page says so and the service logs it.
- **Every decoder runs only in the worker.** That is a fresh process per
  picture (`central-picture-worker`), with no secret, no network, no file
  but a private `/tmp` (where ffmpeg writes an animation's sequence), and
  memory and time limits. The service links none of the
  codecs (`central-picture` holds only the protocol and the checks), so a
  bug in a decoder reaches nothing of value.
- **The service checks the worker's answer.** It must be a PNG master of
  exactly IHDR, IDAT and IEND at 256 px with every CRC right, and an AVIF
  of exactly `ftyp` (`avif`), `meta` and `mdat`. The service computes the
  master's SHA-256 itself, and reports and decisions key on it.
- **Pictures are public under a random key.** The key is 16 bytes of
  independent randomness and new at every upload; it is never the account
  id or the tag. Removal and account deletion make both URLs answer 404 at
  once; a browser may keep a copy it has for up to an hour.
- **Reports and moderation.** Any signed-in account can report a picture
  with a reason from a closed list and an optional note of up to 1,000
  characters. Administrators see the reporter; the reported person never
  does. An administrator keeps or removes a picture: a removal deletes the
  bytes at once, tells the person the reason, and may block new pictures
  for 7 or 30 days or until lifted. Only the digest stays with the
  reports, and decided reports are kept for one year.

## Formats

| Format | Decoder (worker) | Notes |
| --- | --- | --- |
| PNG, APNG | `png` 0.18.1 | every depth to 8 bits; text and ICC chunks skipped unread; CRCs and Adler-32 checked; an APNG's frames |
| JPEG | `zune-jpeg` 0.5.15, scalar | baseline, progressive, grey; EXIF orientation read by our own TIFF reader and applied |
| GIF | `gif` 0.14.2 | drawn on the logical screen; an animation's frames |
| WebP | `image-webp` 0.2.4 | lossy, lossless, extended; an animation's frames, composited by `image-webp` |
| AVIF | `avif-decode` 3.0.0 over `avif-parse` 2.1.0 and `rav1d` 1.1.0 | the primary item and its alpha; 8, 10 and 12 bits |
| SVG | `resvg` 0.50.0 without text, fonts, svgz or rasters | rasterised straight at 256 px; no reference is resolved; never served as SVG |
| BMP, ICO | `image` 0.25.10 (bmp and ico only) | an icon's largest entry; cursors refused |

A file is taken only when its first bytes say the format the request
declared (`Content-Type`); a file name or extension decides nothing. The
browser's own formats beyond these (HEIC and JPEG XL in Safari) reach
Central only as the page's PNG.

## Animations

An animated GIF, WebP or APNG (two frames or more) is read frame by frame
in `crates/central-picture-codec/src/animation.rs`:

- **Compositing.** Every frame is laid on one canvas of the picture's own
  size, as the format says:
  - GIF: drawn where the frame says, its transparent pixels leaving the
    canvas as it was; then kept, cleared, or restored to the canvas before
    it;
  - APNG: each `fcTL` frame replaces its rectangle (`SOURCE`) or is laid
    over it (`OVER`, straight alpha); then kept, cleared, or restored (a
    first frame's restore clears, as the specification says); a default
    image that is not a frame is skipped;
  - WebP: `image-webp` composites its frames itself.
- **Each composited canvas** is cut to its centred square and averaged to
  256 px at once, so the worker never holds more than the canvas, its copy
  for a restore, and the small frames.
- **The still** is the first composited frame. It becomes the AVIF and the
  PNG master exactly as any still does.
- **Timing.** A frame shorter than 20 ms plays for 100 ms, as browsers
  play it; the loop count is kept (0: for ever).
- **Bounds stop, they do not refuse.** Past 100 frames, 10 s of play, or
  200 million composited canvas pixels, the animation keeps its
  beginning. A canvas over 2048² is taken as a still. A frame that cannot
  be read ends the animation there; a file of one frame is a still.
- **Encoding** (owner, 12.11.2026, [proposal §0.8](dependency-proposal/profile-picture.md)),
  in `src/ffmpeg.rs`:
  - ffmpeg is a child of the worker: an absolute path from the worker's
    command line (`--ffmpeg`, default `/usr/bin/ffmpeg`), no shell, no
    environment, fixed arguments, only the pipe as input and one file as
    output, in a fresh directory only the worker may enter (the `avif`
    muxer cannot write to a pipe), removed after;
  - it gets the composited 256 px frames as raw RGBA and never sees the
    uploaded file;
  - `libaom-av1` on one thread, speed 6, constant rate factor 32, then 40
    and 48 until the sequence fits 512 KiB; 12 s for the probe and every
    attempt together, then it is ended;
  - every frame keeps its duration, the last one included: frames are
    timed in milliseconds and the input's frame rate is the last frame's;
  - an opaque picture has one track, a transparent one an alpha track as
    well; 4:2:0, 8 bits, the stills' colour description.
- **Checks.** The worker keeps the sequence only when its sample tables
  hold every frame and every duration. Otherwise the still stands alone,
  with the reason in the answer (`no_encoder`, `failed`, `too_large`);
  the service logs a fixed line for each, with a count, and never fails
  the upload for it. The service checks the sequence's framing again.
- **An AVIF image sequence** uploaded (major brand `avis`) is refused as
  `unsupported`: `avif-parse` 2.1.0 refuses it and no accepted package
  decodes AV1 sequences safely. The page draws its first frame and sends
  the square, as for any still.
- **The website.** The page sees an animation in the file's first MiB:
  an APNG's `acTL` before its image data, a WebP's animation flag, or a
  second GIF image. It then sends the file itself and says so
  (`picture-moving`). It plays a stored sequence only where the system
  allows motion (`prefers-reduced-motion: no-preference`), and says when
  an animation was kept as its still (`picture-kept-still`).

## Limits

Each limit is checked before the work it guards:

| What | Bound | Checked by |
| --- | --- | --- |
| Upload | 8 MiB; SVG and ICO 1 MiB | the route, from `Content-Length`, before reading |
| Body time | 20 s, at least 16 KiB every 5 s | the route |
| Sides | 16 to 8,192 px, at most 4096² = 16.8 million pixels | our header readers (PNG IHDR, JPEG SOF, GIF screen, WebP VP8/VP8L/VP8X, AVIF `ispe`, BMP DIB, every ICO entry), then each decoder's own limits |
| AV1 frame size | `frame_width_bits` and `frame_height_bits` of every sequence header, so no frame can exceed the sides or the pixel bound | our reader of the AV1 sequence header, before `rav1d`, which avif-decode leaves unlimited |
| SVG | nesting 128, 20,000 elements | our scan, before `usvg`, whose conversion recurses |
| Animation | 100 frames, 10 s of play, a canvas of 2048², 200 million composited pixels | `animation.rs`, frame by frame, before each frame's work |
| Decoder memory | 128 MiB per decoder | `png`, `gif`, `image-webp`, `image` limits |
| Worker time | the worker ends itself at 19 s (ffmpeg's part at most 12 s, and done by 17 s); the service ends it at 20 s and holds its conversion slot until then | the worker, the service's client, and `RuntimeMaxSec=20` |
| Worker memory | `MemoryMax=512M`, ffmpeg included | systemd, production only |
| Sequence | 512 KiB, 2 to 100 samples, one or two tracks | the worker, the service's check, the schema |
| At once | 2 conversions; 1 per account | the service, and the socket's `MaxConnections=2` |
| Budget | 10 uploads an hour, 30 a day per account | the database, counted once the body was read |
| Reads | 600 a minute and 16 in flight per address | the route |

**Measured** with the release worker on Apple Silicon (12.11.2026, load
average about 30). Peak resident memory and CPU time per picture:
- a 4096² PNG: 74 MiB, 0.21 s;
- a 4096² baseline JPEG: 74 MiB, 0.21 s;
- a 4096² progressive JPEG: 122 MiB, 0.25 s;
- 640 × 480 AVIF, WebP and GIF: 13 MiB, about 0.2 s;
- an SVG: 11 MiB, 0.08 s.

Heavy SVG filters can take minutes; the worker is ended at its deadline
(test `heavy_svgs_end_within_the_worker_s_time_and_the_next_picture_converts`).
Nothing was measured on x86-64.

## The worker and its protocol

- **Request** on standard input: `BCPQ`, version 2, the format's byte, the
  length (u32, big-endian), the file. Read with its bound; anything after
  the frame is refused.
- **Answer** on standard output: `BCPA`, version 2, then either 0 with the
  AVIF and the PNG, each length-prefixed, and the motion's byte (`still`,
  `sequence` followed by the length-prefixed sequence, `no_encoder`,
  `failed`, `too_large`), or 1 with a refusal's byte. The
  refusals are `unsupported`, `wrong_type`, `too_large`,
  `too_many_pixels`, `too_small`, `malformed` and `failed`.
- **Panics.** A decoder that panics refuses the file (`malformed`). The
  tests found two that do: `avif-parse` 2.1.0 (its parser-state
  assertions) and `resvg` 0.50.0's turbulence filter (an overflow).
- **Aborts.** `rav1d` 1.1.0 aborts the whole process on some damaged AV1
  (an `unwrap` inside its C ABI, where a panic cannot unwind). The service
  sees a worker that ended without an answer and refuses the picture
  (`failed`); the next picture converts as before (test
  `a_decoder_that_aborts_the_worker_harms_nothing`).
- **In development** the service starts the binary beside it as a child:
  no environment, the file system's root as its directory, its pipes only,
  killed at the deadline and always waited for. Nothing bounds its memory
  but the decoders' own limits.
- **In production** systemd starts one per connection to
  `/run/baylee-central/picture.sock` (`ops/central-picture.socket` and
  `ops/central-picture@.service`): `DynamicUser`, `PrivateNetwork`,
  `RestrictAddressFamilies=none`, `ProtectSystem=strict`, no
  capabilities, `@system-service` minus `@privileged` and `@resources`,
  `MemoryDenyWriteExecute`, `PrivateIPC`, nothing executable but the
  worker, ffmpeg and the system's libraries (`NoExecPaths=/`, `ExecPaths=`),
  a private 64 MiB `/tmp`, one CPU, 512 MiB, 32 tasks
  and 20 s; ffmpeg runs inside the same unit.

**The output.** The centred square, averaged exactly to 256 px (integer
area averaging weighted by alpha; a fully transparent pixel's colour
becomes 0). Then two pictures:
- a PNG master, RGB when opaque and RGBA otherwise, compressed at the
  highest level; the same pixels give the same bytes;
- an AVIF from `ravif` 0.13.0 with `rav1e` 0.8.1's assembly, one thread,
  8 bits, quality 80 and speed 8. If it is larger than 160 KiB it is made
  again at 60, then 40.

Every stored file says BT.709 primaries, the sRGB curve, BT.601
coefficients and full range; the stills are 4:4:4, the sequences 4:2:0.

## What the service stores and serves

Migration 000027 ([postgresql.md](postgresql.md#profile-pictures-migration-000027))
adds four tables:
- `pictures`: one row per account;
- `picture_reports`;
- `picture_notices`: the last removal's notice and the block;
- `picture_budgets`.

**`PUT /api/v1/picture`** on the browser listener: the raw file, its media
type in `Content-Type`. The checks run in this order:
1. Headers alone: the loopback proxy, `Host`, `Origin`, same-origin fetch
   metadata, `X-Baylee-Central: 1`, no bearer, the client's address and a
   place in flight, `Content-Length` within the format's bound.
2. The account, before a byte of the body is read: the session, Central's
   terms, no block, room in the budget.
3. The body is read, within the listener's places per address. Only then
   a conversion slot (two in all, one per account), which stays with the
   conversion until the worker ended; the budget is spent from here,
   stored or not.
4. The worker converts it; the service checks the answer and computes the
   digest: SHA-256 of the PNG master followed by the sequence, if any, so
   a report names the animation that was shown.
5. One transaction locks the account, checks the session and the block
   again, stores the picture under a new key, and marks open reports of
   the replaced picture `gone`.

The answer is `MyPicture` as JSON, or `422 {"refusal": …}` for a picture
Central refused, or `403 {"refusal": "blocked"}`, or the listener's usual
statuses. A picture's refusal is never 401, which the page reads as an
ended session.

**`GET /api/v1/pictures/{key}.avif` and `.png`**, and
**`{key}.sequence.avif`** for a picture that moves, to anyone:
- same-origin or direct (`Sec-Fetch-Site` absent, `same-origin` or
  `none`), no bearer;
- answered with `Content-Security-Policy: default-src 'none'; sandbox`,
  `nosniff`, `Cross-Origin-Resource-Policy: same-origin`,
  `Content-Disposition: inline`, `Cache-Control: private, max-age=3600` and
  the digest as `ETag`;
- every refused or unknown read is the same `404`, `no-store`;
- other methods `405`.

The page draws
`<picture><source type="image/avif" srcset="….avif"><img src="….png"></picture>`,
so the browser chooses; for a picture that moves, a first source
`….sequence.avif` with `media="(prefers-reduced-motion: no-preference)"`.
**Where the website shows pictures**, each in a mark of one fixed size,
hidden from screen readers since the name stands beside it, with the
initials (a group's sign, or in the header a neutral mark) where there is
no picture:
- the header's account link: the account's own, which the profile page
  notes in the tab's `sessionStorage` (`baylee-central-own-picture`: the
  key and whether it moves; removed at sign-out), so no page asks for it;
- the profile page's card;
- the messenger: one's own, the conversation list, the conversation's
  head, a group message's author, contacts, requests, group members, a
  group invitation (its sign), and the safety number's view.

The messenger reads the keys of all the tags it shows in one request (at
most 300), again only when the contacts or groups change.
Clients learn whether a picture moves from `TaggedPicture.moving` and
`Picture.sequence_bytes`, without a download. Administrators see the
animation in the moderation list, and a link to open it alone.

**Who learns a key.** A key is learned only from answers that name an
account already:
- one's own (`Request::Picture { Mine }`);
- `SocialRequest::Pictures { tags }`, at most 300 tags. A tag outside the
  caller's contacts and groups spends a unit of the existing lookup budget
  (120 an hour), all or none. Unknown tags, accounts without a picture and
  pairs with a block either way between them are absent.
- the administration.

The bytes themselves are public: a block withholds the key, not the
picture.

## Reports and moderation

**Filing:** `SocialRequest::ReportPicture { tag, key, reason, note }`.
- One's own picture, a key that is not the account's current one, and a
  picture across a block are all `Rejected` alike.
- A second open report of the same picture by the same reporter is
  `Conflict`.
- More than 20 reports an hour, or 100 open, is `Busy`.
- The report holds the digest, so a decision applies to exactly the
  picture reported.

**Deciding:** `AdminRequest::Picture { … }`.
- **`Reports`** (50 a page, by state) and **`Case`**: reading, by an
  administrator's session.
- **`Decide { Keep }`**: the open reports become `kept`, audited
  `picture.kept`.
- **`Decide { Remove { reason, note, block } }`**: the picture is deleted,
  its reports become `removed`, the person's notice is written (reason
  and a sentence of at most 500 characters), with an optional block.
  Audited `picture.removed` (detail: the reason code, never the note), and
  `picture.blocked` with a block.
- **`LiftBlock`**: audited `picture.unblocked`.
- Every change needs a password sign-in less than 30 minutes old.
- `Conflict` when the picture changed meanwhile.

**The person** sees the notice on the profile page: when, the reason, the
administrator's sentence and the way to object (the contact form), and
whether and until when uploads are blocked. A new picture ends the notice.

**Retention.**
- The runtime cannot delete a report. The operator's sweep deletes decided
  reports a year after their decision.
- Deleting an account deletes its picture, its notice, its budget and the
  reports about it.
- Reports it filed lose their reporter, and decisions it made lose their
  decider.

## Export

The export page adds the picture's facts (`profile_picture`) and the
reports the account filed (`picture_reports`) to the export file, and
offers the PNG master as a download. `AccountExport` itself is unchanged.
Reports about one's own picture are not exported with their reporters.

## Tests

- **`crates/central-picture`:** sniffing; the protocol's framing, bounds
  and every cut; the answer checks (every bit of a master breaks a CRC);
  the client against stand-in workers that crash, hang, babble, echo or
  answer malformed; its empty environment.
- **`crates/central-picture-codec` (`tests/codec`):**
  - every format;
  - every PNG colour type and depth;
  - interlacing and APNG;
  - bombs in IDAT, zTXt and iCCP;
  - headers that claim too much, per format;
  - broken CRCs and Adler-32, and every cut;
  - polyglots (PNG, JPEG, GIF and AVIF followed by HTML or a ZIP's end);
  - EXIF orientation;
  - JPEG, GIF, WebP and BMP damage;
  - AVIF from three encoders, a forged sequence header, other brands;
  - SVG with scripts, events, foreign content, text, CSS imports, `file:`,
    relative, remote and `data:` references, entity bombs, deep nesting,
    crowds, heavy filters, patterns and self-references;
  - animations (`animations.rs`): GIF disposal (keep, background,
    previous) and transparency; loops and short delays; frame bombs of
    thousands of frames and endless plays; huge canvases of tiny frames;
    APNG `SOURCE` and `OVER` with every disposal; a separate default
    image; an APNG that claims more frames than it has; WebP; the still
    agreeing with the first frame;
  - sequences (`sequences.rs`): stand-in encoders that crash, hang,
    write nothing, garbage, too few frames, other durations or too much,
    each keeping the still with its reason, the encoder's empty
    environment and private directory, and nothing left behind; with a
    real `libaom-av1`: every duration kept, the last one included, one or
    two tracks, one colour description for still and sequence, and the
    worker binary's `--ffmpeg`. Those need an ffmpeg with `libaom-av1`
    (`PICTURE_FFMPEG`; on a host without one, Ubuntu 26.04's in a
    container through `tests/ffmpeg/ffmpeg`, built from the `Dockerfile`
    beside it). Without one they say SKIPPED, except where
    `PICTURE_REQUIRE_FFMPEG=1`, which CI sets: there they fail;
  - an uploaded AVIF image sequence refused as `unsupported`;
  - seeded mutations of every format;
  - the worker binary behind the real client.
- **`crates/central-http`:** every header refusal before the body and the
  service; admission before the body; length mismatches; a stalled body;
  every named refusal; reads of all three files to anyone with the headers
  byte for byte; the same 404 for every refusal, other sequence names and
  an unknown key's sequence; 405; the read budget.
- **PostgreSQL fixture:**
  - set, replace and remove under new keys, a picture that moves among
    them (its sequence served, its size the owner's, gone with it);
  - the budget;
  - who learns a key, with the lookup budget and blocks;
  - reports and their bounds;
  - decisions with a fresh sign-in and the exact audit;
  - blocks;
  - the runtime's grants both ways;
  - the sweep, deletion's cascade, and the upgrade over 000026 with the
    sequence column's bounds.
- **The site (`crates/central-web/browser/e2e-site/picture.spec.ts`):**
  - PNG, JPEG, GIF, WebP, AVIF, BMP, ICO and SVG through the browser,
    checked by decoding what was sent;
  - EXIF orientation;
  - an SVG's script that never runs;
  - headers refused before decoding;
  - the file sent as it is where the browser cannot open it or will not
    let the page read its canvas;
  - animated GIF, WebP and APNG sent as they are, without the square's
    controls, and a still PNG still drawn;
  - a stored sequence played only where motion is allowed, and an
    animation kept as its still said so;
  - keyboard and zoom;
  - named refusals;
  - removal with confirmation;
  - 320 px in both schemes with axe;
  - `blob:` in the profile page's CSP only;
  - the administrators' list, with a moving picture and the link to its
    animation.
- **Fuzzing:** `fuzz/` (outside the workspace) holds a seeded mutation
  fuzzer for every decoder entry point, not coverage-guided;
  [its README](../fuzz/README.md) says why and what ran.

## The whole path against a real Central (12.11.2026)

`web/e2e-real/site-picture.spec.ts` runs a picture's whole path against a
throwaway Central built from this branch:
1. The browser prepares a still and sends it, and sends an animated GIF
   whole, through `front.ts`, which lets the upload through to 9 MiB as the
   Caddy handle does.
2. Central hands each one to the worker through a Unix socket.
   `picture-socket.ts` stands in for systemd's activation: one worker per
   connection, two at a time. The worker was the Linux build, in an
   Ubuntu 26.04 container whose ffmpeg has libaom, without network, with a
   read-only root, a 64 MiB `/tmp`, 512 MiB of memory and 32 tasks.
3. Central stores what comes back and serves it.

What it checks:
- **The still:** a noisy PNG over 64 KiB comes back as a 256 × 256 PNG and an
  AVIF (`ftypavif`), with the served headers (CSP sandbox, `nosniff`,
  CORP, `private, max-age=3600`) and no sequence.
- **The animation:** it comes back as an AVIF sequence (`ftypavis`), shown
  only where motion is welcome. The replaced still answers 404 at once.
- **Elsewhere on the site:** the header's face, and the contact's
  conversation list, show the picture.
- **The report:** the contact reports it from the chat's details, with a
  reason and a note that stays text.
- **The removal:** an administrator the operator appointed removes it
  with a reason and a sentence, after the password once more.
- **Afterwards:** the person sees the notice with the reason, and every file
  answers 404, `no-store`.

Result: it passed at a desktop's and a phone's size, each against a
Central started afresh. It is a local run, not part of CI. Not covered
here: systemd's own sandbox (the post-deploy check), x86-64, and any
browser but Chromium.

## Not done here

- The native client's own route and conversion (M6): the native listener
  refuses picture requests.
- Gateways: they receive no picture.
- Translations: the nine languages besides English and German show the
  English words.
- The legal texts: the owner must read the privacy notice's new section
  (`privacy-picture` in `site.ftl`, and `legal/privacy.*.md`).
- The game client's decoder: the owner chose `rav1d` 1.1.0 behind a small
  wrapper in the game client ([proposal §11](dependency-proposal/profile-picture.md)).
- Not tested here: ffmpeg inside the production unit's sandbox (its
  system-call filter, `MemoryDenyWriteExecute`, the task limit), anything
  on x86-64, and Firefox and Safari playing the sequences; the operator's
  post-deploy check ([operations](operations.md#profile-pictures)) covers
  the first.
