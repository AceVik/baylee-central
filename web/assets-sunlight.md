# Sunlight asset provenance

The owner requested Central's adaptation on 27 October 2026, using the approved
Baylee memorial assets. All sources come from the sibling `AceVik/baylee`
repository at commit `4fcd41de1` under `crates/baylee-client/assets/`.
Its `art/baylee/memorial/assets.json` records the generation identifiers and
photograph sources; `art/baylee/memorial/prompts.md` preserves the iterations.

The cloud uses the explicitly approved fourth portrait: Baylee remains small,
slender and curled up beside garden birds and pigeons. No new portrait was
generated or painted for Central. The cloud master SHA-256 is
`1c76c23d31ef8f447d3e403ecef6df92326f64f8d6d67c1a5912cfbb7eec316e`.
These are project-created visuals and owner-supplied photos, not Wizards assets.

| Website asset | Source in Baylee | Preparation |
| --- | --- | --- |
| `public/img/baylees-world.webp` | `scenes/baylees-world.png` | 1672 × 941, WebP quality 91 |
| `public/img/baylee-cloud.webp` | `scenes/baylee-cloud.png` | 1200 × 800, WebP quality 95, alpha quality 100 |
| `public/img/baylee-sunlight-960.webp` | `brand/baylee-logo.png` | 960 × 400, WebP quality 95 |
| `public/img/baylee-sunlight-480.webp` | `brand/baylee-logo.png` | 480 × 200, WebP quality 95 |
| `public/img/baylee-sunlight-icon.webp` | `brand/baylee-icon.png` | 192 × 192, WebP quality 95 |
| `public/favicon-32.png` | `brand/favicon-32.png` | Exact copy |
| `public/apple-touch-icon.png` | `brand/apple-touch-icon.png` | Exact copy |
| `public/favicon.ico` | `brand/baylee-icon.png` | Pillow, 16/32/48 px |
| `public/img/og.jpg` | `scenes/baylees-world.png` | FFmpeg resize and centre crop to 1200 × 630 |
| `public/img/memorial/baylee-{1..5}.jpg` | `memorial/baylee-{1..5}.jpg` | Exact copies of the existing metadata-stripped, maximum 1400 px derivatives |

WebP derivatives use `cwebp`, preserving aspect ratio with `-resize WIDTH 0`.
For example: `cwebp -q 95 -alpha_q 100 -resize 1200 0 INPUT.png -o OUTPUT.webp`.
Only size/format conversion was applied. Hover light, halo and movement are
separate browser layers, so they never repaint Baylee's likeness.

The photos, in album order, derive from the owner's `PXL_20260212_084326728.jpg`,
`PXL_20260127_075332867.jpg`, `PXL_20260113_125944795.jpg`,
`PXL_20260131_092837183.jpg` and `PXL_20260212_084306146.jpg`.
Original files remain untouched. The full approved family remembrance now replaces the short draft: eight
chapters, 37 paragraphs, identical to the native client in each language.
The date range explicitly describes her time with the family; no birthday
is invented. The cloud in the album masthead uses the same approved asset.

## Parchment album and ten photographs — 27 October 2026

The five additional owner-supplied photos are, in album order 6–10:
`PXL_20260116_225450836.MP.jpg`, `PXL_20260324_064651597.jpg`,
`PXL_20260409_112337689.jpg`, `PXL_20251220_022019300.jpg`, and
`PXL_20260307_202151314.jpg`. New previews have a maximum side of 960 px.
All ten `public/img/memorial/full/baylee-{1..10}.jpg` preserve the full original
4032-pixel-side composition. FFmpeg decodes the original and writes JPEG quality
2 with `-map_metadata -1`; motion-photo trailers and capture/location metadata
are not shipped. The owner’s source files are unchanged. Hashes and provenance
are recorded in Baylee's `art/baylee/memorial/assets.json`.

Only the small preview uses cover cropping, individual subject anchors, rounded
corners and a mild CSS vignette. The modal uses contain at fit, explicit bounded
zoom and drag. Full images load only when opened. Stationary parchment and
italic ink carry the complete 37 paragraphs; chapter six uses the family's
requested description of increasing size and weeping. Page leaves, finite slide
transitions and layered shadows respect reduced motion; controls never scale.
