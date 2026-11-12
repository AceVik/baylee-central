//! The centred square, averaged exactly to 256 × 256 and turned upright.
//!
//! Each output pixel is the average of the source area it covers, weighted
//! by the share of each source pixel inside it and by its alpha, in integer
//! arithmetic, so the result is the same on every machine. A fully
//! transparent pixel's colour becomes 0: nothing hides under transparency.
use super::{Raster, Refusal};

/// The stored side.
pub(crate) const SIDE: u32 = central_picture::SIDE;
const OUT: usize = SIDE as usize;

/// 256 × 256 RGBA, upright.
pub(crate) struct Square(pub(crate) Vec<u8>);

/// The centred square of `raster`, averaged to [`SIDE`] and turned upright.
///
/// The centred square of the stored picture is the centred square of the
/// upright one (turning about the centre moves no centre), so the
/// orientation is applied to the small result alone.
pub(crate) fn square(raster: &Raster) -> Result<Square, Refusal> {
    square_of(
        raster.width,
        raster.height,
        &raster.rgba,
        raster.orientation,
    )
}

/// [`square`] of `rgba`, `width` × `height`, turned upright from
/// `orientation`: one frame of an animation (orientation 1).
pub(crate) fn square_of(
    width: u32,
    height: u32,
    rgba: &[u8],
    orientation: u8,
) -> Result<Square, Refusal> {
    let width = usize::try_from(width).map_err(|_| Refusal::Failed)?;
    let height = usize::try_from(height).map_err(|_| Refusal::Failed)?;
    if rgba.len() != width * height * 4 || width == 0 || height == 0 {
        return Err(Refusal::Failed);
    }
    let side = width.min(height);
    let (x0, y0) = ((width - side) / 2, (height - side) / 2);
    let spans = spans(side);
    let mut out = vec![0_u8; OUT * OUT * 4];
    // Per output row: Σ w·a·r, Σ w·a·g, Σ w·a·b, Σ w·a and Σ w per column.
    let mut sums = vec![[0_u64; 5]; OUT];
    for (oy, rows) in spans.iter().enumerate() {
        sums.fill([0; 5]);
        for &(sy, wy) in rows {
            let row = &rgba[((y0 + sy) * width + x0) * 4..][..side * 4];
            for (ox, columns) in spans.iter().enumerate() {
                let sum = &mut sums[ox];
                for &(sx, wx) in columns {
                    let p = &row[sx * 4..sx * 4 + 4];
                    let w = wx * wy;
                    let wa = w * u64::from(p[3]);
                    sum[0] += wa * u64::from(p[0]);
                    sum[1] += wa * u64::from(p[1]);
                    sum[2] += wa * u64::from(p[2]);
                    sum[3] += wa;
                    sum[4] += w;
                }
            }
        }
        for (ox, sum) in sums.iter().enumerate() {
            let pixel = &mut out[(oy * OUT + ox) * 4..][..4];
            let alpha = rounded(sum[3], sum[4]);
            if alpha == 0 || sum[3] == 0 {
                pixel.copy_from_slice(&[0, 0, 0, 0]);
            } else {
                pixel[0] = rounded(sum[0], sum[3]);
                pixel[1] = rounded(sum[1], sum[3]);
                pixel[2] = rounded(sum[2], sum[3]);
                pixel[3] = alpha;
            }
        }
    }
    Ok(Square(orient(&out, orientation)))
}

/// `numerator / denominator`, rounded half up, as a sample.
fn rounded(numerator: u64, denominator: u64) -> u8 {
    if denominator == 0 {
        return 0;
    }
    u8::try_from((numerator + denominator / 2) / denominator).unwrap_or(u8::MAX)
}

/// For each of the [`SIDE`] output positions, the source positions it covers
/// within `side` and the share of each, in units of 1/[`SIDE`] of a source
/// pixel; the shares of one output position add up to `side`.
fn spans(side: usize) -> Vec<Vec<(usize, u64)>> {
    (0..OUT)
        .map(|o| {
            // The output position covers [o·side, (o+1)·side) in units of
            // 1/SIDE source pixels.
            let (start, end) = (o * side, (o + 1) * side);
            let first = start / OUT;
            let last = (end - 1) / OUT;
            (first..=last)
                .map(|s| {
                    let from = start.max(s * OUT);
                    let to = end.min((s + 1) * OUT);
                    (s, u64::try_from(to - from).unwrap_or(0))
                })
                .filter(|&(_, w)| w > 0)
                .collect()
        })
        .collect()
}

/// `pixels` (an `OUT`² square) turned upright from EXIF orientation `o`.
fn orient(pixels: &[u8], o: u8) -> Vec<u8> {
    if o == 1 {
        return pixels.to_vec();
    }
    let last = OUT - 1;
    let mut out = vec![0; pixels.len()];
    for y in 0..OUT {
        for x in 0..OUT {
            // Where the upright pixel (x, y) is in the stored picture.
            let (sx, sy) = match o {
                2 => (last - x, y),
                3 => (last - x, last - y),
                4 => (x, last - y),
                5 => (y, x),
                6 => (y, last - x),
                7 => (last - y, last - x),
                8 => (last - y, x),
                _ => (x, y),
            };
            let from = (sy * OUT + sx) * 4;
            out[(y * OUT + x) * 4..][..4].copy_from_slice(&pixels[from..from + 4]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raster(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Raster {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&pixel(x, y));
            }
        }
        Raster {
            width,
            height,
            rgba,
            orientation: 1,
        }
    }

    fn at(square: &Square, x: usize, y: usize) -> [u8; 4] {
        square.0[(y * OUT + x) * 4..][..4].try_into().unwrap()
    }

    #[test]
    fn shares_add_up_for_every_size() {
        for side in [16, 100, 255, 256, 257, 511, 512, 1000, 4096] {
            for span in spans(side) {
                assert_eq!(
                    span.iter().map(|&(_, w)| w).sum::<u64>(),
                    side as u64,
                    "{side}"
                );
            }
        }
    }

    #[test]
    fn a_checkerboard_becomes_even_grey_and_halves_stay_halves() {
        let board = raster(512, 512, |x, y| {
            if (x + y) % 2 == 0 {
                [255, 255, 255, 255]
            } else {
                [0, 0, 0, 255]
            }
        });
        let grey = square(&board).unwrap();
        assert!(grey.0.chunks(4).all(|p| p == [128, 128, 128, 255]));
        let halves = raster(512, 512, |x, _| {
            if x < 256 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        });
        let out = square(&halves).unwrap();
        assert_eq!(at(&out, 0, 0), [255, 0, 0, 255]);
        assert_eq!(at(&out, 127, 200), [255, 0, 0, 255]);
        assert_eq!(at(&out, 128, 200), [0, 0, 255, 255]);
    }

    #[test]
    fn the_centred_square_is_taken_and_a_small_one_is_enlarged() {
        // A wide picture: only the middle square is kept.
        let wide = raster(300, 100, |x, _| {
            if (100..200).contains(&x) {
                [0, 255, 0, 255]
            } else {
                [255, 0, 0, 255]
            }
        });
        assert!(
            square(&wide)
                .unwrap()
                .0
                .chunks(4)
                .all(|p| p == [0, 255, 0, 255])
        );
        let tall = raster(16, 48, |_, y| {
            if (16..32).contains(&y) {
                [9, 9, 9, 255]
            } else {
                [200, 0, 0, 255]
            }
        });
        assert!(
            square(&tall)
                .unwrap()
                .0
                .chunks(4)
                .all(|p| p == [9, 9, 9, 255])
        );
    }

    #[test]
    fn nothing_hides_under_transparency() {
        let hidden = raster(256, 256, |_, _| [12, 34, 56, 0]);
        assert!(square(&hidden).unwrap().0.iter().all(|&b| b == 0));
        // a transparent pixel's colour does not tint its opaque neighbour
        let mixed = raster(512, 512, |x, _| {
            if x % 2 == 0 {
                [255, 0, 0, 0]
            } else {
                [0, 0, 255, 255]
            }
        });
        assert_eq!(at(&square(&mixed).unwrap(), 10, 10), [0, 0, 255, 128]);
    }

    #[test]
    fn every_orientation_turns_the_corner_where_it_belongs() {
        // A stored picture whose top-left quarter is red.
        let marked = |orientation| {
            let mut r = raster(256, 256, |x, y| {
                if x < 128 && y < 128 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 0, 255]
                }
            });
            r.orientation = orientation;
            square(&r).unwrap()
        };
        let red = [255, 0, 0, 255];
        // where the red quarter ends up, upright
        let corners = [
            (1, (0, 0)),
            (2, (255, 0)),
            (3, (255, 255)),
            (4, (0, 255)),
            (5, (0, 0)),
            (6, (255, 0)),
            (7, (255, 255)),
            (8, (0, 255)),
        ];
        for (orientation, (x, y)) in corners {
            assert_eq!(
                at(&marked(orientation), x, y),
                red,
                "orientation {orientation}"
            );
        }
        // 6 turns clockwise: the stored top row becomes the right column
        let mut top = raster(256, 256, |_, y| {
            if y == 0 {
                [0, 255, 0, 255]
            } else {
                [0, 0, 0, 255]
            }
        });
        top.orientation = 6;
        let turned = square(&top).unwrap();
        assert!((0..OUT).all(|y| at(&turned, OUT - 1, y) == [0, 255, 0, 255]));
    }

    #[test]
    fn a_broken_raster_is_refused() {
        let mut broken = raster(16, 16, |_, _| [0, 0, 0, 255]);
        broken.rgba.pop();
        assert!(square(&broken).is_err());
    }
}
