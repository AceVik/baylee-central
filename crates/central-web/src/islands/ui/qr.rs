//! Pure-Rust zero-dependency QR Code SVG encoder (ISO/IEC 18004, Versions 1–10,
//! Byte mode, Error Correction Level M).
//!
//! Used by the invite dialog to render crisp scannable friend-invite QR
//! codes without adding external dependencies to `Cargo.lock`.

use leptos::prelude::*;
use std::fmt::Write as _;

/// QR Version parameters for Error Correction Level M (ISO/IEC 18004 Tables 1 & 9).
#[derive(Clone, Copy, Debug)]
struct VersionSpec {
    version: u8,
    size: usize,
    total_data_codewords: usize,
    ec_per_block: usize,
    g1_blocks: usize,
    g1_data: usize,
    g2_blocks: usize,
    g2_data: usize,
    alignments: &'static [usize],
}

const VERSIONS: [VersionSpec; 10] = [
    VersionSpec {
        version: 1,
        size: 21,
        total_data_codewords: 16,
        ec_per_block: 10,
        g1_blocks: 1,
        g1_data: 16,
        g2_blocks: 0,
        g2_data: 0,
        alignments: &[],
    },
    VersionSpec {
        version: 2,
        size: 25,
        total_data_codewords: 28,
        ec_per_block: 16,
        g1_blocks: 1,
        g1_data: 28,
        g2_blocks: 0,
        g2_data: 0,
        alignments: &[6, 18],
    },
    VersionSpec {
        version: 3,
        size: 29,
        total_data_codewords: 44,
        ec_per_block: 26,
        g1_blocks: 1,
        g1_data: 44,
        g2_blocks: 0,
        g2_data: 0,
        alignments: &[6, 22],
    },
    VersionSpec {
        version: 4,
        size: 33,
        total_data_codewords: 64,
        ec_per_block: 18,
        g1_blocks: 2,
        g1_data: 32,
        g2_blocks: 0,
        g2_data: 0,
        alignments: &[6, 26],
    },
    VersionSpec {
        version: 5,
        size: 37,
        total_data_codewords: 86,
        ec_per_block: 24,
        g1_blocks: 2,
        g1_data: 43,
        g2_blocks: 0,
        g2_data: 0,
        alignments: &[6, 30],
    },
    VersionSpec {
        version: 6,
        size: 41,
        total_data_codewords: 108,
        ec_per_block: 16,
        g1_blocks: 4,
        g1_data: 27,
        g2_blocks: 0,
        g2_data: 0,
        alignments: &[6, 34],
    },
    VersionSpec {
        version: 7,
        size: 45,
        total_data_codewords: 124,
        ec_per_block: 18,
        g1_blocks: 4,
        g1_data: 31,
        g2_blocks: 0,
        g2_data: 0,
        alignments: &[6, 22, 38],
    },
    VersionSpec {
        version: 8,
        size: 49,
        total_data_codewords: 154,
        ec_per_block: 22,
        g1_blocks: 2,
        g1_data: 38,
        g2_blocks: 2,
        g2_data: 39,
        alignments: &[6, 24, 42],
    },
    VersionSpec {
        version: 9,
        size: 53,
        total_data_codewords: 182,
        ec_per_block: 22,
        g1_blocks: 3,
        g1_data: 36,
        g2_blocks: 2,
        g2_data: 37,
        alignments: &[6, 26, 46],
    },
    VersionSpec {
        version: 10,
        size: 57,
        total_data_codewords: 216,
        ec_per_block: 26,
        g1_blocks: 4,
        g1_data: 43,
        g2_blocks: 1,
        g2_data: 44,
        alignments: &[6, 28, 50],
    },
];

/// Quiet zone width in modules around the symbol (ISO standard minimum is 4).
const QUIET: usize = 4;

/// A rendered QR matrix (`size × size` dark/light modules).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QrMatrix {
    /// Grid side length in modules (without quiet zone).
    pub size: usize,
    /// Selected QR version (`1..=10`).
    pub version: u8,
    /// Selected mask pattern (`0..=7`).
    pub mask: u8,
    /// Row-major module bits (`true` = dark).
    pub modules: Vec<bool>,
}

impl QrMatrix {
    /// Returns whether module `(row, col)` is dark.
    #[must_use]
    pub fn get(&self, row: usize, col: usize) -> bool {
        self.modules
            .get(row * self.size + col)
            .copied()
            .unwrap_or(false)
    }

    /// Builds a compact SVG `<path d="...">` uniting horizontal runs of dark
    /// modules, offset by the 4-module quiet zone.
    #[must_use]
    pub fn svg_path(&self) -> String {
        let mut d = String::with_capacity(self.size * self.size / 2);
        for row in 0..self.size {
            let mut col = 0;
            while col < self.size {
                if !self.get(row, col) {
                    col += 1;
                    continue;
                }
                let start = col;
                while col < self.size && self.get(row, col) {
                    col += 1;
                }
                let run = col - start;
                let _ = write!(d, "M{},{}h{}v1h-{}z", start + QUIET, row + QUIET, run, run);
            }
        }
        d
    }

    /// Total SVG coordinate side length including quiet zones on all sides.
    #[must_use]
    pub const fn svg_size(&self) -> usize {
        self.size + QUIET * 2
    }
}

/// Encodes `data` in Byte mode at Error Correction Level M (Versions 1–10).
/// Returns `None` if `data` exceeds Version 10-M capacity (213 bytes).
#[must_use]
pub fn encode(data: &[u8]) -> Option<QrMatrix> {
    let spec = select_version(data.len())?;
    let data_codewords = build_data_codewords(spec, data)?;
    let stream = interleave_with_ecc(spec, &data_codewords);

    let mut base = vec![false; spec.size * spec.size];
    let mut fixed = vec![false; spec.size * spec.size];
    place_function_patterns(spec, &mut base, &mut fixed);
    place_data_bits(spec.size, &stream, &fixed, &mut base);

    let mut best_mask = 0_u8;
    let mut best_penalty = u32::MAX;
    let mut best_modules = Vec::new();

    for mask in 0_u8..8 {
        let mut candidate = base.clone();
        apply_mask(spec.size, mask, &fixed, &mut candidate);
        write_format_info(spec.size, mask, &mut candidate);
        if spec.version >= 7 {
            write_version_info(spec.size, spec.version, &mut candidate);
        }
        let penalty = evaluate_penalty(spec.size, &candidate);
        if penalty < best_penalty {
            best_penalty = penalty;
            best_mask = mask;
            best_modules = candidate;
        }
    }

    Some(QrMatrix {
        size: spec.size,
        version: spec.version,
        mask: best_mask,
        modules: best_modules,
    })
}

/// Encodes `text` as a QR matrix and renders an accessible `<svg>` Leptos view
/// with a solid white quiet zone and crisp module edges.
pub fn qr_svg(text: String, aria_label: String) -> AnyView {
    let Some(matrix) = encode(text.as_bytes()) else {
        return view! { <span class="qr-unavailable" role="status">{text}</span> }.into_any();
    };
    let dim = matrix.svg_size();
    let view_box = format!("0 0 {dim} {dim}");
    let full = dim.to_string();
    let path_d = matrix.svg_path();
    view! {
        <svg
            class="qr-svg"
            viewBox=view_box
            shape-rendering="crispEdges"
            role="img"
            aria-label=aria_label
        >
            <rect width=full.clone() height=full fill="#ffffff" />
            <path d=path_d fill="#111318" />
        </svg>
    }
    .into_any()
}

fn select_version(byte_len: usize) -> Option<VersionSpec> {
    VERSIONS.into_iter().find(|spec| {
        let count_bits = if spec.version <= 9 { 8 } else { 16 };
        let needed_bits = 4 + count_bits + byte_len.saturating_mul(8);
        needed_bits <= spec.total_data_codewords * 8
    })
}

struct BitBuf {
    bytes: Vec<u8>,
    bit_len: usize,
}

impl BitBuf {
    const fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_len: 0,
        }
    }

    fn push_bits(&mut self, value: usize, width: usize) {
        for shift in (0..width).rev() {
            let bit = ((value >> shift) & 1) == 1;
            if self.bit_len.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if bit {
                let idx = self.bytes.len() - 1;
                let offset = 7 - (self.bit_len % 8);
                self.bytes[idx] |= 1_u8 << offset;
            }
            self.bit_len += 1;
        }
    }
}

fn build_data_codewords(spec: VersionSpec, data: &[u8]) -> Option<Vec<u8>> {
    let total_bits = spec.total_data_codewords * 8;
    let count_bits = if spec.version <= 9 { 8 } else { 16 };
    let mut bits = BitBuf::new();
    // Mode indicator: Byte = 0b0100.
    bits.push_bits(0b0100, 4);
    bits.push_bits(data.len(), count_bits);
    for &b in data {
        bits.push_bits(usize::from(b), 8);
    }
    if bits.bit_len > total_bits {
        return None;
    }
    let term_bits = (total_bits - bits.bit_len).min(4);
    bits.push_bits(0, term_bits);
    while !bits.bit_len.is_multiple_of(8) {
        bits.push_bits(0, 1);
    }
    let mut pad_toggle = false;
    while bits.bytes.len() < spec.total_data_codewords {
        bits.bytes.push(if pad_toggle { 0x11 } else { 0xEC });
        pad_toggle = !pad_toggle;
    }
    Some(bits.bytes)
}

struct GfTables {
    exp: [u8; 256],
    log: [u8; 256],
}

/// GF(256) log / exp tables with primitive polynomial `x^8 + x^4 + x^3 + x^2 + 1` (`0x11D`).
fn gf_tables() -> GfTables {
    let mut exp = [0_u8; 256];
    let mut log = [0_u8; 256];
    let mut v = 1_u8;
    for i in 0_u8..255 {
        exp[usize::from(i)] = v;
        log[usize::from(v)] = i;
        let carry = (v & 0x80) != 0;
        v <<= 1;
        if carry {
            v ^= 0x1D;
        }
    }
    exp[255] = exp[0];
    GfTables { exp, log }
}

fn gf_mul(gf: &GfTables, a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let idx = (usize::from(gf.log[usize::from(a)]) + usize::from(gf.log[usize::from(b)])) % 255;
    gf.exp[idx]
}

fn rs_generator(gf: &GfTables, degree: usize) -> Vec<u8> {
    let mut poly = vec![1_u8];
    for i in 0..degree {
        let root = gf.exp[i % 255];
        let mut next = vec![0_u8; poly.len() + 1];
        for (j, &coeff) in poly.iter().enumerate() {
            next[j] ^= coeff;
            next[j + 1] ^= gf_mul(gf, coeff, root);
        }
        poly = next;
    }
    poly
}

fn rs_remainder(gf: &GfTables, data: &[u8], ec_len: usize) -> Vec<u8> {
    let gen_poly = rs_generator(gf, ec_len);
    let mut rem = vec![0_u8; ec_len];
    for &byte in data {
        let factor = byte ^ rem[0];
        rem.copy_within(1.., 0);
        rem[ec_len - 1] = 0;
        for (slot, &g) in rem.iter_mut().zip(&gen_poly[1..]) {
            *slot ^= gf_mul(gf, g, factor);
        }
    }
    rem
}

fn interleave_with_ecc(spec: VersionSpec, data: &[u8]) -> Vec<u8> {
    let gf = gf_tables();
    let total_blocks = spec.g1_blocks + spec.g2_blocks;
    let mut data_blocks: Vec<&[u8]> = Vec::with_capacity(total_blocks);
    let mut ec_blocks: Vec<Vec<u8>> = Vec::with_capacity(total_blocks);
    let mut offset = 0;

    for _ in 0..spec.g1_blocks {
        let slice = &data[offset..offset + spec.g1_data];
        offset += spec.g1_data;
        ec_blocks.push(rs_remainder(&gf, slice, spec.ec_per_block));
        data_blocks.push(slice);
    }
    for _ in 0..spec.g2_blocks {
        let slice = &data[offset..offset + spec.g2_data];
        offset += spec.g2_data;
        ec_blocks.push(rs_remainder(&gf, slice, spec.ec_per_block));
        data_blocks.push(slice);
    }

    let total_cw = spec.total_data_codewords + total_blocks * spec.ec_per_block;
    let mut out = Vec::with_capacity(total_cw);
    let max_data = spec.g1_data.max(spec.g2_data);

    for col in 0..max_data {
        for blk in &data_blocks {
            if let Some(&b) = blk.get(col) {
                out.push(b);
            }
        }
    }
    for col in 0..spec.ec_per_block {
        for ec in &ec_blocks {
            out.push(ec[col]);
        }
    }
    out
}

fn set_fn(
    size: usize,
    fixed: &mut [bool],
    modules: &mut [bool],
    row: usize,
    col: usize,
    dark: bool,
) {
    let idx = row * size + col;
    modules[idx] = dark;
    fixed[idx] = true;
}

fn draw_finder(size: usize, fixed: &mut [bool], modules: &mut [bool], top: usize, left: usize) {
    for dr in 0..9 {
        let Some(r) = (top + dr).checked_sub(1) else {
            continue;
        };
        if r >= size {
            continue;
        }
        for dc in 0..9 {
            let Some(c) = (left + dc).checked_sub(1) else {
                continue;
            };
            if c >= size {
                continue;
            }
            let in_outer = (1..=7).contains(&dr) && (1..=7).contains(&dc);
            let dark = in_outer
                && (dr == 1
                    || dr == 7
                    || dc == 1
                    || dc == 7
                    || ((3..=5).contains(&dr) && (3..=5).contains(&dc)));
            set_fn(size, fixed, modules, r, c, dark);
        }
    }
}

fn draw_alignment(size: usize, fixed: &mut [bool], modules: &mut [bool], cr: usize, cc: usize) {
    for dr in 0..5 {
        for dc in 0..5 {
            let r = cr - 2 + dr;
            let c = cc - 2 + dc;
            if fixed[r * size + c] {
                return;
            }
        }
    }
    for dr in 0..5 {
        for dc in 0..5 {
            let dark = dr == 0 || dr == 4 || dc == 0 || dc == 4 || (dr == 2 && dc == 2);
            set_fn(size, fixed, modules, cr - 2 + dr, cc - 2 + dc, dark);
        }
    }
}

fn place_function_patterns(spec: VersionSpec, modules: &mut [bool], fixed: &mut [bool]) {
    let n = spec.size;
    draw_finder(n, fixed, modules, 0, 0);
    draw_finder(n, fixed, modules, 0, n - 7);
    draw_finder(n, fixed, modules, n - 7, 0);

    // Alignment patterns (skipping coordinates overlapping finder patterns).
    for &r in spec.alignments {
        for &c in spec.alignments {
            draw_alignment(n, fixed, modules, r, c);
        }
    }

    // Timing patterns (row 6 and col 6).
    for i in 8..n - 8 {
        let dark = i.is_multiple_of(2);
        if !fixed[6 * n + i] {
            set_fn(n, fixed, modules, 6, i, dark);
        }
        if !fixed[i * n + 6] {
            set_fn(n, fixed, modules, i, 6, dark);
        }
    }

    // Dark module.
    set_fn(n, fixed, modules, n - 8, 8, true);

    // Reserve format information areas so data bits skip them.
    for i in 0..9 {
        if i != 6 {
            fixed[8 * n + i] = true;
            fixed[i * n + 8] = true;
        }
    }
    for i in 0..8 {
        fixed[8 * n + (n - 8 + i)] = true;
        if i < 7 {
            fixed[(n - 1 - i) * n + 8] = true;
        }
    }

    // Reserve version information areas for Versions 7+.
    if spec.version >= 7 {
        for r in 0..6 {
            for c in 0..3 {
                fixed[r * n + (n - 11 + c)] = true;
                fixed[(n - 11 + c) * n + r] = true;
            }
        }
    }
}

fn place_data_bits(size: usize, codewords: &[u8], fixed: &[bool], modules: &mut [bool]) {
    let total_bits = codewords.len() * 8;
    let mut bit_idx = 0;
    let mut up = true;
    let mut right = size - 1;

    loop {
        if right == 6 {
            right -= 1;
        }
        for step in 0..size {
            let row = if up { size - 1 - step } else { step };
            for offset in 0..=1 {
                let col = right - offset;
                let idx = row * size + col;
                if fixed[idx] {
                    continue;
                }
                let dark = if bit_idx < total_bits {
                    let byte = codewords[bit_idx / 8];
                    let shift = 7 - (bit_idx % 8);
                    ((byte >> shift) & 1) == 1
                } else {
                    false
                };
                modules[idx] = dark;
                bit_idx += 1;
            }
        }
        up = !up;
        let Some(next) = right.checked_sub(2) else {
            break;
        };
        if next == 0 {
            break;
        }
        right = next;
    }
}

const fn mask_hit(mask: u8, r: usize, c: usize) -> bool {
    match mask {
        0 => (r + c).is_multiple_of(2),
        1 => r.is_multiple_of(2),
        2 => c.is_multiple_of(3),
        3 => (r + c).is_multiple_of(3),
        4 => ((r / 2) + (c / 3)).is_multiple_of(2),
        5 => (r * c) % 2 + (r * c) % 3 == 0,
        6 => ((r * c) % 2 + (r * c) % 3).is_multiple_of(2),
        _ => ((r + c) % 2 + (r * c) % 3).is_multiple_of(2),
    }
}

fn apply_mask(size: usize, mask: u8, fixed: &[bool], modules: &mut [bool]) {
    for r in 0..size {
        for c in 0..size {
            let idx = r * size + c;
            if !fixed[idx] && mask_hit(mask, r, c) {
                modules[idx] = !modules[idx];
            }
        }
    }
}

fn format_bits_level_m(mask: u8) -> u16 {
    // Error correction Level M = 0b00, combined 5-bit value = mask.
    let data = u16::from(mask & 0x07) << 10;
    let mut rem = data;
    let gen_poly: u16 = 0x537;
    let mut bit = 14_u32;
    while bit >= 10 {
        if (rem >> bit) & 1 == 1 {
            rem ^= gen_poly << (bit - 10);
        }
        bit -= 1;
    }
    (data | rem) ^ 0x5412
}

fn write_format_info(size: usize, mask: u8, modules: &mut [bool]) {
    let bits = format_bits_level_m(mask);
    let bit = |pos: u8| -> bool { ((bits >> pos) & 1) == 1 };

    // Top-left strip.
    for i in 0..=5 {
        modules[8 * size + i] = bit(14 - u8::try_from(i).unwrap_or(0));
    }
    modules[8 * size + 7] = bit(8);
    modules[8 * size + 8] = bit(7);
    modules[7 * size + 8] = bit(6);
    for i in 0..6 {
        modules[(5 - i) * size + 8] = bit(u8::try_from(i).unwrap_or(0));
    }

    // Bottom-left & top-right mirrors.
    for i in 0..7 {
        modules[(size - 1 - i) * size + 8] = bit(14 - u8::try_from(i).unwrap_or(0));
    }
    for i in 0..8 {
        modules[8 * size + (size - 8 + i)] = bit(7 - u8::try_from(i).unwrap_or(0));
    }
}

fn version_bits(version: u8) -> u32 {
    let data = u32::from(version) << 12;
    let mut rem = data;
    let gen_poly: u32 = 0x1F25;
    let mut bit = 17_u32;
    while bit >= 12 {
        if (rem >> bit) & 1 == 1 {
            rem ^= gen_poly << (bit - 12);
        }
        bit -= 1;
    }
    data | rem
}

fn write_version_info(size: usize, version: u8, modules: &mut [bool]) {
    let bits = version_bits(version);
    for k in 0..18 {
        let dark = ((bits >> k) & 1) == 1;
        let row = k / 3;
        let col = k % 3;
        modules[row * size + (size - 11 + col)] = dark;
        modules[(size - 11 + col) * size + row] = dark;
    }
}

fn evaluate_penalty(size: usize, modules: &[bool]) -> u32 {
    let mut score = 0_u32;
    let at = |r: usize, c: usize| modules[r * size + c];

    // Rule N1 & N3 across rows and columns.
    for line in 0..size {
        score += line_penalty(size, |i| at(line, i));
        score += line_penalty(size, |i| at(i, line));
    }

    // Rule N2: 2×2 blocks of identical modules.
    for r in 0..size - 1 {
        for c in 0..size - 1 {
            let first = at(r, c);
            if at(r, c + 1) == first && at(r + 1, c) == first && at(r + 1, c + 1) == first {
                score += 3;
            }
        }
    }

    // Rule N4: balance of dark vs light modules.
    let total = u32::try_from(modules.len()).unwrap_or(1);
    let dark = u32::try_from(modules.iter().filter(|&&m| m).count()).unwrap_or(0);
    let pct = (dark * 100) / total;
    let prev5 = (pct / 5) * 5;
    let next5 = prev5 + 5;
    let dev = prev5.abs_diff(50).min(next5.abs_diff(50));
    score += (dev / 5) * 10;

    score
}

fn line_penalty(size: usize, mut get: impl FnMut(usize) -> bool) -> u32 {
    let mut score = 0_u32;
    let mut run_color = false;
    let mut run_len = 0_u32;
    let mut hist = 0_u32;

    for i in 0..size {
        let cell = get(i);
        if i == 0 || cell != run_color {
            if run_len >= 5 {
                score += 3 + (run_len - 5);
            }
            run_color = cell;
            run_len = 1;
        } else {
            run_len += 1;
        }

        hist = ((hist << 1) | u32::from(cell)) & 0x7FF;
        if i >= 10 && (hist == 0b101_1101_0000 || hist == 0b000_0101_1101) {
            score += 40;
        }
    }
    if run_len >= 5 {
        score += 3 + (run_len - 5);
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_across_versions_and_respects_capacity() {
        let short = encode(b"https://baylee.example/invite/ABCD-1234").expect("v2/v3 invite");
        assert!(short.size >= 25);
        assert_eq!(short.modules.len(), short.size * short.size);

        // Version 1-M holds up to 14 bytes; Version 10-M holds up to 213 bytes.
        let v1 = encode(b"12345678901234").expect("14 bytes fits v1-M");
        assert_eq!(v1.version, 1);
        assert_eq!(v1.size, 21);

        let v10 = encode(&vec![b'x'; 213]).expect("213 bytes fits v10-M");
        assert_eq!(v10.version, 10);
        assert_eq!(v10.size, 57);

        assert!(
            encode(&vec![b'x'; 214]).is_none(),
            "over Version 10-M limit is rejected"
        );
    }

    #[test]
    fn finder_dark_module_and_format_vectors_match_iso_spec() {
        assert_eq!(format_bits_level_m(0), 0x5412);
        assert_eq!(format_bits_level_m(5), 0x40CE);
        assert_eq!(version_bits(7), 0x07C94);

        let m = encode(b"hello world").expect("v1");
        assert_eq!(m.version, 1);
        // Top-left finder top row (0, 0..7) is dark, separator (0, 7) is light.
        for c in 0..7 {
            assert!(m.get(0, c), "finder col {c}");
        }
        assert!(!m.get(0, 7));
        // Required fixed dark module at (4*version + 9, 8) = (13, 8).
        assert!(m.get(m.size - 8, 8));
        // Determinism check.
        assert_eq!(encode(b"hello world").expect("v1"), m);
    }

    #[test]
    fn svg_path_is_bounded_and_valid() {
        let m = encode(b"https://central.baylee.gg/invite/test").expect("qr");
        let path = m.svg_path();
        assert!(
            path.starts_with("M4,4h7v1h-7z"),
            "starts with top-left finder bar"
        );
        assert!(path.contains("v1h-"), "contains horizontal run commands");
        assert_eq!(m.svg_size(), m.size + 8);
    }
}
