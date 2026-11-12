# Picture fixtures

Synthetic pictures for `tests/codec`, drawn on 12.11.2026 from pixels made
by a script (a 64 × 64 quadrant: the top-left quarter red, the rest blue;
48 × 32 for the wide one); no photo, no person. The other formats are drawn
by the tests themselves with the encoders in the build.

| File | Made with |
| --- | --- |
| `quadrant.jpg` | `cjpeg -quality 95` (libjpeg-turbo), baseline |
| `progressive.jpg` | `cjpeg -quality 95 -progressive` |
| `grey.jpg` | `cjpeg -quality 95 -grayscale` |
| `lossy.webp` | `cwebp -q 90` |
| `animated.webp` | `img2webp -lossless`, a red frame then a blue one |
| `imageio.avif` | macOS `sips -s format avif`, 48 × 32 quadrant |
| `ten-bit.avif` | `ffmpeg` with SVT-AV1, 48 × 32 of `#336699`, 10-bit 4:2:0 |
