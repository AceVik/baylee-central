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
Original files remain untouched. The short dedication uses only the details
the owner supplied; it does not invent a birthday, biography or age.
