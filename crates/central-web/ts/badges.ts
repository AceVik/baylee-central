// Ranked badges, avatar frames, procedural trophy insignia, and card finishes
// rendered with a tiered GPU backend:
//   1. WebGPU (WGSL, primary — zero-copy direct canvas swapchains, visual
//      deduplication with copyTextureToTexture, single queue.submit per frame,
//      premultiplied mipmaps with 4x anisotropic 1-tap filtering)
//   2. WebGL2 (GLSL ES 3.00 fallback — deduplicated shelf atlas + cached 2D)
//   3. 2D Canvas (software fallback when no hardware GPU backend is available)
// Plain DOM calls only (CSP & Trusted Types clean).

const SLUGS: readonly string[] = [
  "papier",
  "holz",
  "eisen",
  "bronze",
  "silber",
  "gold",
  "platin",
  "diamant",
  "master",
  "grandmaster",
  "champion",
  "unplatziert",
];

type RgbTuple = readonly [number, number, number];
type TierSpec = readonly [number, RgbTuple, RgbTuple];

// Exact client catalog: [periodSeconds, [metalR, metalG, metalB], [gemR, gemG, gemB]]
const TIERS: readonly TierSpec[] = [
  [0, [172, 154, 119], [234, 215, 169]],
  [0, [144, 88, 49], [214, 149, 79]],
  [10, [121, 134, 146], [186, 208, 221]],
  [9, [186, 116, 68], [85, 154, 138]],
  [8, [188, 204, 222], [145, 202, 237]],
  [8, [233, 169, 66], [255, 222, 130]],
  [9, [196, 224, 222], [90, 221, 192]],
  [7, [157, 209, 245], [67, 183, 252]],
  [10, [219, 171, 99], [178, 112, 248]],
  [9, [224, 158, 131], [236, 68, 95]],
  [12, [242, 203, 124], [217, 234, 243]],
  [0, [120, 133, 143], [155, 166, 173]],
];

const PODIUM: Record<number, readonly [RgbTuple, RgbTuple]> = {
  1: [[242, 203, 124], [211, 248, 255]],
  2: [[219, 229, 240], [132, 201, 255]],
  3: [[218, 155, 116], [249, 166, 171]],
};

const INSIGNIA_TIER_RGB: readonly RgbTuple[] = [
  [0.35, 0.39, 0.42],
  [0.79, 0.48, 0.26],
  [0.77, 0.84, 0.9],
  [0.96, 0.74, 0.32],
  [0.78, 0.94, 0.96],
  [0.6, 0.5, 0.86],
];

const INSIGNIA_MOTIFS: Record<string, readonly [number, RgbTuple]> = {
  daily: [0, [1.0, 0.68, 0.25]],
  weekly: [1, [0.48, 0.73, 1.0]],
  tactician: [2, [0.84, 0.5, 0.9]],
  first_try: [3, [0.98, 0.48, 0.33]],
  regular: [4, [0.36, 0.86, 0.62]],
  tower_pathfinder: [5, [0.4, 0.86, 0.76]],
  tower_summit: [6, [0.96, 0.72, 0.28]],
  tower_unbroken: [7, [0.64, 0.58, 0.98]],
  echoes_victories: [8, [0.96, 0.38, 0.29]],
  echoes_ascents: [9, [0.32, 0.86, 0.77]],
  echoes_unbroken: [10, [0.72, 0.58, 1.0]],
  recruiter: [11, [0.24, 0.88, 0.72]],
};

const ROMAN: readonly string[] = ["", "i", "ii", "iii", "iv"];
const MAX_SLOTS = 128;
const UNIFORM_STRIDE_BYTES = 256;
const UNIFORM_STRIDE_FLOATS = 64;
const ATLAS_WIDTH = 1024;
const ATLAS_MAX_HEIGHT = 2048;
const MAX_SCRATCH_TEXTURES = 16;

function rankUrls(): Record<string, string> {
  const meta = document.querySelector('meta[name="baylee-ranks"]');
  if (!meta) return {};
  try {
    const parsed = JSON.parse(meta.getAttribute("content") || "{}") as unknown;
    return typeof parsed === "object" && parsed !== null ? (parsed as Record<string, string>) : {};
  } catch {
    return {};
  }
}

export function assetKey(
  kind: string | undefined,
  tier: string | number | undefined,
  div: string | number | undefined,
  podium: string | number | undefined,
  track: string | number | undefined,
): string | null {
  const t = Math.max(0, Math.min(11, Number(tier) || 0));
  if (t >= 11) return null;
  const slug = SLUGS[t];
  const d = Math.max(0, Math.min(4, Number(div) || 0));
  const p = Math.max(0, Math.min(3, Number(podium) || 0));
  const stem =
    t === 10 && p >= 1 && p <= 3 && Number(track) !== 1
      ? `champion-${p}`
      : d >= 1 && d <= 4 && ROMAN[d]
        ? `${slug}-${ROMAN[d]}`
        : slug;
  if (kind === "frame") return `frames/player/${stem}.png`;
  return Number(track) === 1 ? `deck/${stem}.png` : `player/${stem}.png`;
}

// ============================================================================
// WGSL Shaders (WebGPU Primary Backend)
// Uniform control flow: keep textureSample / fwidth outside non-uniform ifs.
// Fast ALU gamma (x*x / sqrt) + squaring powers instead of pow().
// ============================================================================

const COMMON_WGSL = /* wgsl */ `
struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
  var p = vec2<f32>(-1.0, 3.0);
  if (vid == 0u) {
    p = vec2<f32>(-1.0, -1.0);
  } else if (vid == 1u) {
    p = vec2<f32>(3.0, -1.0);
  }
  var out: VsOut;
  out.pos = vec4<f32>(p, 0.0, 1.0);
  out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
  return out;
}

struct SlotUniforms {
  v0: vec4<f32>,
  v1: vec4<f32>,
  v2: vec4<f32>,
  v3: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: SlotUniforms;

fn fastToLinear(c: vec3<f32>) -> vec3<f32> {
  let cl = max(c, vec3<f32>(0.0));
  return cl * cl;
}

fn fastToSrgb(c: vec3<f32>) -> vec3<f32> {
  return sqrt(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)));
}

fn pow6(x: f32) -> f32 {
  let x2 = x * x;
  return x2 * x2 * x2;
}

fn pow8(x: f32) -> f32 {
  let x2 = x * x;
  let x4 = x2 * x2;
  return x4 * x4;
}

fn hash12(p: vec2<f32>) -> f32 {
  var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
  p3 += dot(p3, vec3<f32>(p3.y, p3.z, p3.x) + vec3<f32>(33.33));
  return fract((p3.x + p3.y) * p3.z);
}
`;

const MIP_WGSL = /* wgsl */ `
struct MipVsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn vs_mip(@builtin(vertex_index) vid: u32) -> MipVsOut {
  var p = vec2<f32>(-1.0, 3.0);
  if (vid == 0u) {
    p = vec2<f32>(-1.0, -1.0);
  } else if (vid == 1u) {
    p = vec2<f32>(3.0, -1.0);
  }
  var out: MipVsOut;
  out.pos = vec4<f32>(p, 0.0, 1.0);
  out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
  return out;
}

@group(0) @binding(0) var srcTex: texture_2d<f32>;
@group(0) @binding(1) var srcSamp: sampler;

@fragment
fn fs_mip(in: MipVsOut) -> @location(0) vec4<f32> {
  return textureSample(srcTex, srcSamp, clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0)));
}
`;

const RANK_WGSL = COMMON_WGSL + /* wgsl */ `
@group(1) @binding(0) var uTex: texture_2d<f32>;
@group(1) @binding(1) var uSamp: sampler;

fn pFill(d: f32, aa: f32) -> f32 {
  return 1.0 - smoothstep(-aa, aa, d);
}

fn pRim(d: f32, width: f32, aa: f32) -> f32 {
  return pFill(abs(d) - width, aa);
}

fn roundedPlate(p: vec2<f32>, sides: vec2<f32>) -> f32 {
  let v = abs(p) - sides;
  return length(max(v, vec2<f32>(0.0))) + min(max(v.x, v.y), 0.0) - 0.05;
}

fn neutralFrame(uv: vec2<f32>, deck: bool, metal: vec3<f32>, rDiff: f32) -> vec4<f32> {
  let p = 2.0 * uv - 1.0;
  let squared = p * p;
  let r = select(length(p), sqrt(sqrt(dot(squared, squared))), deck);
  let aa = max(rDiff * 0.55, 0.001);
  let col = metal * (0.65 + 0.55 * pRim(r - 0.947, 0.005, aa));
  let alpha = pRim(r - 0.952, 0.023, aa);
  return vec4<f32>(col, alpha);
}

fn unrankedBadge(uv: vec2<f32>, deck: bool, metal: vec3<f32>, aaP: f32) -> vec4<f32> {
  let p = (uv - 0.5) * 2.0;
  let aa = max(aaP * 0.6, 0.001);
  var coverage = 0.0;
  var bevel = 0.0;
  if (deck) {
    let q = p + vec2<f32>(0.20, 0.04);
    let back = roundedPlate(vec2<f32>(q.x * 0.95 - q.y * 0.31, q.x * 0.31 + q.y * 0.95), vec2<f32>(0.33, 0.52));
    let front = roundedPlate(p - vec2<f32>(0.12, 0.04), vec2<f32>(0.34, 0.51));
    coverage = max(pFill(back, aa), pFill(front, aa));
    bevel = max(pRim(back + 0.052, 0.026, aa) * (1.0 - pFill(front, aa)), pRim(front + 0.062, 0.029, aa));
  } else {
    let crown = length((p + vec2<f32>(0.0, 0.12)) / vec2<f32>(0.56, 0.60)) - 1.0;
    let cheek = abs(p.x) - (0.54 - 0.39 * clamp(p.y + 0.08, 0.0, 1.0));
    let edge = max(cheek, max(-p.y - 0.18, p.y - 0.66 + abs(p.x) * 0.45));
    let shape = min(crown * 0.54, edge);
    let visor = max(abs(p.y + 0.04 + abs(p.x) * 0.12) - 0.084, abs(p.x) - 0.50);
    let seam = max(abs(p.x) - 0.044, max(-p.y - 0.04, p.y - 0.55));
    coverage = pFill(max(shape, -min(visor, seam)), aa);
    bevel = pRim(shape + 0.045, 0.025, aa);
  }
  let highlight = 0.17 * clamp(1.0 - uv.y, 0.0, 1.0);
  return vec4<f32>(metal * (0.58 + highlight + 0.63 * bevel), coverage);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = in.uv;
  let phase = u.v0.x;
  let tier = u.v0.z;
  let div = u.v0.w;
  let podium = u.v1.x;
  let kind = u.v1.y;
  let track = u.v1.z;
  let hasTex = u.v1.w;
  let uMetal = u.v2.xyz;
  let uGem = u.v3.xyz;

  let pUnr = 2.0 * uv - 1.0;
  let sq = pUnr * pUnr;
  let deck = track > 0.5;
  let rUnr = select(length(pUnr), sqrt(sqrt(dot(sq, sq))), deck);
  let rDiff = fwidth(rUnr);
  let aaP = fwidth(pUnr.x);

  // Single hardware-filtered + 4x anisotropic fetch on premultiplied mipmap chain
  let base = textureSample(uTex, uSamp, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)));

  if (tier >= 10.5 || hasTex < 0.5) {
    let linMetal = fastToLinear(uMetal);
    let res = select(
      unrankedBadge(uv, deck, linMetal, aaP),
      neutralFrame(uv, deck, linMetal, rDiff),
      kind > 0.5
    );
    let outRgb = fastToSrgb(res.rgb);
    return vec4<f32>(outRgb * res.a, res.a);
  }

  if (base.a <= 0.002) {
    return vec4<f32>(0.0);
  }

  let straight = base.rgb / max(base.a, 0.001);
  var rgb = fastToLinear(straight);
  let luma = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
  let relief = smoothstep(0.035, 0.30, luma);
  let cx = uv.x - 0.5;
  let curve = select(
    0.78 * uv.x + 0.26 * uv.y + 0.70 * cx * cx,
    abs(cx) * 0.85 + uv.y * 0.28,
    deck
  );
  let sw = (fract(curve * 0.65 - phase) - 0.5) * 6.5;
  let sweep = exp(-(sw * sw));
  let hl = (fract(curve * 0.65 - phase - 0.105) - 0.5) * 14.0;
  let hairline = exp(-(hl * hl));
  let boxMask = select(1.0, 0.15, kind < 0.5 && abs(cx) < 0.112 && uv.y > 0.79 && uv.y < 0.907);

  let metal = fastToLinear(uMetal);
  let gem = fastToLinear(uGem);
  var add = vec3<f32>(0.0);
  let t = i32(floor(tier + 0.5));

  if (t == 0) {
    add += metal * (0.14 * smoothstep(0.2, 0.9, uv.x));
  } else if (t == 1) {
    let grain = 0.5 + 0.5 * cos(uv.y * 42.0 + sin(uv.x * 8.0) * 1.6);
    add += metal * (0.18 * grain);
  } else if (t == 2) {
    let brush = 0.75 + 0.25 * cos(uv.y * 81.0 + uv.x * 3.0);
    add += mix(metal, gem, 0.5) * sweep * 0.48 * brush;
  } else if (t == 3) {
    add += metal * sweep * 0.62 + gem * (1.0 - relief) * 0.22;
  } else if (t == 4) {
    add += gem * hairline * 0.72 + metal * sweep * 0.35;
  } else if (t == 5) {
    let rays = 0.7 + 0.3 * cos(atan2(uv.y - 0.48, cx) * 10.0 + phase * 6.28318);
    add += mix(metal, gem, 0.45) * sweep * 0.70 * rays;
  } else if (t == 6) {
    let en = (fract(curve * 0.65 - phase - 0.17) - 0.5) * 8.0;
    let enamel = exp(-(en * en));
    add += metal * sweep * 0.55 + gem * enamel * 0.65;
  } else if (t == 7) {
    let cell = floor(uv * 14.0);
    let offset = hash12(cell);
    let fs = 0.5 + 0.5 * sin((phase + offset) * 6.28318);
    let fs2 = fs * fs;
    let fs8 = pow8(fs);
    let facet = fs8 * fs2;
    add += gem * facet * 0.85 * relief + metal * sweep * 0.45;
  } else if (t == 8) {
    let dCore = length(uv - vec2<f32>(0.5, 0.48));
    let pulse = (0.5 + 0.5 * sin(phase * 6.28318)) * exp(-dCore * 5.0);
    add += gem * pulse * 0.85 + metal * hairline * 0.65;
  } else if (t == 9) {
    let a1 = (fract(uv.x * 0.7 + uv.y * 0.4 - phase) - 0.5) * 9.0;
    let a2 = (fract(uv.x * 0.5 - uv.y * 0.5 - phase - 0.25) - 0.5) * 9.0;
    add += gem * (exp(-(a1 * a1)) + exp(-(a2 * a2))) * 0.58 + metal * sweep * 0.48;
  } else if (t >= 10) {
    let nacre = uv.x * 3.0 + uv.y * 2.0 + phase * 6.28318;
    let prism = 0.5 + 0.5 * cos(vec3<f32>(0.0, 2.1, 4.2) + vec3<f32>(nacre));
    add += mix(prism, metal, 0.35) * sweep * 0.78 + gem * hairline * 0.5;
    if (podium > 0.5) {
      let crown = exp(-length(uv - vec2<f32>(0.5, 0.12)) * 12.0);
      let twinkle = pow8(0.5 + 0.5 * cos(phase * 12.56636));
      add += vec3<f32>(0.95, 0.98, 1.0) * crown * twinkle * 1.4;
    }
  }

  if (kind > 0.5 && t >= 2) {
    let diag = (fract(uv.x * 0.63 + uv.y * 0.37 - phase) - 0.5) * 11.0;
    let eng = exp(-(diag * diag));
    add += mix(metal, gem, 0.35) * eng * (0.28 + 0.03 * f32(t));
    if (t >= 6) {
      let chroma = length(rgb - vec3<f32>(luma));
      let j = pow6(0.5 + 0.5 * cos(uv.x * 16.0 - uv.y * 12.0 + phase * 6.28318));
      add += gem * chroma * j * 1.1;
    }
  } else if (div > 0.5 && kind < 0.5) {
    let clasp = exp(-length(uv - vec2<f32>(0.5, 0.85)) * 7.5);
    add += metal * clasp * sweep * (0.18 * div);
  }

  rgb += add * relief * boxMask;
  let outSrgb = fastToSrgb(rgb);
  return vec4<f32>(outSrgb * base.a, base.a);
}
`;

const INSIGNIA_WGSL = COMMON_WGSL + /* wgsl */ `
const PI: f32 = 3.14159265;
const TAU: f32 = 6.28318530;

fn iStroke(d: f32, width: f32, aa: f32) -> f32 {
  return 1.0 - smoothstep(width - aa, width + aa, abs(d));
}

fn iFill(d: f32, aa: f32) -> f32 {
  return 1.0 - smoothstep(-aa, aa, d);
}

fn iSegment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
  let q = p - a;
  let v = b - a;
  return length(q - v * clamp(dot(q, v) / dot(v, v), 0.0, 1.0));
}

fn iDiamond(p: vec2<f32>, size: vec2<f32>) -> f32 {
  return (abs(p.x) / size.x + abs(p.y) / size.y - 1.0) * min(size.x, size.y);
}

fn metalLight(p: vec2<f32>, r: f32, centre: f32, width: f32, metal: vec3<f32>) -> vec3<f32> {
  let slope = clamp((r - centre) / width, -0.96, 0.96);
  let radial = p / max(r, 0.001);
  let n = vec3<f32>(radial * slope, sqrt(1.0 - slope * slope));
  let key = normalize(vec3<f32>(-0.45, -0.65, 0.70));
  let diffuse = max(dot(n, key), 0.0);
  let refDot = max(dot(n, normalize(key + vec3<f32>(0.0, 0.0, 1.0))), 0.0);
  let r2 = refDot * refDot;
  let r4 = r2 * r2;
  let reflection = r4 * r4 * r4 * r4 * r4 * r2;
  let brushed = 0.97 + 0.03 * sin(r * 720.0);
  return metal * (0.18 + 0.92 * diffuse) * brushed
    + mix(metal, vec3<f32>(1.0), 0.45) * reflection * 0.70;
}

fn emblem(p: vec2<f32>, kind: f32, aa: f32) -> f32 {
  let r = length(p);
  let a = atan2(p.y, p.x);
  if (kind < 0.5) {
    let sun = iFill(r - 0.135, aa);
    let rays = iStroke(sin(a * 6.0), 0.23, aa * 6.0) * iFill(r - 0.255, aa) * iFill(0.176 - r, aa);
    return max(sun, rays);
  }
  if (kind < 1.5) {
    let moon = min(
      iFill(length(p + vec2<f32>(0.04, 0.0)) - 0.21, aa),
      1.0 - iFill(length(p - vec2<f32>(0.055, -0.045)) - 0.18, aa)
    );
    let star = iFill(iDiamond(p - vec2<f32>(0.15, -0.12), vec2<f32>(0.07, 0.105)), aa);
    return max(moon, star);
  }
  if (kind < 2.5) {
    let arms = max(iFill(iDiamond(p, vec2<f32>(0.065, 0.26)), aa), iFill(iDiamond(p, vec2<f32>(0.22, 0.065)), aa));
    return max(arms, iStroke(iDiamond(p, vec2<f32>(0.17, 0.17)), 0.012, aa));
  }
  if (kind < 3.5) {
    let ring = iStroke(length(p - vec2<f32>(0.02, -0.01)) - 0.16, 0.016, aa);
    let shaft = iStroke(iSegment(p, vec2<f32>(-0.19, 0.22), vec2<f32>(0.14, -0.17)), 0.018, aa);
    let head = max(
      iStroke(iSegment(p, vec2<f32>(0.14, -0.17), vec2<f32>(0.02, -0.16)), 0.018, aa),
      iStroke(iSegment(p, vec2<f32>(0.14, -0.17), vec2<f32>(0.15, -0.05)), 0.018, aa)
    );
    return max(ring, max(shaft, head));
  }
  if (kind > 10.5) {
    let leftNode = length(p - vec2<f32>(-0.17, 0.04));
    let rightNode = length(p - vec2<f32>(0.17, 0.04));
    let nodes = max(iStroke(leftNode - 0.075, 0.017, aa), iStroke(rightNode - 0.075, 0.017, aa));
    let cores = max(iFill(leftNode - 0.032, aa), iFill(rightNode - 0.032, aa));
    let bridge = max(
      iStroke(iSegment(p, vec2<f32>(-0.10, 0.02), vec2<f32>(0.0, -0.14)), 0.017, aa),
      iStroke(iSegment(p, vec2<f32>(0.10, 0.02), vec2<f32>(0.0, -0.14)), 0.017, aa)
    );
    let base = iStroke(iSegment(p, vec2<f32>(-0.085, 0.045), vec2<f32>(0.085, 0.045)), 0.014, aa);
    let beacon = max(
      iFill(iDiamond(p - vec2<f32>(0.0, -0.16), vec2<f32>(0.065, 0.095)), aa),
      iFill(iDiamond(p - vec2<f32>(0.0, -0.16), vec2<f32>(0.095, 0.035)), aa)
    );
    return max(max(nodes, cores), max(max(bridge, base), beacon));
  }
  if (kind > 9.5) {
    let shield = max(abs(p.x) - 0.14, max(-p.y - 0.19, p.y + abs(p.x) * 0.85 - 0.23));
    let star = max(iFill(iDiamond(p, vec2<f32>(0.04, 0.13)), aa), iFill(iDiamond(p, vec2<f32>(0.09, 0.04)), aa));
    let wing = max(
      iStroke(iSegment(vec2<f32>(abs(p.x), p.y), vec2<f32>(0.16, 0.10), vec2<f32>(0.26, -0.15)), 0.018, aa),
      iStroke(iSegment(vec2<f32>(abs(p.x), p.y), vec2<f32>(0.18, 0.02), vec2<f32>(0.28, -0.02)), 0.016, aa)
    );
    return max(iStroke(shield, 0.018, aa), max(star, wing));
  }
  if (kind > 8.5) {
    let peak = iStroke(iDiamond(p - vec2<f32>(0.0, 0.07), vec2<f32>(0.16, 0.19)), 0.020, aa);
    let facets = max(iStroke(p.x, 0.012, aa), iStroke(p.y - 0.07, 0.012, aa))
      * iFill(iDiamond(p - vec2<f32>(0.0, 0.07), vec2<f32>(0.16, 0.19)), aa);
    let crown = iFill(max(abs(p.x) - 0.22, max(p.y + 0.13, -p.y - 0.20 - 0.045 * cos(p.x * 42.0))), aa);
    return max(crown, max(peak, facets));
  }
  if (kind > 7.5) {
    let q = vec2<f32>(abs(p.x), p.y);
    let blade = iStroke(iSegment(q, vec2<f32>(0.0, 0.0), vec2<f32>(0.20, -0.23)), 0.026, aa);
    let grip = iStroke(iSegment(q, vec2<f32>(0.0, 0.0), vec2<f32>(0.19, 0.20)), 0.016, aa);
    let guard = iStroke(iSegment(q, vec2<f32>(0.045, 0.19), vec2<f32>(0.20, 0.045)), 0.022, aa);
    return max(blade, max(grip, guard));
  }
  if (kind > 6.5) {
    let shield = max(abs(p.x) - 0.21, max(-p.y - 0.22, p.y + abs(p.x) * 0.72 - 0.25));
    let rim = iStroke(shield, 0.019, aa);
    let knot = iStroke(iDiamond(p, vec2<f32>(0.10, 0.16)), 0.019, aa);
    return max(rim, knot);
  }
  if (kind > 5.5) {
    let tower = iFill(max(abs(p.x) - 0.12, abs(p.y - 0.055) - 0.18), aa);
    let door = iFill(max(abs(p.x) - 0.045, abs(p.y - 0.15) - 0.08), aa);
    let crown = iFill(max(abs(p.x) - 0.23, max(p.y + 0.11, -p.y - 0.19 - 0.07 * cos(p.x * 28.0))), aa);
    return max(tower * (1.0 - door), crown);
  }
  if (kind > 4.5) {
    let q = vec2<f32>(p.x, p.y + 0.02);
    let peak = max(
      iStroke(iSegment(q, vec2<f32>(-0.23, 0.19), vec2<f32>(0.0, -0.18)), 0.018, aa),
      iStroke(iSegment(q, vec2<f32>(0.0, -0.18), vec2<f32>(0.23, 0.19)), 0.018, aa)
    );
    let path = max(
      iStroke(iSegment(q, vec2<f32>(0.06, 0.21), vec2<f32>(-0.06, 0.08)), 0.017, aa),
      iStroke(iSegment(q, vec2<f32>(-0.06, 0.08), vec2<f32>(0.035, -0.02)), 0.017, aa)
    );
    let star = iFill(iDiamond(p - vec2<f32>(0.18, -0.20), vec2<f32>(0.04, 0.065)), aa);
    return max(peak, max(path, star));
  }
  var tree = iFill(length(p - vec2<f32>(0.0, -0.13)) - 0.13, aa);
  tree = max(tree, iFill(length(p - vec2<f32>(0.12, -0.04)) - 0.105, aa));
  tree = max(tree, iFill(length(p - vec2<f32>(-0.12, -0.04)) - 0.105, aa));
  tree = max(tree, iStroke(iSegment(p, vec2<f32>(0.0, -0.04), vec2<f32>(0.0, 0.2)), 0.025, aa));
  tree = max(tree, iStroke(iSegment(p, vec2<f32>(-0.13, 0.23), vec2<f32>(0.0, 0.15)), 0.014, aa));
  return max(tree, iStroke(iSegment(p, vec2<f32>(0.13, 0.23), vec2<f32>(0.0, 0.15)), 0.014, aa));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uTime = u.v0.x;
  let uEnergy = u.v0.y;
  let tier = u.v0.z;
  let uMotif = u.v2.w;
  let metal = fastToLinear(u.v2.xyz);
  let gem = fastToLinear(u.v3.xyz);

  let p = (in.uv - 0.5) * 2.0;
  let r = length(p);
  let a = atan2(p.y, p.x);
  let aa = max(fwidth(r), 0.003);
  let earned = select(0.0, 1.0, tier > 0.5);
  let t = uTime * uEnergy;

  let disc = iFill(r - 0.53, aa);
  let edge = iStroke(r - 0.51, 0.028, aa);
  let bevel = iStroke(r - 0.465, 0.012, aa);
  let inset = iFill(r - 0.44, aa);
  let engraving = iStroke(r - 0.399, 0.005, aa);
  let light = clamp(0.56 - p.y * 0.42 - p.x * 0.24, 0.18, 1.0);

  var color = metal * light * 0.44;
  color = mix(color, metalLight(p, r, 0.505, 0.027, metal), edge);
  color = mix(color, metalLight(p, r, 0.465, 0.013, metal), bevel);

  let enamel = mix(vec3<f32>(0.006, 0.010, 0.015), gem * 0.035, earned);
  let enamelLight = 0.65 + 0.55 * max(0.0, 1.0 - length(p + vec2<f32>(0.16, 0.20)) * 1.5);
  color = mix(color, enamel * enamelLight, inset);
  color = mix(color, metal * 0.38, engraving);

  let etch = emblem(p, uMotif, aa);
  let relief = emblem(p + vec2<f32>(0.006, 0.009), uMotif, aa);
  color = mix(color, vec3<f32>(0.002, 0.004, 0.006), max(0.0, relief - etch) * 0.8);
  let hiTerm = max(0.0, 1.0 - abs(p.x + p.y * 0.7) * 3.0);
  let raised = metal * (0.48 + light * 0.65)
    + mix(metal, vec3<f32>(1.0), 0.35) * pow6(hiTerm) * 0.24;
  color = mix(color, raised, etch);
  color += mix(metal, vec3<f32>(1.0), 0.5) * max(0.0, etch - relief) * 0.55;

  var alpha = disc;

  if (tier > 1.5) {
    let outer = iStroke(r - 0.585, 0.013, aa);
    let rivets = iStroke(r - 0.56, 0.018, aa) * iStroke(sin(a * 4.0), 0.13, aa * 3.0);
    let detail = max(outer, rivets);
    color = mix(color, metalLight(p, r, 0.585, 0.016, metal), detail);
    alpha = max(alpha, detail);
  }

  if (tier > 2.5) {
    let stem = iStroke(r - 0.70, 0.009, aa) * iFill(-0.62 - p.y, aa) * iFill(p.y - 0.60, aa);
    var leaves = 0.0;
    for (var i = 0; i < 7; i = i + 1) {
      let angle = -1.05 + f32(i) * 0.34;
      let centre = vec2<f32>(cos(angle), sin(angle)) * 0.70;
      let q = vec2<f32>(abs(p.x), p.y) - centre;
      let sAng = sin(angle + 0.7);
      let cAng = cos(angle + 0.7);
      let leaf = vec2<f32>(q.x * cAng + q.y * sAng, -q.x * sAng + q.y * cAng);
      leaves = max(leaves, iFill(iDiamond(leaf, vec2<f32>(0.092, 0.032)), aa));
    }
    let wreath = max(stem, leaves);
    let facets = 0.38 + light * 0.72 + 0.20 * sin(a * 24.0);
    color = mix(color, metal * facets, wreath);
    alpha = max(alpha, wreath);
  }

  if (tier > 3.5) {
    for (var i = 0; i < 5; i = i + 1) {
      let angle = -PI * 0.5 + f32(i) * TAU / 5.0;
      let centre = vec2<f32>(cos(angle), sin(angle)) * 0.59;
      let cut = iDiamond(p - centre, vec2<f32>(0.043, 0.065));
      let socket = iFill(cut - 0.014, aa);
      color = mix(color, metal, socket);
      let facet = select(0.45, 1.25, p.x < centre.x) + select(0.0, 0.25, p.y < centre.y);
      color = mix(color, gem * facet, iFill(cut, aa));
      alpha = max(alpha, socket);
    }
    let pendant = iFill(iDiamond(p - vec2<f32>(0.0, 0.79), vec2<f32>(0.085, 0.105)), aa);
    color = mix(color, gem * select(0.48, 1.20, p.x < 0.0), pendant);
    alpha = max(alpha, pendant);
  }

  if (tier > 4.5) {
    let q = p - vec2<f32>(0.0, -0.72);
    let sx = sin(q.x * 13.65);
    let sx4 = sx * sx * sx * sx;
    let crown = iFill(
      max(abs(q.x) - 0.23, max(q.y - 0.09, -q.y - 0.055 - 0.12 * sx4)),
      aa
    );
    color = mix(color, metal * (0.65 + light), crown);
    alpha = max(alpha, crown);
  }

  let gc = max(0.0, cos(p.x * 3.0 + p.y * 2.0 - t * 0.65));
  let gc8 = pow8(gc);
  let glint = gc8 * gc8 * gc8 * 0.32 * earned * uEnergy;
  color += metal * glint * max(edge, etch);
  let hr = (r - 0.77) * 10.0;
  let halo = exp(-(hr * hr)) * 0.10 * select(0.0, 1.0, tier > 4.5);
  let breath = 0.85 + 0.15 * sin(t * 0.7) * uEnergy;
  color = mix(gem * 0.4 * breath, color, alpha);
  alpha = max(alpha, halo * breath);
  let srgb = fastToSrgb(color * mix(0.50, 1.0, earned));
  return vec4<f32>(srgb * alpha, alpha);
}
`;

const CARD_FINISH_WGSL = COMMON_WGSL + /* wgsl */ `
fn cardCornerSdf(uv: vec2<f32>) -> f32 {
  let q = abs(uv - vec2<f32>(0.5)) - vec2<f32>(0.4525, 0.466);
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - 0.042;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = in.uv;
  let uTime = u.v0.x;
  let uFinish = u.v1.x;
  let uAngle = u.v1.y;

  let diagPhase = (uv.x - uv.y) * 240.0;
  let diagFw = fwidth(diagPhase);

  let sdf = cardCornerSdf(uv);
  let cardMask = 1.0 - smoothstep(-0.004, 0.004, sdf);
  if (cardMask <= 0.001) {
    return vec4<f32>(0.0);
  }
  let ga = 1.0 - clamp(abs(uAngle), 0.0, 1.0);
  let glint = ga * ga;
  let travel = sin(uTime * 0.52 + uAngle * 2.5) * 0.72 + 0.65;
  let diag = uv.x * 0.72 + uv.y;
  let bd = diag - travel;
  let band = exp(-20.0 * bd * bd);
  let counterTravel = sin(-uTime * 0.31 + uAngle * 1.6) * 0.55 + 0.60;
  let cbd = (1.0 - uv.x) * 0.6 + uv.y - counterTravel;
  let counterBand = exp(-26.0 * cbd * cbd);
  let edge = 1.0 - smoothstep(0.003, 0.019, abs(sdf));
  let edgeLight = edge * (0.18 + 0.82 * max(band, counterBand));
  let cornerGlow = exp(-7.5 * length(uv - vec2<f32>(0.18 + uAngle * 0.14, 0.16)));

  var light = vec3<f32>(0.0);
  if (uFinish < 1.5) {
    let ripple = sin(uv.x * 11.0 - uv.y * 8.0 + uTime * 0.35) * 0.12;
    let phase = (uv.x * 0.75 + uv.y * 0.55) + uAngle * 0.92 + uTime * 0.09 + ripple;
    let rainbow = 0.5 + 0.5 * cos(6.283185 * (vec3<f32>(phase) + vec3<f32>(0.0, 0.3333, 0.6667)));
    let sheen = mix(vec3<f32>(0.85, 0.92, 1.0), rainbow, 0.88);
    let sparkCell = floor(uv * vec2<f32>(38.0, 54.0));
    let sparkNoise = hash12(sparkCell);
    let ms = max(0.0, sin(uTime * 1.4 + sparkNoise * 12.0 + uAngle * 2.0));
    let microSpark = step(0.84, sparkNoise) * pow6(ms) * band * 0.42;
    let spec = band * (0.42 + 0.18 * glint)
      + counterBand * 0.16
      + cornerGlow * 0.11 * (0.4 + 0.6 * glint)
      + microSpark;
    light = sheen * spec + mix(rainbow, vec3<f32>(1.0), 0.25) * edgeLight * 0.34;
  } else {
    let grain = (0.5 + 0.5 * sin(diagPhase)) * (1.0 - smoothstep(0.7, 2.5, diagFw));
    let tilt = 0.5 + 0.5 * sin(uTime * 0.42 + uv.y * 4.5 + uAngle * 2.8);
    let warmGold = vec3<f32>(0.98, 0.78, 0.40);
    let coolSteel = vec3<f32>(0.42, 0.70, 0.86);
    let metal = mix(coolSteel, warmGold, tilt);
    let fSin = 0.5 + 0.5 * sin(uv.y * 26.0 - uv.x * 11.0 - uTime * 0.65 + uAngle * 1.4);
    let facet = pow6(fSin) * fSin;
    let bodySpec = (band * 0.34 + facet * 0.18 + cornerGlow * 0.08) * (0.55 + 0.45 * grain);
    let bevel = 1.0 - smoothstep(0.024, 0.044, -sdf);
    light = metal * bodySpec
      + warmGold * edgeLight * 0.58
      + mix(warmGold, coolSteel, 0.35) * bevel * (0.12 + 0.28 * band);
  }

  return vec4<f32>(clamp(light, vec3<f32>(0.0), vec3<f32>(1.0)) * cardMask, cardMask);
}
`;

// ============================================================================
// GLSL ES 3.00 Shaders (WebGL2 Fallback Backend)
// ============================================================================

const VERT = `#version 300 es
precision highp float;
const vec2 POS[3] = vec2[](vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
out vec2 vUv;
void main() {
  vec2 p = POS[gl_VertexID];
  vUv = vec2(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
  gl_Position = vec4(p, 0.0, 1.0);
}`;

const RANK_FRAG = `#version 300 es
precision highp float;
in vec2 vUv;
uniform sampler2D uTex;
uniform float uPhase;
uniform float uTier;
uniform float uDiv;
uniform float uPodium;
uniform float uKind;
uniform float uTrack;
uniform float uHasTex;
uniform vec3 uMetal;
uniform vec3 uGem;
out vec4 fragColor;

float hash12(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

vec3 fastToLinear(vec3 c) {
  vec3 cl = max(c, vec3(0.0));
  return cl * cl;
}

vec3 fastToSrgb(vec3 c) {
  return sqrt(clamp(c, vec3(0.0), vec3(1.0)));
}

float pow6(float x) {
  float x2 = x * x;
  return x2 * x2 * x2;
}

float pow8(float x) {
  float x2 = x * x;
  float x4 = x2 * x2;
  return x4 * x4;
}

float pFill(float d, float aa) {
  return 1.0 - smoothstep(-aa, aa, d);
}

float pRim(float d, float width, float aa) {
  return pFill(abs(d) - width, aa);
}

float roundedPlate(vec2 p, vec2 sides) {
  vec2 v = abs(p) - sides;
  return length(max(v, vec2(0.0))) + min(max(v.x, v.y), 0.0) - 0.05;
}

vec4 neutralFrame(vec2 uv, bool deck, vec3 metal) {
  vec2 p = 2.0 * uv - 1.0;
  vec2 squared = p * p;
  float r = deck ? sqrt(sqrt(dot(squared, squared))) : length(p);
  float aa = max(fwidth(r) * 0.55, 0.001);
  vec3 col = metal * (0.65 + 0.55 * pRim(r - 0.947, 0.005, aa));
  float alpha = pRim(r - 0.952, 0.023, aa);
  return vec4(col, alpha);
}

vec4 unrankedBadge(vec2 uv, bool deck, vec3 metal) {
  vec2 p = (uv - 0.5) * 2.0;
  float aa = max(fwidth(p.x) * 0.6, 0.001);
  float coverage = 0.0;
  float bevel = 0.0;
  if (deck) {
    vec2 q = p + vec2(0.20, 0.04);
    float back = roundedPlate(vec2(q.x * 0.95 - q.y * 0.31, q.x * 0.31 + q.y * 0.95), vec2(0.33, 0.52));
    float front = roundedPlate(p - vec2(0.12, 0.04), vec2(0.34, 0.51));
    coverage = max(pFill(back, aa), pFill(front, aa));
    bevel = max(pRim(back + 0.052, 0.026, aa) * (1.0 - pFill(front, aa)), pRim(front + 0.062, 0.029, aa));
  } else {
    float crown = length((p + vec2(0.0, 0.12)) / vec2(0.56, 0.60)) - 1.0;
    float cheek = abs(p.x) - (0.54 - 0.39 * clamp(p.y + 0.08, 0.0, 1.0));
    float edge = max(cheek, max(-p.y - 0.18, p.y - 0.66 + abs(p.x) * 0.45));
    float shape = min(crown * 0.54, edge);
    float visor = max(abs(p.y + 0.04 + abs(p.x) * 0.12) - 0.084, abs(p.x) - 0.50);
    float seam = max(abs(p.x) - 0.044, max(-p.y - 0.04, p.y - 0.55));
    coverage = pFill(max(shape, -min(visor, seam)), aa);
    bevel = pRim(shape + 0.045, 0.025, aa);
  }
  float highlight = 0.17 * clamp(1.0 - uv.y, 0.0, 1.0);
  return vec4(metal * (0.58 + highlight + 0.63 * bevel), coverage);
}

vec4 unrankedRender(vec2 uv) {
  bool deck = uTrack > 0.5;
  vec3 metal = fastToLinear(uMetal);
  vec4 res = uKind > 0.5 ? neutralFrame(uv, deck, metal) : unrankedBadge(uv, deck, metal);
  vec3 srgb = fastToSrgb(res.rgb);
  return vec4(srgb * res.a, res.a);
}

void main() {
  if (uTier >= 10.5 || uHasTex < 0.5) {
    fragColor = unrankedRender(vUv);
    return;
  }
  vec4 base = texture(uTex, clamp(vUv, vec2(0.0), vec2(1.0)));
  if (base.a <= 0.002) {
    fragColor = vec4(0.0);
    return;
  }
  vec3 straight = base.rgb / max(base.a, 0.001);
  vec3 rgb = fastToLinear(straight);
  float luma = dot(rgb, vec3(0.2126, 0.7152, 0.0722));
  float relief = smoothstep(0.035, 0.30, luma);
  float cx = vUv.x - 0.5;
  float curve = uTrack > 0.5
    ? abs(cx) * 0.85 + vUv.y * 0.28
    : 0.78 * vUv.x + 0.26 * vUv.y + 0.70 * cx * cx;
  float sw = (fract(curve * 0.65 - uPhase) - 0.5) * 6.5;
  float sweep = exp(-(sw * sw));
  float hl = (fract(curve * 0.65 - uPhase - 0.105) - 0.5) * 14.0;
  float hairline = exp(-(hl * hl));
  float boxMask = (uKind < 0.5 && abs(cx) < 0.112 && vUv.y > 0.79 && vUv.y < 0.907) ? 0.15 : 1.0;

  vec3 metal = fastToLinear(uMetal);
  vec3 gem = fastToLinear(uGem);
  vec3 add = vec3(0.0);
  int t = int(floor(uTier + 0.5));

  if (t == 0) {
    add += metal * (0.14 * smoothstep(0.2, 0.9, vUv.x));
  } else if (t == 1) {
    float grain = 0.5 + 0.5 * cos(vUv.y * 42.0 + sin(vUv.x * 8.0) * 1.6);
    add += metal * (0.18 * grain);
  } else if (t == 2) {
    float brush = 0.75 + 0.25 * cos(vUv.y * 81.0 + vUv.x * 3.0);
    add += mix(metal, gem, 0.5) * sweep * 0.48 * brush;
  } else if (t == 3) {
    add += metal * sweep * 0.62 + gem * (1.0 - relief) * 0.22;
  } else if (t == 4) {
    add += gem * hairline * 0.72 + metal * sweep * 0.35;
  } else if (t == 5) {
    float rays = 0.7 + 0.3 * cos(atan(vUv.y - 0.48, cx) * 10.0 + uPhase * 6.28318);
    add += mix(metal, gem, 0.45) * sweep * 0.70 * rays;
  } else if (t == 6) {
    float en = (fract(curve * 0.65 - uPhase - 0.17) - 0.5) * 8.0;
    float enamel = exp(-(en * en));
    add += metal * sweep * 0.55 + gem * enamel * 0.65;
  } else if (t == 7) {
    vec2 cell = floor(vUv * 14.0);
    float offset = hash12(cell);
    float fs = 0.5 + 0.5 * sin((uPhase + offset) * 6.28318);
    float facet = pow8(fs) * fs * fs;
    add += gem * facet * 0.85 * relief + metal * sweep * 0.45;
  } else if (t == 8) {
    float dCore = length(vUv - vec2(0.5, 0.48));
    float pulse = (0.5 + 0.5 * sin(uPhase * 6.28318)) * exp(-dCore * 5.0);
    add += gem * pulse * 0.85 + metal * hairline * 0.65;
  } else if (t == 9) {
    float a1 = (fract(vUv.x * 0.7 + vUv.y * 0.4 - uPhase) - 0.5) * 9.0;
    float a2 = (fract(vUv.x * 0.5 - vUv.y * 0.5 - uPhase - 0.25) - 0.5) * 9.0;
    add += gem * (exp(-(a1 * a1)) + exp(-(a2 * a2))) * 0.58 + metal * sweep * 0.48;
  } else if (t >= 10) {
    float nacre = vUv.x * 3.0 + vUv.y * 2.0 + uPhase * 6.28318;
    vec3 prism = 0.5 + 0.5 * cos(vec3(0.0, 2.1, 4.2) + nacre);
    add += mix(prism, metal, 0.35) * sweep * 0.78 + gem * hairline * 0.5;
    if (uPodium > 0.5) {
      float crown = exp(-length(vUv - vec2(0.5, 0.12)) * 12.0);
      float twinkle = pow8(0.5 + 0.5 * cos(uPhase * 12.56636));
      add += vec3(0.95, 0.98, 1.0) * crown * twinkle * 1.4;
    }
  }

  if (uKind > 0.5 && t >= 2) {
    float diag = (fract(vUv.x * 0.63 + vUv.y * 0.37 - uPhase) - 0.5) * 11.0;
    float eng = exp(-(diag * diag));
    add += mix(metal, gem, 0.35) * eng * (0.28 + 0.03 * float(t));
    if (t >= 6) {
      float chroma = length(rgb - vec3(luma));
      float j = pow6(0.5 + 0.5 * cos(vUv.x * 16.0 - vUv.y * 12.0 + uPhase * 6.28318));
      add += gem * chroma * j * 1.1;
    }
  } else if (uDiv > 0.5 && uKind < 0.5) {
    float clasp = exp(-length(vUv - vec2(0.5, 0.85)) * 7.5);
    add += metal * clasp * sweep * (0.18 * uDiv);
  }

  rgb += add * relief * boxMask;
  vec3 srgb = fastToSrgb(rgb);
  fragColor = vec4(srgb * base.a, base.a);
}`;

const INSIGNIA_FRAG = `#version 300 es
precision highp float;
in vec2 vUv;
uniform float uTime;
uniform float uEnergy;
uniform float uTier;
uniform float uMotif;
uniform vec3 uMetal;
uniform vec3 uGem;
out vec4 fragColor;

const float PI = 3.14159265;
const float TAU = 6.28318530;

vec3 fastToLinear(vec3 c) {
  vec3 cl = max(c, vec3(0.0));
  return cl * cl;
}

vec3 fastToSrgb(vec3 c) {
  return sqrt(clamp(c, vec3(0.0), vec3(1.0)));
}

float pow6(float x) {
  float x2 = x * x;
  return x2 * x2 * x2;
}

float pow8(float x) {
  float x2 = x * x;
  float x4 = x2 * x2;
  return x4 * x4;
}

float stroke(float d, float width, float aa) {
  return 1.0 - smoothstep(width - aa, width + aa, abs(d));
}

float fill(float d, float aa) {
  return 1.0 - smoothstep(-aa, aa, d);
}

float segment(vec2 p, vec2 a, vec2 b) {
  vec2 q = p - a;
  vec2 v = b - a;
  return length(q - v * clamp(dot(q, v) / dot(v, v), 0.0, 1.0));
}

float diamond(vec2 p, vec2 size) {
  return (abs(p.x) / size.x + abs(p.y) / size.y - 1.0) * min(size.x, size.y);
}

vec3 metalLight(vec2 p, float r, float centre, float width, vec3 metal) {
  float slope = clamp((r - centre) / width, -0.96, 0.96);
  vec2 radial = p / max(r, 0.001);
  vec3 n = vec3(radial * slope, sqrt(1.0 - slope * slope));
  vec3 key = normalize(vec3(-0.45, -0.65, 0.70));
  float diffuse = max(dot(n, key), 0.0);
  float refDot = max(dot(n, normalize(key + vec3(0.0, 0.0, 1.0))), 0.0);
  float r2 = refDot * refDot;
  float r4 = r2 * r2;
  float reflection = r4 * r4 * r4 * r4 * r4 * r2;
  float brushed = 0.97 + 0.03 * sin(r * 720.0);
  return metal * (0.18 + 0.92 * diffuse) * brushed
    + mix(metal, vec3(1.0), 0.45) * reflection * 0.70;
}

float emblem(vec2 p, float kind, float aa) {
  float r = length(p);
  float a = atan(p.y, p.x);
  if (kind < 0.5) {
    float sun = fill(r - 0.135, aa);
    float rays = stroke(sin(a * 6.0), 0.23, aa * 6.0) * fill(r - 0.255, aa) * fill(0.176 - r, aa);
    return max(sun, rays);
  }
  if (kind < 1.5) {
    float moon = min(
      fill(length(p + vec2(0.04, 0.0)) - 0.21, aa),
      1.0 - fill(length(p - vec2(0.055, -0.045)) - 0.18, aa)
    );
    float star = fill(diamond(p - vec2(0.15, -0.12), vec2(0.07, 0.105)), aa);
    return max(moon, star);
  }
  if (kind < 2.5) {
    float arms = max(fill(diamond(p, vec2(0.065, 0.26)), aa), fill(diamond(p, vec2(0.22, 0.065)), aa));
    return max(arms, stroke(diamond(p, vec2(0.17)), 0.012, aa));
  }
  if (kind < 3.5) {
    float ring = stroke(length(p - vec2(0.02, -0.01)) - 0.16, 0.016, aa);
    float shaft = stroke(segment(p, vec2(-0.19, 0.22), vec2(0.14, -0.17)), 0.018, aa);
    float head = max(
      stroke(segment(p, vec2(0.14, -0.17), vec2(0.02, -0.16)), 0.018, aa),
      stroke(segment(p, vec2(0.14, -0.17), vec2(0.15, -0.05)), 0.018, aa)
    );
    return max(ring, max(shaft, head));
  }
  if (kind > 10.5) {
    float leftNode = length(p - vec2(-0.17, 0.04));
    float rightNode = length(p - vec2(0.17, 0.04));
    float nodes = max(stroke(leftNode - 0.075, 0.017, aa), stroke(rightNode - 0.075, 0.017, aa));
    float cores = max(fill(leftNode - 0.032, aa), fill(rightNode - 0.032, aa));
    float bridge = max(
      stroke(segment(p, vec2(-0.10, 0.02), vec2(0.0, -0.14)), 0.017, aa),
      stroke(segment(p, vec2(0.10, 0.02), vec2(0.0, -0.14)), 0.017, aa)
    );
    float base = stroke(segment(p, vec2(-0.085, 0.045), vec2(0.085, 0.045)), 0.014, aa);
    float beacon = max(
      fill(diamond(p - vec2(0.0, -0.16), vec2(0.065, 0.095)), aa),
      fill(diamond(p - vec2(0.0, -0.16), vec2(0.095, 0.035)), aa)
    );
    return max(max(nodes, cores), max(max(bridge, base), beacon));
  }
  if (kind > 9.5) {
    float shield = max(abs(p.x) - 0.14, max(-p.y - 0.19, p.y + abs(p.x) * 0.85 - 0.23));
    float star = max(fill(diamond(p, vec2(0.04, 0.13)), aa), fill(diamond(p, vec2(0.09, 0.04)), aa));
    float wing = max(
      stroke(segment(vec2(abs(p.x), p.y), vec2(0.16, 0.10), vec2(0.26, -0.15)), 0.018, aa),
      stroke(segment(vec2(abs(p.x), p.y), vec2(0.18, 0.02), vec2(0.28, -0.02)), 0.016, aa)
    );
    return max(stroke(shield, 0.018, aa), max(star, wing));
  }
  if (kind > 8.5) {
    float peak = stroke(diamond(p - vec2(0.0, 0.07), vec2(0.16, 0.19)), 0.020, aa);
    float facets = max(stroke(p.x, 0.012, aa), stroke(p.y - 0.07, 0.012, aa))
      * fill(diamond(p - vec2(0.0, 0.07), vec2(0.16, 0.19)), aa);
    float crown = fill(max(abs(p.x) - 0.22, max(p.y + 0.13, -p.y - 0.20 - 0.045 * cos(p.x * 42.0))), aa);
    return max(crown, max(peak, facets));
  }
  if (kind > 7.5) {
    vec2 q = vec2(abs(p.x), p.y);
    float blade = stroke(segment(q, vec2(0.0, 0.0), vec2(0.20, -0.23)), 0.026, aa);
    float grip = stroke(segment(q, vec2(0.0, 0.0), vec2(0.19, 0.20)), 0.016, aa);
    float guard = stroke(segment(q, vec2(0.045, 0.19), vec2(0.20, 0.045)), 0.022, aa);
    return max(blade, max(grip, guard));
  }
  if (kind > 6.5) {
    float shield = max(abs(p.x) - 0.21, max(-p.y - 0.22, p.y + abs(p.x) * 0.72 - 0.25));
    float rim = stroke(shield, 0.019, aa);
    float knot = stroke(diamond(p, vec2(0.10, 0.16)), 0.019, aa);
    return max(rim, knot);
  }
  if (kind > 5.5) {
    float tower = fill(max(abs(p.x) - 0.12, abs(p.y - 0.055) - 0.18), aa);
    float door = fill(max(abs(p.x) - 0.045, abs(p.y - 0.15) - 0.08), aa);
    float crown = fill(max(abs(p.x) - 0.23, max(p.y + 0.11, -p.y - 0.19 - 0.07 * cos(p.x * 28.0))), aa);
    return max(tower * (1.0 - door), crown);
  }
  if (kind > 4.5) {
    vec2 q = vec2(p.x, p.y + 0.02);
    float peak = max(
      stroke(segment(q, vec2(-0.23, 0.19), vec2(0.0, -0.18)), 0.018, aa),
      stroke(segment(q, vec2(0.0, -0.18), vec2(0.23, 0.19)), 0.018, aa)
    );
    float path = max(
      stroke(segment(q, vec2(0.06, 0.21), vec2(-0.06, 0.08)), 0.017, aa),
      stroke(segment(q, vec2(-0.06, 0.08), vec2(0.035, -0.02)), 0.017, aa)
    );
    float star = fill(diamond(p - vec2(0.18, -0.20), vec2(0.04, 0.065)), aa);
    return max(peak, max(path, star));
  }
  float tree = fill(length(p - vec2(0.0, -0.13)) - 0.13, aa);
  tree = max(tree, fill(length(p - vec2(0.12, -0.04)) - 0.105, aa));
  tree = max(tree, fill(length(p - vec2(-0.12, -0.04)) - 0.105, aa));
  tree = max(tree, stroke(segment(p, vec2(0.0, -0.04), vec2(0.0, 0.2)), 0.025, aa));
  tree = max(tree, stroke(segment(p, vec2(-0.13, 0.23), vec2(0.0, 0.15)), 0.014, aa));
  return max(tree, stroke(segment(p, vec2(0.13, 0.23), vec2(0.0, 0.15)), 0.014, aa));
}

void main() {
  vec2 p = (vUv - 0.5) * 2.0;
  float r = length(p);
  float a = atan(p.y, p.x);
  float aa = max(fwidth(r), 0.003);
  float tier = uTier;
  float earned = tier > 0.5 ? 1.0 : 0.0;
  float t = uTime * uEnergy;
  vec3 metal = fastToLinear(uMetal);
  vec3 gem = fastToLinear(uGem);

  float disc = fill(r - 0.53, aa);
  float edge = stroke(r - 0.51, 0.028, aa);
  float bevel = stroke(r - 0.465, 0.012, aa);
  float inset = fill(r - 0.44, aa);
  float engraving = stroke(r - 0.399, 0.005, aa);
  float light = clamp(0.56 - p.y * 0.42 - p.x * 0.24, 0.18, 1.0);

  vec3 color = metal * light * 0.44;
  color = mix(color, metalLight(p, r, 0.505, 0.027, metal), edge);
  color = mix(color, metalLight(p, r, 0.465, 0.013, metal), bevel);

  vec3 enamel = mix(vec3(0.006, 0.010, 0.015), gem * 0.035, earned);
  float enamelLight = 0.65 + 0.55 * max(0.0, 1.0 - length(p + vec2(0.16, 0.20)) * 1.5);
  color = mix(color, enamel * enamelLight, inset);
  color = mix(color, metal * 0.38, engraving);

  float etch = emblem(p, uMotif, aa);
  float relief = emblem(p + vec2(0.006, 0.009), uMotif, aa);
  color = mix(color, vec3(0.002, 0.004, 0.006), max(0.0, relief - etch) * 0.8);
  float hiTerm = max(0.0, 1.0 - abs(p.x + p.y * 0.7) * 3.0);
  vec3 raised = metal * (0.48 + light * 0.65)
    + mix(metal, vec3(1.0), 0.35) * pow6(hiTerm) * 0.24;
  color = mix(color, raised, etch);
  color += mix(metal, vec3(1.0), 0.5) * max(0.0, etch - relief) * 0.55;

  float alpha = disc;

  if (tier > 1.5) {
    float outer = stroke(r - 0.585, 0.013, aa);
    float rivets = stroke(r - 0.56, 0.018, aa) * stroke(sin(a * 4.0), 0.13, aa * 3.0);
    float detail = max(outer, rivets);
    color = mix(color, metalLight(p, r, 0.585, 0.016, metal), detail);
    alpha = max(alpha, detail);
  }

  if (tier > 2.5) {
    float stem = stroke(r - 0.70, 0.009, aa) * fill(-0.62 - p.y, aa) * fill(p.y - 0.60, aa);
    float leaves = 0.0;
    for (int i = 0; i < 7; i++) {
      float angle = -1.05 + float(i) * 0.34;
      vec2 centre = vec2(cos(angle), sin(angle)) * 0.70;
      vec2 q = vec2(abs(p.x), p.y) - centre;
      float sAng = sin(angle + 0.7);
      float cAng = cos(angle + 0.7);
      vec2 leaf = vec2(q.x * cAng + q.y * sAng, -q.x * sAng + q.y * cAng);
      leaves = max(leaves, fill(diamond(leaf, vec2(0.092, 0.032)), aa));
    }
    float wreath = max(stem, leaves);
    float facets = 0.38 + light * 0.72 + 0.20 * sin(a * 24.0);
    color = mix(color, metal * facets, wreath);
    alpha = max(alpha, wreath);
  }

  if (tier > 3.5) {
    for (int i = 0; i < 5; i++) {
      float angle = -PI * 0.5 + float(i) * TAU / 5.0;
      vec2 centre = vec2(cos(angle), sin(angle)) * 0.59;
      float cut = diamond(p - centre, vec2(0.043, 0.065));
      float socket = fill(cut - 0.014, aa);
      color = mix(color, metal, socket);
      float facet = (p.x < centre.x ? 1.25 : 0.45) + (p.y < centre.y ? 0.25 : 0.0);
      color = mix(color, gem * facet, fill(cut, aa));
      alpha = max(alpha, socket);
    }
    float pendant = fill(diamond(p - vec2(0.0, 0.79), vec2(0.085, 0.105)), aa);
    color = mix(color, gem * (p.x < 0.0 ? 1.20 : 0.48), pendant);
    alpha = max(alpha, pendant);
  }

  if (tier > 4.5) {
    vec2 q = p - vec2(0.0, -0.72);
    float sx = sin(q.x * 13.65);
    float sx4 = sx * sx * sx * sx;
    float crown = fill(
      max(abs(q.x) - 0.23, max(q.y - 0.09, -q.y - 0.055 - 0.12 * sx4)),
      aa
    );
    color = mix(color, metal * (0.65 + light), crown);
    alpha = max(alpha, crown);
  }

  float gc = max(0.0, cos(p.x * 3.0 + p.y * 2.0 - t * 0.65));
  float gc8 = pow8(gc);
  float glint = gc8 * gc8 * gc8 * 0.32 * earned * uEnergy;
  color = color + metal * glint * max(edge, etch);
  float hr = (r - 0.77) * 10.0;
  float halo = exp(-(hr * hr)) * 0.10 * (tier > 4.5 ? 1.0 : 0.0);
  float breath = 0.85 + 0.15 * sin(t * 0.7) * uEnergy;
  color = mix(gem * 0.4 * breath, color, alpha);
  alpha = max(alpha, halo * breath);
  vec3 srgb = fastToSrgb(color * mix(0.50, 1.0, earned));
  fragColor = vec4(srgb * alpha, alpha);
}`;

const CARD_FINISH_FRAG = `#version 300 es
precision highp float;
in vec2 vUv;
uniform float uTime;
uniform float uFinish;
uniform float uAngle;
out vec4 fragColor;

float hash12(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

float pow6(float x) {
  float x2 = x * x;
  return x2 * x2 * x2;
}

float cardCornerSdf(vec2 uv) {
  vec2 q = abs(uv - 0.5) - vec2(0.4525, 0.466);
  return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - 0.042;
}

void main() {
  float sdf = cardCornerSdf(vUv);
  float cardMask = 1.0 - smoothstep(-0.004, 0.004, sdf);
  if (cardMask <= 0.001) {
    fragColor = vec4(0.0);
    return;
  }

  float ga = 1.0 - clamp(abs(uAngle), 0.0, 1.0);
  float glint = ga * ga;
  float travel = sin(uTime * 0.52 + uAngle * 2.5) * 0.72 + 0.65;
  float diag = vUv.x * 0.72 + vUv.y;
  float bd = diag - travel;
  float band = exp(-20.0 * bd * bd);
  float counterTravel = sin(-uTime * 0.31 + uAngle * 1.6) * 0.55 + 0.60;
  float cbd = (1.0 - vUv.x) * 0.6 + vUv.y - counterTravel;
  float counterBand = exp(-26.0 * cbd * cbd);
  float edge = 1.0 - smoothstep(0.003, 0.019, abs(sdf));
  float edgeLight = edge * (0.18 + 0.82 * max(band, counterBand));
  float cornerGlow = exp(-7.5 * length(vUv - vec2(0.18 + uAngle * 0.14, 0.16)));

  vec3 light = vec3(0.0);
  if (uFinish < 1.5) {
    float ripple = sin(vUv.x * 11.0 - vUv.y * 8.0 + uTime * 0.35) * 0.12;
    float phase = (vUv.x * 0.75 + vUv.y * 0.55) + uAngle * 0.92 + uTime * 0.09 + ripple;
    vec3 rainbow = 0.5 + 0.5 * cos(6.283185 * (vec3(phase) + vec3(0.0, 0.3333, 0.6667)));
    vec3 sheen = mix(vec3(0.85, 0.92, 1.0), rainbow, 0.88);
    vec2 sparkCell = floor(vUv * vec2(38.0, 54.0));
    float sparkNoise = hash12(sparkCell);
    float ms = max(0.0, sin(uTime * 1.4 + sparkNoise * 12.0 + uAngle * 2.0));
    float microSpark = step(0.84, sparkNoise) * pow6(ms) * band * 0.42;
    float spec = band * (0.42 + 0.18 * glint)
      + counterBand * 0.16
      + cornerGlow * 0.11 * (0.4 + 0.6 * glint)
      + microSpark;
    light = sheen * spec + mix(rainbow, vec3(1.0), 0.25) * edgeLight * 0.34;
  } else {
    float diagPhase = (vUv.x - vUv.y) * 240.0;
    float grain = (0.5 + 0.5 * sin(diagPhase)) * (1.0 - smoothstep(0.7, 2.5, fwidth(diagPhase)));
    float tilt = 0.5 + 0.5 * sin(uTime * 0.42 + vUv.y * 4.5 + uAngle * 2.8);
    vec3 warmGold = vec3(0.98, 0.78, 0.40);
    vec3 coolSteel = vec3(0.42, 0.70, 0.86);
    vec3 metal = mix(coolSteel, warmGold, tilt);
    float fSin = 0.5 + 0.5 * sin(vUv.y * 26.0 - vUv.x * 11.0 - uTime * 0.65 + uAngle * 1.4);
    float facet = pow6(fSin) * fSin;
    float bodySpec = (band * 0.34 + facet * 0.18 + cornerGlow * 0.08) * (0.55 + 0.45 * grain);
    float bevel = 1.0 - smoothstep(0.024, 0.044, -sdf);
    light = metal * bodySpec
      + warmGold * edgeLight * 0.58
      + mix(warmGold, coolSteel, 0.35) * bevel * (0.12 + 0.28 * band);
  }

  fragColor = vec4(clamp(light, vec3(0.0), vec3(1.0)) * cardMask, cardMask);
}`;

// ============================================================================
// Batched Tiered Renderer Engine (WebGPU -> WebGL2 -> 2D Canvas)
// ============================================================================

type ShaderBackend = "webgpu" | "webgl2" | "2d";

interface CanvasMeta {
  cssW: number;
  cssH: number;
  backingW: number;
  backingH: number;
  lastKey: string;
  lastDrawMs: number;
  hoverUntil: number;
  gpuCtx: GPUCanvasContext | null;
  gpuConfigured: boolean;
  ctx2d: CanvasRenderingContext2D | null;
  backendTag: ShaderBackend | "";
  lit: boolean;
}

interface ImageRecord {
  readonly img: HTMLImageElement;
  ready: boolean;
}

interface GpuTextureEntry {
  readonly texture: GPUTexture;
  readonly bindGroup: GPUBindGroup;
}

interface GpuScratchEntry {
  readonly texture: GPUTexture;
  readonly view: GPUTextureView;
}

interface GpuState {
  readonly device: GPUDevice;
  readonly format: GPUTextureFormat;
  readonly canCopyDst: boolean;
  readonly canvasUsage: GPUTextureUsageFlags;
  readonly rank: GPURenderPipeline;
  readonly insignia: GPURenderPipeline;
  readonly cardFinish: GPURenderPipeline;
  readonly mipPipe: GPURenderPipeline;
  readonly mipLayout: GPUBindGroupLayout;
  readonly texLayout: GPUBindGroupLayout;
  readonly sampler: GPUSampler;
  readonly uBuffer: GPUBuffer;
  readonly uBindGroup: GPUBindGroup;
  readonly dummyBindGroup: GPUBindGroup;
  readonly scratchPool: Map<string, GpuScratchEntry>;
}

interface GlPrograms {
  readonly vao: WebGLVertexArrayObject | null;
  readonly rank: {
    readonly prog: WebGLProgram;
    readonly uTex: WebGLUniformLocation | null;
    readonly uPhase: WebGLUniformLocation | null;
    readonly uTier: WebGLUniformLocation | null;
    readonly uDiv: WebGLUniformLocation | null;
    readonly uPodium: WebGLUniformLocation | null;
    readonly uKind: WebGLUniformLocation | null;
    readonly uTrack: WebGLUniformLocation | null;
    readonly uHasTex: WebGLUniformLocation | null;
    readonly uMetal: WebGLUniformLocation | null;
    readonly uGem: WebGLUniformLocation | null;
  };
  readonly insignia: {
    readonly prog: WebGLProgram;
    readonly uTime: WebGLUniformLocation | null;
    readonly uEnergy: WebGLUniformLocation | null;
    readonly uTier: WebGLUniformLocation | null;
    readonly uMotif: WebGLUniformLocation | null;
    readonly uMetal: WebGLUniformLocation | null;
    readonly uGem: WebGLUniformLocation | null;
  };
  readonly cardFinish: {
    readonly prog: WebGLProgram;
    readonly uTime: WebGLUniformLocation | null;
    readonly uFinish: WebGLUniformLocation | null;
    readonly uAngle: WebGLUniformLocation | null;
  };
}

interface BaseJob {
  readonly canvas: HTMLCanvasElement;
  readonly meta: CanvasMeta;
  readonly visualKey: string;
  readonly w: number;
  readonly h: number;
  readonly intervalMs: number;
}

interface InsigniaJob extends BaseJob {
  readonly pipeline: "insignia";
  readonly nowSec: number;
  readonly energy: number;
  readonly tier: number;
  readonly motifIdx: number;
  readonly metal: RgbTuple;
  readonly gem: RgbTuple;
}

interface CardFinishJob extends BaseJob {
  readonly pipeline: "cardFinish";
  readonly t: number;
  readonly finish: number;
  readonly angle: number;
}

interface RankJob extends BaseJob {
  readonly pipeline: "rank";
  readonly kind: string;
  readonly phase: number;
  readonly tier: number;
  readonly div: number;
  readonly podium: number;
  readonly track: number;
  readonly url: string | null;
  readonly metal: RgbTuple;
  readonly gem: RgbTuple;
}

type RenderJob = InsigniaJob | CardFinishJob | RankJob;

interface PackedJob {
  readonly job: RenderJob;
  readonly x: number;
  readonly y: number;
  readonly rw: number;
  readonly rh: number;
}

function writeSlotUniforms(
  stage: Float32Array,
  base: number,
  s: RenderJob,
  hasGpuTex: boolean,
): void {
  if (s.pipeline === "insignia") {
    stage[base + 0] = s.nowSec;
    stage[base + 1] = s.energy;
    stage[base + 2] = s.tier;
    stage[base + 3] = 0;
    stage[base + 4] = 0;
    stage[base + 5] = 0;
    stage[base + 6] = 0;
    stage[base + 7] = 0;
    stage[base + 8] = s.metal[0];
    stage[base + 9] = s.metal[1];
    stage[base + 10] = s.metal[2];
    stage[base + 11] = s.motifIdx;
    stage[base + 12] = s.gem[0];
    stage[base + 13] = s.gem[1];
    stage[base + 14] = s.gem[2];
    stage[base + 15] = 0;
  } else if (s.pipeline === "cardFinish") {
    stage[base + 0] = s.t;
    stage[base + 1] = 0;
    stage[base + 2] = 0;
    stage[base + 3] = 0;
    stage[base + 4] = s.finish;
    stage[base + 5] = s.angle;
    stage[base + 6] = 0;
    stage[base + 7] = 0;
  } else {
    stage[base + 0] = s.phase;
    stage[base + 1] = 0;
    stage[base + 2] = s.tier;
    stage[base + 3] = s.div;
    stage[base + 4] = s.podium;
    stage[base + 5] = s.kind === "frame" ? 1.0 : 0.0;
    stage[base + 6] = s.track;
    stage[base + 7] = hasGpuTex ? 1.0 : 0.0;
    stage[base + 8] = s.metal[0];
    stage[base + 9] = s.metal[1];
    stage[base + 10] = s.metal[2];
    stage[base + 11] = 0;
    stage[base + 12] = s.gem[0];
    stage[base + 13] = s.gem[1];
    stage[base + 14] = s.gem[2];
    stage[base + 15] = 0;
  }
}

class BadgeEngine {
  urls: Record<string, string>;
  readonly reduced: boolean;
  readonly items = new Set<HTMLCanvasElement>();
  readonly visible = new Set<HTMLCanvasElement>();
  readonly meta = new WeakMap<HTMLCanvasElement, CanvasMeta>();
  readonly images = new Map<string, ImageRecord>();
  readonly glTextures = new Map<string, WebGLTexture>();
  readonly gpuTextures = new Map<string, GpuTextureEntry>();
  readonly pendingAdded = new Set<Element>();
  readonly start = performance.now();
  raf = 0;
  scanScheduled = false;
  gpuPending = false;
  scrollingUntil = 0;
  hasAnimatedVisible = false;

  backend: ShaderBackend = "2d";
  gpu: GpuState | null = null;
  gl: WebGL2RenderingContext | null = null;
  glCanvas: HTMLCanvasElement | null = null;
  glPrograms: GlPrograms | null = null;
  glAnisoExt: EXT_texture_filter_anisotropic | null = null;
  glMaxAniso = 1;
  readonly uniformStage = new Float32Array(MAX_SLOTS * UNIFORM_STRIDE_FLOATS);
  readonly ro: ResizeObserver | null;
  readonly io: IntersectionObserver | null;

  constructor() {
    this.urls = rankUrls();
    this.reduced = window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches ?? false;

    this.ro =
      typeof ResizeObserver === "function"
        ? new ResizeObserver((entries) => {
            let changed = false;
            for (const entry of entries) {
              const canvas = entry.target;
              if (!(canvas instanceof HTMLCanvasElement)) continue;
              const m = this.meta.get(canvas);
              if (!m) continue;
              const cr = entry.contentRect;
              if (this.syncCssSize(canvas, m, cr.width, cr.height)) {
                changed = true;
              }
            }
            if (changed) this.kick(true);
          })
        : null;

    this.io =
      typeof IntersectionObserver === "function"
        ? new IntersectionObserver(
            (entries) => {
              let newlyVisible = false;
              for (const e of entries) {
                if (!(e.target instanceof HTMLCanvasElement)) continue;
                if (e.isIntersecting) {
                  if (!this.visible.has(e.target)) newlyVisible = true;
                  this.visible.add(e.target);
                } else {
                  this.visible.delete(e.target);
                  const m = this.meta.get(e.target);
                  if (m?.gpuConfigured && m.gpuCtx) {
                    m.gpuCtx.unconfigure();
                    m.gpuConfigured = false;
                    m.lastKey = "";
                  }
                }
              }
              this.refreshAnimatedFlag();
              if (newlyVisible || this.hasAnimatedVisible) this.kick(true);
            },
            { rootMargin: "120px" },
          )
        : null;

    document.addEventListener(
      "scroll",
      () => {
        this.scrollingUntil = performance.now() + 120;
      },
      { capture: true, passive: true },
    );

    this.watchDpr();
    this.initBackends();
  }

  private watchDpr(): void {
    if (typeof window.matchMedia !== "function") return;
    const dpr = window.devicePixelRatio || 1;
    const mql = window.matchMedia(`(resolution: ${dpr}dppx)`);
    mql.addEventListener(
      "change",
      () => {
        this.watchDpr();
        this.flushVisible(true);
      },
      { once: true },
    );
  }

  markDomState(s: RenderJob, tag: ShaderBackend): void {
    const m = s.meta;
    if (!m.lit) {
      s.canvas.parentElement?.classList.add("is-lit");
      m.lit = true;
    }
    if (m.backendTag !== tag) {
      s.canvas.dataset.shaderBackend = tag;
      m.backendTag = tag;
    }
  }

  setBackendTag(name: ShaderBackend): void {
    this.backend = name;
    document.documentElement.dataset.shaderBackend = name;
    for (const c of this.items) {
      if (!c.isConnected) continue;
      const m = this.meta.get(c);
      if (m) m.backendTag = name;
      c.dataset.shaderBackend = name;
    }
  }

  initBackends(): void {
    if (typeof navigator !== "undefined" && navigator.gpu) {
      // Defer 2D fallback context creation while WebGPU initializes so <canvas>
      // elements are never permanently locked away from canvas.getContext("webgpu").
      this.gpuPending = true;
      document.documentElement.dataset.shaderBackend = "webgpu";
      void this.initWebGpu().then((ok) => {
        this.gpuPending = false;
        if (!ok) {
          this.initWebGl2();
        }
        this.invalidateAll();
        this.refreshAnimatedFlag();
        this.flushVisible(true);
      });
    } else {
      this.initWebGl2();
    }
  }

  invalidateAll(): void {
    for (const c of this.items) {
      const m = this.meta.get(c);
      if (m) m.lastKey = "";
    }
  }

  async initWebGpu(): Promise<boolean> {
    try {
      const gpuApi = navigator.gpu;
      if (!gpuApi) return false;
      const adapter = await gpuApi.requestAdapter({ powerPreference: "high-performance" });
      if (!adapter) return false;
      const device = await adapter.requestDevice();
      const format = gpuApi.getPreferredCanvasFormat();

      let canCopyDst = false;
      const probeCanvas = document.createElement("canvas");
      probeCanvas.width = 4;
      probeCanvas.height = 4;
      const probeCtx = probeCanvas.getContext("webgpu") as GPUCanvasContext | null;
      if (!probeCtx) {
        device.destroy?.();
        return false;
      }
      try {
        probeCtx.configure({
          device,
          format,
          alphaMode: "premultiplied",
          usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_DST,
        });
        const probeTex = probeCtx.getCurrentTexture();
        canCopyDst = (probeTex.usage & GPUTextureUsage.COPY_DST) !== 0;
        probeCtx.unconfigure();
      } catch {
        canCopyDst = false;
        try {
          probeCtx.configure({
            device,
            format,
            alphaMode: "premultiplied",
            usage: GPUTextureUsage.RENDER_ATTACHMENT,
          });
          probeCtx.unconfigure();
        } catch {
          device.destroy?.();
          return false;
        }
      }

      const canvasUsage = canCopyDst
        ? GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_DST
        : GPUTextureUsage.RENDER_ATTACHMENT;

      const uniformLayout = device.createBindGroupLayout({
        entries: [
          {
            binding: 0,
            visibility: GPUShaderStage.FRAGMENT,
            buffer: {
              type: "uniform",
              hasDynamicOffset: true,
              minBindingSize: 64,
            },
          },
        ],
      });
      const texLayout = device.createBindGroupLayout({
        entries: [
          { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: "float" } },
          { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: "filtering" } },
        ],
      });
      const mipLayout = device.createBindGroupLayout({
        entries: [
          { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: "float" } },
          { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: "filtering" } },
        ],
      });

      const procPipelineLayout = device.createPipelineLayout({ bindGroupLayouts: [uniformLayout] });
      const rankPipelineLayout = device.createPipelineLayout({ bindGroupLayouts: [uniformLayout, texLayout] });
      const mipPipelineLayout = device.createPipelineLayout({ bindGroupLayouts: [mipLayout] });

      const blend: GPUBlendState = {
        color: { srcFactor: "one", dstFactor: "zero", operation: "add" },
        alpha: { srcFactor: "one", dstFactor: "zero", operation: "add" },
      };
      const buildPipeline = (code: string, layout: GPUPipelineLayout): GPURenderPipeline => {
        const shaderModule = device.createShaderModule({ code });
        return device.createRenderPipeline({
          layout,
          vertex: { module: shaderModule, entryPoint: "vs_main" },
          fragment: {
            module: shaderModule,
            entryPoint: "fs_main",
            targets: [{ format, blend }],
          },
          primitive: { topology: "triangle-list" },
        });
      };

      const rank = buildPipeline(RANK_WGSL, rankPipelineLayout);
      const insignia = buildPipeline(INSIGNIA_WGSL, procPipelineLayout);
      const cardFinish = buildPipeline(CARD_FINISH_WGSL, procPipelineLayout);

      const mipMod = device.createShaderModule({ code: MIP_WGSL });
      const mipPipe = device.createRenderPipeline({
        layout: mipPipelineLayout,
        vertex: { module: mipMod, entryPoint: "vs_mip" },
        fragment: {
          module: mipMod,
          entryPoint: "fs_mip",
          targets: [{ format: "rgba8unorm" }],
        },
        primitive: { topology: "triangle-list" },
      });

      const sampler = device.createSampler({
        addressModeU: "clamp-to-edge",
        addressModeV: "clamp-to-edge",
        minFilter: "linear",
        magFilter: "linear",
        mipmapFilter: "linear",
        maxAnisotropy: 4,
      });

      const uBuffer = device.createBuffer({
        size: MAX_SLOTS * UNIFORM_STRIDE_BYTES,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      });
      const uBindGroup = device.createBindGroup({
        layout: uniformLayout,
        entries: [{ binding: 0, resource: { buffer: uBuffer, size: 64 } }],
      });

      const dummyTex = device.createTexture({
        size: [1, 1, 1],
        format: "rgba8unorm",
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
      });
      device.queue.writeTexture(
        { texture: dummyTex },
        new Uint8Array([0, 0, 0, 0]),
        { bytesPerRow: 4 },
        [1, 1, 1],
      );
      const dummyBindGroup = device.createBindGroup({
        layout: texLayout,
        entries: [
          { binding: 0, resource: dummyTex.createView() },
          { binding: 1, resource: sampler },
        ],
      });

      this.gpu = {
        device,
        format,
        canCopyDst,
        canvasUsage,
        rank,
        insignia,
        cardFinish,
        mipPipe,
        mipLayout,
        texLayout,
        sampler,
        uBuffer,
        uBindGroup,
        dummyBindGroup,
        scratchPool: new Map(),
      };
      this.setBackendTag("webgpu");

      for (const [url, rec] of this.images) {
        if (rec.ready) this.uploadGpuTexture(url, rec.img);
      }

      void device.lost.then(() => {
        if (this.gpu?.device !== device) return;
        for (const entry of this.gpu.scratchPool.values()) entry.texture.destroy();
        this.gpu = null;
        this.gpuTextures.clear();
        for (const c of this.items) {
          const m = this.meta.get(c);
          if (m) {
            m.gpuCtx = null;
            m.gpuConfigured = false;
            m.lastKey = "";
          }
        }
        this.initWebGl2();
        this.invalidateAll();
        this.refreshAnimatedFlag();
        this.flushVisible(true);
      });

      return true;
    } catch {
      this.gpu = null;
      return false;
    }
  }

  initWebGl2(): boolean {
    if (this.gl) {
      this.setBackendTag("webgl2");
      return true;
    }
    const glCanvas = document.createElement("canvas");
    glCanvas.width = ATLAS_WIDTH;
    glCanvas.height = 512;
    const gl = glCanvas.getContext("webgl2", {
      alpha: true,
      premultipliedAlpha: true,
      antialias: true,
      powerPreference: "high-performance",
    });
    const programs = gl ? this.buildGlPrograms(gl) : null;
    if (gl && programs) {
      this.glCanvas = glCanvas;
      this.gl = gl;
      this.glPrograms = programs;
      const ext = gl.getExtension("EXT_texture_filter_anisotropic");
      this.glAnisoExt = ext;
      this.glMaxAniso = ext ? Math.min(4, Number(gl.getParameter(ext.MAX_TEXTURE_MAX_ANISOTROPY_EXT)) || 1) : 1;
      this.setBackendTag("webgl2");
      for (const [url, rec] of this.images) {
        if (rec.ready) this.uploadGlTexture(url, rec.img);
      }
      return true;
    }
    this.setBackendTag("2d");
    return false;
  }

  buildGlPrograms(gl: WebGL2RenderingContext): GlPrograms | null {
    const compile = (type: number, src: string): WebGLShader | null => {
      const sh = gl.createShader(type);
      if (!sh) return null;
      gl.shaderSource(sh, src);
      gl.compileShader(sh);
      if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
        gl.deleteShader(sh);
        return null;
      }
      return sh;
    };
    const link = (frag: string): WebGLProgram | null => {
      const vs = compile(gl.VERTEX_SHADER, VERT);
      const fs = compile(gl.FRAGMENT_SHADER, frag);
      if (!vs || !fs) return null;
      const prog = gl.createProgram();
      if (!prog) return null;
      gl.attachShader(prog, vs);
      gl.attachShader(prog, fs);
      gl.linkProgram(prog);
      gl.deleteShader(vs);
      gl.deleteShader(fs);
      if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
        gl.deleteProgram(prog);
        return null;
      }
      return prog;
    };
    const rank = link(RANK_FRAG);
    const insignia = link(INSIGNIA_FRAG);
    const cardFinish = link(CARD_FINISH_FRAG);
    if (!rank || !insignia || !cardFinish) return null;
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    return {
      vao,
      rank: {
        prog: rank,
        uTex: gl.getUniformLocation(rank, "uTex"),
        uPhase: gl.getUniformLocation(rank, "uPhase"),
        uTier: gl.getUniformLocation(rank, "uTier"),
        uDiv: gl.getUniformLocation(rank, "uDiv"),
        uPodium: gl.getUniformLocation(rank, "uPodium"),
        uKind: gl.getUniformLocation(rank, "uKind"),
        uTrack: gl.getUniformLocation(rank, "uTrack"),
        uHasTex: gl.getUniformLocation(rank, "uHasTex"),
        uMetal: gl.getUniformLocation(rank, "uMetal"),
        uGem: gl.getUniformLocation(rank, "uGem"),
      },
      insignia: {
        prog: insignia,
        uTime: gl.getUniformLocation(insignia, "uTime"),
        uEnergy: gl.getUniformLocation(insignia, "uEnergy"),
        uTier: gl.getUniformLocation(insignia, "uTier"),
        uMotif: gl.getUniformLocation(insignia, "uMotif"),
        uMetal: gl.getUniformLocation(insignia, "uMetal"),
        uGem: gl.getUniformLocation(insignia, "uGem"),
      },
      cardFinish: {
        prog: cardFinish,
        uTime: gl.getUniformLocation(cardFinish, "uTime"),
        uFinish: gl.getUniformLocation(cardFinish, "uFinish"),
        uAngle: gl.getUniformLocation(cardFinish, "uAngle"),
      },
    };
  }

  uploadGpuTexture(url: string, img: HTMLImageElement): void {
    const g = this.gpu;
    if (!g || this.gpuTextures.has(url)) return;
    const w = Math.max(1, img.naturalWidth || img.width || 1);
    const h = Math.max(1, img.naturalHeight || img.height || 1);
    const mipLevelCount = Math.floor(Math.log2(Math.max(w, h))) + 1;
    const texture = g.device.createTexture({
      size: [w, h, 1],
      mipLevelCount,
      format: "rgba8unorm",
      usage:
        GPUTextureUsage.TEXTURE_BINDING |
        GPUTextureUsage.COPY_DST |
        GPUTextureUsage.RENDER_ATTACHMENT,
    });
    g.device.queue.copyExternalImageToTexture({ source: img }, { texture, premultipliedAlpha: true }, [w, h]);
    if (mipLevelCount > 1) {
      const encoder = g.device.createCommandEncoder();
      for (let level = 1; level < mipLevelCount; level++) {
        const srcView = texture.createView({ baseMipLevel: level - 1, mipLevelCount: 1 });
        const dstView = texture.createView({ baseMipLevel: level, mipLevelCount: 1 });
        const bg = g.device.createBindGroup({
          layout: g.mipLayout,
          entries: [
            { binding: 0, resource: srcView },
            { binding: 1, resource: g.sampler },
          ],
        });
        const pass = encoder.beginRenderPass({
          colorAttachments: [{ view: dstView, loadOp: "clear", storeOp: "store" }],
        });
        pass.setPipeline(g.mipPipe);
        pass.setBindGroup(0, bg);
        pass.draw(3);
        pass.end();
      }
      g.device.queue.submit([encoder.finish()]);
    }
    const bindGroup = g.device.createBindGroup({
      layout: g.texLayout,
      entries: [
        { binding: 0, resource: texture.createView() },
        { binding: 1, resource: g.sampler },
      ],
    });
    this.gpuTextures.set(url, { texture, bindGroup });
  }

  uploadGlTexture(url: string, img: HTMLImageElement): void {
    const gl = this.gl;
    if (!gl || this.glTextures.has(url)) return;
    const tex = gl.createTexture();
    if (!tex) return;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    if (this.glAnisoExt && this.glMaxAniso > 1) {
      gl.texParameterf(gl.TEXTURE_2D, this.glAnisoExt.TEXTURE_MAX_ANISOTROPY_EXT, this.glMaxAniso);
    }
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.generateMipmap(gl.TEXTURE_2D);
    this.glTextures.set(url, tex);
  }

  loadImage(url: string): ImageRecord | null {
    if (!url) return null;
    let rec = this.images.get(url);
    if (rec) return rec;
    const img = new Image();
    rec = { img, ready: false };
    img.crossOrigin = "anonymous";
    img.decoding = "async";
    img.addEventListener("load", () => {
      if (rec) rec.ready = true;
      if (this.gpu) this.uploadGpuTexture(url, img);
      if (this.gl) this.uploadGlTexture(url, img);
      this.invalidateAll();
      this.flushVisible(true);
    });
    img.src = url;
    this.images.set(url, rec);
    return rec;
  }

  ensureImageForCanvas(canvas: HTMLCanvasElement): void {
    const kind = canvas.dataset.rankKind;
    if (kind === "badge" || kind === "frame") {
      const key = assetKey(
        kind,
        canvas.dataset.rankTier,
        canvas.dataset.rankDiv,
        canvas.dataset.rankPodium,
        canvas.dataset.rankTrack,
      );
      if (key && this.urls[key]) this.loadImage(this.urls[key]);
    }
  }

  private syncCssSize(
    canvas: HTMLCanvasElement,
    m: CanvasMeta,
    boxW: number,
    boxH: number,
  ): boolean {
    if (boxW <= 0 || boxH <= 0) return false;
    const expandedToBacking =
      m.backingW > 0 &&
      m.backingH > 0 &&
      Math.abs(boxW - m.backingW) <= 1 &&
      Math.abs(boxH - m.backingH) <= 1 &&
      (m.backingW > Math.round(m.cssW) + 1 || m.backingH > Math.round(m.cssH) + 1);
    if (expandedToBacking) {
      // Unconstrained canvas (`width/height: auto`) resized its layout box to the
      // DPR-scaled bitmap; pin CSS size to the true pre-DPR layout box instead.
      canvas.style.inlineSize = `${m.cssW}px`;
      canvas.style.blockSize = `${m.cssH}px`;
      return false;
    }
    if (Math.abs(m.cssW - boxW) > 0.25 || Math.abs(m.cssH - boxH) > 0.25) {
      m.cssW = boxW;
      m.cssH = boxH;
      m.lastKey = "";
      return true;
    }
    return false;
  }

  ensureMeta(canvas: HTMLCanvasElement): CanvasMeta {
    let m = this.meta.get(canvas);
    if (!m) {
      const rect = canvas.getBoundingClientRect();
      const attrW = Number(canvas.getAttribute("width")) || 0;
      const attrH = Number(canvas.getAttribute("height")) || 0;
      m = {
        cssW: rect.width || canvas.clientWidth || attrW || 40,
        cssH: rect.height || canvas.clientHeight || attrH || 40,
        backingW: 0,
        backingH: 0,
        lastKey: "",
        lastDrawMs: 0,
        hoverUntil: 0,
        gpuCtx: null,
        gpuConfigured: false,
        ctx2d: null,
        backendTag: "",
        lit: false,
      };
      this.meta.set(canvas, m);
    }
    return m;
  }

  markIfVisible(canvas: HTMLCanvasElement): boolean {
    const rect = canvas.getBoundingClientRect();
    const m = this.ensureMeta(canvas);
    this.syncCssSize(canvas, m, rect.width, rect.height);
    const vh = window.innerHeight || 900;
    const vw = window.innerWidth || 1200;
    const inView =
      rect.width > 0 &&
      rect.height > 0 &&
      rect.bottom >= -120 &&
      rect.top <= vh + 120 &&
      rect.right >= -120 &&
      rect.left <= vw + 120;
    if (inView) {
      this.visible.add(canvas);
    }
    return inView;
  }

  registerRankCanvas(canvas: HTMLCanvasElement, newlyVisible: HTMLCanvasElement[]): void {
    if (this.items.has(canvas)) return;
    this.items.add(canvas);
    this.ensureMeta(canvas);
    this.ensureImageForCanvas(canvas);
    if (this.ro) this.ro.observe(canvas);
    if (this.io) this.io.observe(canvas);
    else this.visible.add(canvas);
    if (this.markIfVisible(canvas)) newlyVisible.push(canvas);
  }

  registerCardCanvas(canvas: HTMLCanvasElement, newlyVisible: HTMLCanvasElement[]): void {
    if (this.items.has(canvas)) return;
    this.items.add(canvas);
    this.ensureMeta(canvas);
    const frame = canvas.parentElement;
    if (frame && !frame.dataset.tiltBound) {
      frame.dataset.tiltBound = "1";
      frame.addEventListener(
        "pointermove",
        (ev) => {
          const target = frame.querySelector<HTMLCanvasElement>("canvas[data-card-finish]");
          if (!target) return;
          const rect = frame.getBoundingClientRect();
          if (rect.width <= 0) return;
          const nx = Math.max(-1, Math.min(1, ((ev.clientX - rect.left) / rect.width) * 2 - 1));
          target.dataset.tiltAngle = nx.toFixed(3);
          const m = this.meta.get(target);
          if (m) {
            m.lastKey = "";
            m.hoverUntil = performance.now() + 450;
          }
          this.kick(true);
        },
        { passive: true },
      );
      frame.addEventListener(
        "pointerleave",
        () => {
          const target = frame.querySelector<HTMLCanvasElement>("canvas[data-card-finish]");
          if (target) {
            delete target.dataset.tiltAngle;
            const m = this.meta.get(target);
            if (m) {
              m.lastKey = "";
              m.hoverUntil = 0;
            }
            this.kick(true);
          }
        },
        { passive: true },
      );
    }
    if (this.ro) this.ro.observe(canvas);
    if (this.io) this.io.observe(canvas);
    else this.visible.add(canvas);
    if (this.markIfVisible(canvas)) newlyVisible.push(canvas);
  }

  scan(root: Document | Element = document): void {
    if (root === document) this.urls = rankUrls();
    const newVisible: HTMLCanvasElement[] = [];
    for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-rank-kind]")) {
      this.registerRankCanvas(canvas, newVisible);
    }
    for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-card-finish]")) {
      this.registerCardCanvas(canvas, newVisible);
    }
    if (root instanceof HTMLCanvasElement) {
      if (root.dataset.rankKind) this.registerRankCanvas(root, newVisible);
      else if (root.dataset.cardFinish) this.registerCardCanvas(root, newVisible);
    }
    this.refreshAnimatedFlag();
    if (newVisible.length && !this.gpuPending) {
      const nowMs = performance.now();
      this.renderBatch(newVisible, nowMs);
    }
    this.kick(false);
  }

  scanElements(elements: readonly Element[]): void {
    if (!elements.length) return;
    const newVisible: HTMLCanvasElement[] = [];
    for (const root of elements) {
      if (!root.isConnected) continue;
      if (root instanceof HTMLCanvasElement) {
        if (root.dataset.rankKind) this.registerRankCanvas(root, newVisible);
        else if (root.dataset.cardFinish) this.registerCardCanvas(root, newVisible);
        continue;
      }
      for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-rank-kind]")) {
        this.registerRankCanvas(canvas, newVisible);
      }
      for (const canvas of root.querySelectorAll<HTMLCanvasElement>("canvas[data-card-finish]")) {
        this.registerCardCanvas(canvas, newVisible);
      }
    }
    this.refreshAnimatedFlag();
    if (newVisible.length && !this.gpuPending) {
      this.renderBatch(newVisible, performance.now());
    }
    this.kick(false);
  }

  prune(): void {
    for (const canvas of this.items) {
      if (!canvas.isConnected) {
        const m = this.meta.get(canvas);
        if (m?.gpuConfigured && m.gpuCtx) {
          m.gpuCtx.unconfigure();
          m.gpuConfigured = false;
        }
        this.items.delete(canvas);
        this.visible.delete(canvas);
        if (this.io) this.io.unobserve(canvas);
        if (this.ro) this.ro.unobserve(canvas);
      }
    }
    this.refreshAnimatedFlag();
  }

  isAnimated(canvas: HTMLCanvasElement): boolean {
    if (this.reduced || (!this.gpu && !this.gl)) return false;
    if (canvas.dataset.cardFinish === "foil" || canvas.dataset.cardFinish === "etched") return true;
    if (canvas.dataset.rankKind === "insignia") {
      return (Number(canvas.dataset.rankTier) || 0) >= 1;
    }
    const tier = Math.max(0, Math.min(11, Number(canvas.dataset.rankTier) || 0));
    return TIERS[tier][0] > 0;
  }

  refreshAnimatedFlag(): void {
    if (this.reduced || (!this.gpu && !this.gl)) {
      this.hasAnimatedVisible = false;
      return;
    }
    for (const c of this.visible) {
      if (c.isConnected && this.isAnimated(c)) {
        this.hasAnimatedVisible = true;
        return;
      }
    }
    this.hasAnimatedVisible = false;
  }

  prepareDescriptor(canvas: HTMLCanvasElement, nowMs: number, nowSec: number, scrolling: boolean): RenderJob | null {
    const m = this.ensureMeta(canvas);
    const kind = canvas.dataset.rankKind || "";
    const finishAttr = canvas.dataset.cardFinish || "";

    const baseDpr = Math.min(window.devicePixelRatio || 1, 2);
    const dpr = finishAttr ? Math.min(baseDpr, 1.25) : baseDpr;
    const maxW = finishAttr ? 320 : 384;
    const maxH = finishAttr ? 448 : 384;
    const w = Math.max(24, Math.min(maxW, Math.round(m.cssW * dpr)));
    const h = Math.max(24, Math.min(maxH, Math.round(m.cssH * dpr)));
    const resized = canvas.width !== w || canvas.height !== h;
    if (resized) {
      canvas.width = w;
      canvas.height = h;
      m.backingW = w;
      m.backingH = h;
      m.lastKey = "";
    }

    const animated = this.isAnimated(canvas);
    const firstPaint = m.lastKey === "";

    // Scroll pause: skip ambient redraws while scrolling, never skip unpainted canvases.
    if (scrolling && !firstPaint && nowMs >= m.hoverUntil) return null;

    // Adaptive FPS pacing per element category
    const intervalMs = finishAttr
      ? nowMs < m.hoverUntil
        ? 16
        : 33
      : m.cssW < 64 && kind !== "insignia"
        ? 50
        : 33;
    if (animated && !firstPaint && m.lastDrawMs > 0 && nowMs - m.lastDrawMs < intervalMs) {
      return null;
    }

    if (kind === "insignia") {
      const motif = canvas.dataset.rankMotif || "daily";
      const mInfo = INSIGNIA_MOTIFS[motif] || INSIGNIA_MOTIFS.daily;
      const tier = Math.max(0, Math.min(5, Number(canvas.dataset.rankTier) || 0));
      const visualKey = animated
        ? `${w}x${h}|ins|${tier}|${motif}|${Math.round(nowSec * 30)}`
        : `${w}x${h}|ins|${tier}|${motif}|s`;
      if (!animated && m.lastKey === visualKey) return null;
      return {
        canvas,
        meta: m,
        visualKey,
        w,
        h,
        intervalMs,
        pipeline: "insignia",
        nowSec,
        energy: this.reduced ? 0.0 : 1.0,
        tier,
        motifIdx: mInfo[0],
        metal: INSIGNIA_TIER_RGB[tier],
        gem: mInfo[1],
      };
    }

    if (finishAttr) {
      const finish = finishAttr === "etched" ? 2.0 : 1.0;
      const pointerTilt = Number(canvas.dataset.tiltAngle);
      const hasTilt = Number.isFinite(pointerTilt);
      const angle = hasTilt ? pointerTilt : this.reduced ? 0.524 : 0.44 * Math.sin(nowSec * 0.55);
      const t = this.reduced ? 1.2 : nowSec;
      const visualKey = animated
        ? `${w}x${h}|cf|${finish}|${hasTilt ? pointerTilt.toFixed(3) : Math.round(t * 30)}`
        : `${w}x${h}|cf|${finish}|s`;
      if (!animated && m.lastKey === visualKey) return null;
      return {
        canvas,
        meta: m,
        visualKey,
        w,
        h,
        intervalMs,
        pipeline: "cardFinish",
        t,
        finish,
        angle,
      };
    }

    const tier = Math.max(0, Math.min(11, Number(canvas.dataset.rankTier) || 0));
    const div = Math.max(0, Math.min(4, Number(canvas.dataset.rankDiv) || 0));
    const podium = Math.max(0, Math.min(3, Number(canvas.dataset.rankPodium) || 0));
    const track = Number(canvas.dataset.rankTrack) === 1 ? 1 : 0;
    const [period, mRgb, gRgb] = TIERS[tier];
    const [metal255, gem255] = tier === 10 && PODIUM[podium] ? PODIUM[podium] : [mRgb, gRgb];
    const phase = this.reduced || period <= 0 ? 0.34 : (nowSec % period) / period;
    const aKey = assetKey(kind, tier, div, podium, track);
    const url = aKey ? (this.urls[aKey] ?? null) : null;
    const rec = url ? this.images.get(url) : null;
    const texReady = rec?.ready ? 1 : 0;
    const qPhase = animated ? Math.round(phase * 256) : "s";
    const visualKey = `${w}x${h}|r|${kind}|${tier}|${div}|${podium}|${track}|${texReady}|${qPhase}`;
    if (!animated && m.lastKey === visualKey) return null;

    return {
      canvas,
      meta: m,
      visualKey,
      w,
      h,
      intervalMs,
      pipeline: "rank",
      kind,
      phase,
      tier,
      div,
      podium,
      track,
      url,
      metal: [metal255[0] / 255, metal255[1] / 255, metal255[2] / 255],
      gem: [gem255[0] / 255, gem255[1] / 255, gem255[2] / 255],
    };
  }

  ensureGpuCanvasCtx(s: RenderJob): GPUCanvasContext | null {
    const g = this.gpu;
    if (!g) return null;
    const m = s.meta;
    if (!m.gpuCtx) {
      m.gpuCtx = s.canvas.getContext("webgpu") as GPUCanvasContext | null;
    }
    if (!m.gpuCtx) return null;
    if (!m.gpuConfigured) {
      m.gpuCtx.configure({
        device: g.device,
        format: g.format,
        alphaMode: "premultiplied",
        usage: g.canvasUsage,
      });
      m.gpuConfigured = true;
    }
    return m.gpuCtx;
  }

  getGpuScratch(w: number, h: number): GpuScratchEntry | null {
    const g = this.gpu;
    if (!g) return null;
    const key = `${w}x${h}`;
    const existing = g.scratchPool.get(key);
    if (existing) {
      g.scratchPool.delete(key);
      g.scratchPool.set(key, existing);
      return existing;
    }
    if (g.scratchPool.size >= MAX_SCRATCH_TEXTURES) {
      const oldestKey = g.scratchPool.keys().next().value;
      if (oldestKey !== undefined) {
        g.scratchPool.get(oldestKey)?.texture.destroy();
        g.scratchPool.delete(oldestKey);
      }
    }
    const texture = g.device.createTexture({
      size: [w, h, 1],
      format: g.format,
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC,
    });
    const entry: GpuScratchEntry = { texture, view: texture.createView() };
    g.scratchPool.set(key, entry);
    return entry;
  }

  encodeGpuDraw(
    encoder: GPUCommandEncoder,
    targetView: GPUTextureView,
    slotIndex: number,
    s: RenderJob,
  ): void {
    const g = this.gpu;
    if (!g) return;
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: targetView,
          clearValue: { r: 0, g: 0, b: 0, a: 0 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });
    pass.setViewport(0, 0, s.w, s.h, 0, 1);
    pass.setScissorRect(0, 0, s.w, s.h);
    if (s.pipeline === "insignia") {
      pass.setPipeline(g.insignia);
      pass.setBindGroup(0, g.uBindGroup, [slotIndex * UNIFORM_STRIDE_BYTES]);
    } else if (s.pipeline === "cardFinish") {
      pass.setPipeline(g.cardFinish);
      pass.setBindGroup(0, g.uBindGroup, [slotIndex * UNIFORM_STRIDE_BYTES]);
    } else {
      const texEntry = s.url ? this.gpuTextures.get(s.url) : null;
      pass.setPipeline(g.rank);
      pass.setBindGroup(0, g.uBindGroup, [slotIndex * UNIFORM_STRIDE_BYTES]);
      pass.setBindGroup(1, texEntry ? texEntry.bindGroup : g.dummyBindGroup);
    }
    pass.draw(3);
    pass.end();
  }

  renderGpu(jobs: readonly RenderJob[], nowMs: number): void {
    const g = this.gpu;
    if (!g) return;

    // Group jobs by visualKey so identical tier/size targets share 1 shader pass.
    const groups = new Map<string, RenderJob[]>();
    for (const s of jobs) {
      let arr = groups.get(s.visualKey);
      if (!arr) {
        arr = [];
        groups.set(s.visualKey, arr);
      }
      arr.push(s);
    }

    const stage = this.uniformStage;
    let slotCount = 0;
    let encoder: GPUCommandEncoder | null = null;
    const submitChunk = (): void => {
      if (!encoder || slotCount === 0) return;
      g.device.queue.writeBuffer(g.uBuffer, 0, stage, 0, slotCount * UNIFORM_STRIDE_FLOATS);
      g.device.queue.submit([encoder.finish()]);
      slotCount = 0;
      encoder = null;
    };

    for (const group of groups.values()) {
      const rep = group[0];
      const hasGpuTex = rep.pipeline === "rank" && Boolean(rep.url && this.gpuTextures.has(rep.url));

      if (group.length >= 2 && g.canCopyDst) {
        const scratch = this.getGpuScratch(rep.w, rep.h);
        if (scratch) {
          if (slotCount >= MAX_SLOTS) submitChunk();
          encoder ??= g.device.createCommandEncoder();
          writeSlotUniforms(stage, slotCount * UNIFORM_STRIDE_FLOATS, rep, hasGpuTex);
          this.encodeGpuDraw(encoder, scratch.view, slotCount, rep);
          slotCount++;
          for (const targetJob of group) {
            const ctx = this.ensureGpuCanvasCtx(targetJob);
            if (!ctx || targetJob.canvas.width === 0 || targetJob.canvas.height === 0) continue;
            const dstTex = ctx.getCurrentTexture();
            encoder.copyTextureToTexture(
              { texture: scratch.texture },
              { texture: dstTex },
              [rep.w, rep.h, 1],
            );
            targetJob.meta.lastKey = targetJob.visualKey;
            targetJob.meta.lastDrawMs = nowMs;
            this.markDomState(targetJob, "webgpu");
          }
          continue;
        }
      }

      for (const targetJob of group) {
        const ctx = this.ensureGpuCanvasCtx(targetJob);
        if (!ctx || targetJob.canvas.width === 0 || targetJob.canvas.height === 0) continue;
        if (slotCount >= MAX_SLOTS) submitChunk();
        encoder ??= g.device.createCommandEncoder();
        writeSlotUniforms(stage, slotCount * UNIFORM_STRIDE_FLOATS, targetJob, hasGpuTex);
        this.encodeGpuDraw(encoder, ctx.getCurrentTexture().createView(), slotCount, targetJob);
        slotCount++;
        targetJob.meta.lastKey = targetJob.visualKey;
        targetJob.meta.lastDrawMs = nowMs;
        this.markDomState(targetJob, "webgpu");
      }
    }

    submitChunk();
  }

  renderGl(jobs: readonly RenderJob[], nowMs: number): void {
    const groups = new Map<string, RenderJob[]>();
    for (const s of jobs) {
      let arr = groups.get(s.visualKey);
      if (!arr) {
        arr = [];
        groups.set(s.visualKey, arr);
      }
      arr.push(s);
    }

    const reps = [...groups.values()];
    let idx = 0;
    while (idx < reps.length) {
      const chunk: PackedJob[] = [];
      const chunkGroups: RenderJob[][] = [];
      let sx = 0;
      let sy = 0;
      let rowH = 0;
      while (idx < reps.length && chunk.length < MAX_SLOTS) {
        const group = reps[idx];
        const s = group[0];
        const rw = Math.min(ATLAS_WIDTH, s.w);
        const rh = Math.min(ATLAS_MAX_HEIGHT, s.h);
        if (sx + rw > ATLAS_WIDTH) {
          sy += rowH;
          sx = 0;
          rowH = 0;
        }
        if (sy + rh > ATLAS_MAX_HEIGHT && chunk.length > 0) break;
        chunk.push({ job: s, x: sx, y: sy, rw, rh });
        chunkGroups.push(group);
        sx += rw;
        if (rh > rowH) rowH = rh;
        idx++;
      }
      const totalH = Math.max(256, Math.min(ATLAS_MAX_HEIGHT, Math.ceil((sy + rowH) / 256) * 256));
      this.flushGlChunk(chunk, chunkGroups, totalH, nowMs);
    }
  }

  renderBatch(canvases: Iterable<HTMLCanvasElement>, nowMs: number): void {
    if (this.gpuPending) return;
    const nowSec = (nowMs - this.start) * 0.001;
    const scrolling = nowMs < this.scrollingUntil;
    const jobs: RenderJob[] = [];
    for (const c of canvases) {
      if (!c.isConnected) continue;
      const d = this.prepareDescriptor(c, nowMs, nowSec, scrolling);
      if (d) jobs.push(d);
    }
    if (!jobs.length) return;

    if (this.gpu) {
      this.renderGpu(jobs, nowMs);
      return;
    }

    if (this.gl) {
      this.renderGl(jobs, nowMs);
      return;
    }

    for (const s of jobs) {
      this.renderFallback2D(s);
      s.meta.lastKey = s.visualKey;
      s.meta.lastDrawMs = nowMs;
      this.markDomState(s, "2d");
    }
  }

  flushGlChunk(
    chunk: readonly PackedJob[],
    chunkGroups: readonly (readonly RenderJob[])[],
    neededH: number,
    nowMs: number,
  ): void {
    const gl = this.gl;
    const glCanvas = this.glCanvas;
    if (!gl || !glCanvas || !this.glPrograms) return;
    if (glCanvas.height < neededH) {
      glCanvas.height = neededH;
    }
    const ch = glCanvas.height;
    gl.disable(gl.SCISSOR_TEST);
    gl.viewport(0, 0, glCanvas.width, ch);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.SCISSOR_TEST);
    gl.bindVertexArray(this.glPrograms.vao);

    for (const packed of chunk) {
      const s = packed.job;
      const glY = ch - packed.y - packed.rh;
      gl.scissor(packed.x, glY, packed.rw, packed.rh);
      gl.viewport(packed.x, glY, packed.rw, packed.rh);
      if (s.pipeline === "insignia") {
        const u = this.glPrograms.insignia;
        gl.useProgram(u.prog);
        gl.uniform1f(u.uTime, s.nowSec);
        gl.uniform1f(u.uEnergy, s.energy);
        gl.uniform1f(u.uTier, s.tier);
        gl.uniform1f(u.uMotif, s.motifIdx);
        gl.uniform3fv(u.uMetal, s.metal);
        gl.uniform3fv(u.uGem, s.gem);
      } else if (s.pipeline === "cardFinish") {
        const u = this.glPrograms.cardFinish;
        gl.useProgram(u.prog);
        gl.uniform1f(u.uTime, s.t);
        gl.uniform1f(u.uFinish, s.finish);
        gl.uniform1f(u.uAngle, s.angle);
      } else {
        const u = this.glPrograms.rank;
        const tex = s.url ? this.glTextures.get(s.url) : null;
        gl.useProgram(u.prog);
        if (tex) {
          gl.activeTexture(gl.TEXTURE0);
          gl.bindTexture(gl.TEXTURE_2D, tex);
          gl.uniform1i(u.uTex, 0);
          gl.uniform1f(u.uHasTex, 1.0);
        } else {
          gl.uniform1f(u.uHasTex, 0.0);
        }
        gl.uniform1f(u.uPhase, s.phase);
        gl.uniform1f(u.uTier, s.tier);
        gl.uniform1f(u.uDiv, s.div);
        gl.uniform1f(u.uPodium, s.podium);
        gl.uniform1f(u.uKind, s.kind === "frame" ? 1.0 : 0.0);
        gl.uniform1f(u.uTrack, s.track);
        gl.uniform3fv(u.uMetal, s.metal);
        gl.uniform3fv(u.uGem, s.gem);
      }
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }
    gl.disable(gl.SCISSOR_TEST);

    for (let i = 0; i < chunk.length; i++) {
      const packed = chunk[i];
      const group = chunkGroups[i];
      for (const target of group) {
        target.meta.ctx2d ??= target.canvas.getContext("2d");
        const ctx = target.meta.ctx2d;
        if (ctx) {
          ctx.clearRect(0, 0, target.w, target.h);
          ctx.drawImage(glCanvas, packed.x, packed.y, packed.rw, packed.rh, 0, 0, target.w, target.h);
        }
        target.meta.lastKey = target.visualKey;
        target.meta.lastDrawMs = nowMs;
        this.markDomState(target, "webgl2");
      }
    }
  }

  renderFallback2D(s: RenderJob): void {
    const { canvas, w, h, meta } = s;
    meta.ctx2d ??= canvas.getContext("2d");
    const ctx = meta.ctx2d;
    if (!ctx) return;
    ctx.clearRect(0, 0, w, h);
    if (s.pipeline === "cardFinish") {
      const grad = ctx.createLinearGradient(0, 0, w, h);
      grad.addColorStop(0.0, "rgba(0, 0, 0, 0)");
      if (s.finish > 1.5) {
        grad.addColorStop(0.28, "rgba(102, 171, 209, 0.14)");
        grad.addColorStop(0.5, "rgba(245, 196, 110, 0.36)");
        grad.addColorStop(0.72, "rgba(102, 171, 209, 0.14)");
      } else {
        grad.addColorStop(0.22, "rgba(110, 210, 255, 0.18)");
        grad.addColorStop(0.5, "rgba(255, 224, 130, 0.30)");
        grad.addColorStop(0.78, "rgba(196, 130, 255, 0.18)");
      }
      grad.addColorStop(1.0, "rgba(0, 0, 0, 0)");
      ctx.fillStyle = grad;
      ctx.fillRect(0, 0, w, h);
      return;
    }
    if (s.pipeline === "insignia") {
      const [mr, mg, mb] = s.metal;
      const [gr, gg, gb] = s.tier === 0 ? s.metal : s.gem;
      const dim = s.tier === 0 ? 0.55 : 1.0;
      const cx = w / 2;
      const cy = h / 2;
      const metalCol = `rgb(${Math.round(mr * 230 * dim)}, ${Math.round(mg * 230 * dim)}, ${Math.round(mb * 230 * dim)})`;
      const metalMid = `rgb(${Math.round(mr * 125 * dim)}, ${Math.round(mg * 125 * dim)}, ${Math.round(mb * 125 * dim)})`;
      const gemCol = `rgb(${Math.round(gr * 235 * dim)}, ${Math.round(gg * 235 * dim)}, ${Math.round(gb * 235 * dim)})`;
      if (s.tier >= 2) {
        ctx.beginPath();
        ctx.arc(cx, cy, w * 0.295, 0, Math.PI * 2);
        ctx.lineWidth = Math.max(1.5, w * 0.018);
        ctx.strokeStyle = metalCol;
        ctx.stroke();
      }
      ctx.beginPath();
      ctx.arc(cx, cy, w * 0.265, 0, Math.PI * 2);
      ctx.fillStyle = metalMid;
      ctx.fill();
      ctx.lineWidth = Math.max(2, w * 0.032);
      ctx.strokeStyle = metalCol;
      ctx.stroke();
      ctx.beginPath();
      ctx.arc(cx, cy, w * 0.22, 0, Math.PI * 2);
      ctx.fillStyle = "#080c12";
      ctx.fill();
      ctx.lineWidth = Math.max(1, w * 0.012);
      ctx.strokeStyle = metalMid;
      ctx.stroke();
      const d = w * 0.095;
      ctx.beginPath();
      ctx.moveTo(cx, cy - d);
      ctx.lineTo(cx + d * 0.78, cy);
      ctx.lineTo(cx, cy + d);
      ctx.lineTo(cx - d * 0.78, cy);
      ctx.closePath();
      ctx.fillStyle = gemCol;
      ctx.fill();
      return;
    }
    const rec = s.url ? this.images.get(s.url) : null;
    if (rec?.ready) {
      ctx.drawImage(rec.img, 0, 0, w, h);
    } else if (s.kind === "frame") {
      ctx.beginPath();
      ctx.arc(w / 2, h / 2, w * 0.45, 0, Math.PI * 2);
      ctx.lineWidth = Math.max(2, w * 0.06);
      ctx.strokeStyle = "#78858f";
      ctx.stroke();
    }
  }

  flushVisible(force = false): void {
    if (this.gpuPending) return;
    if (force) this.invalidateAll();
    this.renderBatch(this.visible, performance.now());
    this.kick(false);
  }

  kick(once = false): void {
    if (this.raf || this.gpuPending || (!once && !this.hasAnimatedVisible)) return;
    this.raf = requestAnimationFrame((ts) => {
      this.raf = 0;
      this.renderBatch(this.visible, ts);
      if (this.hasAnimatedVisible && !document.hidden) this.kick(false);
    });
  }
}

const engine = new BadgeEngine();
engine.scan(document);

const watcher = new MutationObserver((list) => {
  let removed = false;
  let attrChanged = false;
  for (const m of list) {
    if (m.type === "attributes") {
      if (m.target instanceof HTMLCanvasElement) {
        engine.ensureImageForCanvas(m.target);
        const meta = engine.meta.get(m.target);
        if (meta) meta.lastKey = "";
      }
      attrChanged = true;
      continue;
    }
    for (const node of m.addedNodes) {
      if (node instanceof Element) engine.pendingAdded.add(node);
    }
    if (m.removedNodes.length) removed = true;
  }
  if (removed) engine.prune();
  if (engine.pendingAdded.size && !engine.scanScheduled) {
    engine.scanScheduled = true;
    queueMicrotask(() => {
      engine.scanScheduled = false;
      const batch = [...engine.pendingAdded];
      engine.pendingAdded.clear();
      engine.scanElements(batch);
    });
  } else if (attrChanged) {
    engine.refreshAnimatedFlag();
    engine.kick(true);
  }
});
watcher.observe(document.documentElement, {
  childList: true,
  subtree: true,
  attributes: true,
  attributeFilter: [
    "data-card-finish",
    "data-rank-kind",
    "data-rank-tier",
    "data-rank-div",
    "data-rank-podium",
    "data-rank-track",
    "data-rank-motif",
  ],
});

document.addEventListener("baylee:page", () => {
  engine.prune();
  engine.scan(document);
});
window.addEventListener("baylee:badges", () => {
  engine.prune();
  engine.scan(document);
});
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) engine.kick(true);
});
