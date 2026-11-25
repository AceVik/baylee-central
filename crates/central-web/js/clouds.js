// GENERATED from crates/central-web/ts/clouds.ts via TypeScript 7.1 — do not edit directly.
// The start page's sky, alive: clouds that drift on a gusting wind and that
// the pointer stirs (Stam's stable fluids on WebGPU first, WebGL2 fallback).
const SUNLIGHT = { lit: [1, 0.99, 0.955], shade: [0.7, 0.75, 0.88], rim: [1, 0.9, 0.68], opacity: 0.9, room: 0.8, cover: 0.6 };
const BLUE_HOUR = { lit: [0.97, 0.97, 1], shade: [0.58, 0.64, 0.83], rim: [1, 0.82, 0.62], opacity: 0.8, room: 0.7, cover: 0.58 };
const AIR = 96;
const CLOUD = 224;
const JACOBI = 10;
const WIND = 0.0055;
const WISP_LIFE = 7;
const AIR_FADE = 0.42;
const REST = 0.012;
const GATHER = 0.3;
const DISSOLVE = 0.24;
const MEMORY = 2.4;
const THIN = 0.28;
const SWIRL = 30;
const LIFT = 4;
const SCALE = 0.5;
const MAX_WIDTH = 960;
const REACH = 0.034;
const SWEEP = 2.6;
const SWIFT = 3;
const FORCE = 0.8;
const GRIP = 0.045;
// ============================================================================
// WGSL Fluid + Volumetric Display Shaders (WebGPU Primary Backend)
// Note: vUv uses texture coords (0,0 top-left), converted via mathUv = (x, 1-y)
// and samp() -> textureSampleLevel(..., 0.0) so all derivatives & samples obey
// Dawn uniform control flow even with early discard returns.
// ============================================================================
const CLOUD_COMMON_WGSL = /* wgsl */ `
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

struct CloudUniforms {
  v0: vec4<f32>,
  v1: vec4<f32>,
  v2: vec4<f32>,
  v3: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: CloudUniforms;
@group(1) @binding(0) var t0: texture_2d<f32>;
@group(1) @binding(1) var t1: texture_2d<f32>;
@group(1) @binding(2) var t2: texture_2d<f32>;
@group(1) @binding(3) var sLin: sampler;

fn samp(tex: texture_2d<f32>, mUv: vec2<f32>) -> vec4<f32> {
  let c = clamp(vec2<f32>(mUv.x, 1.0 - mUv.y), vec2<f32>(0.0), vec2<f32>(1.0));
  return textureSampleLevel(tex, sLin, c, 0.0);
}

fn hash(p: vec2<f32>) -> f32 {
  var q = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
  q += dot(q, vec3<f32>(q.y, q.z, q.x) + vec3<f32>(33.33));
  return fract((q.x + q.y) * q.z);
}

fn noise(p: vec2<f32>) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let u2 = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  let a = hash(i);
  let b = hash(i + vec2<f32>(1.0, 0.0));
  let c = hash(i + vec2<f32>(0.0, 1.0));
  let d = hash(i + vec2<f32>(1.0, 1.0));
  return mix(mix(a, b, u2.x), mix(c, d, u2.x), u2.y);
}

fn sloped(p: vec2<f32>) -> vec3<f32> {
  let i = floor(p);
  let f = fract(p);
  let u2 = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
  let a = hash(i);
  let b = hash(i + vec2<f32>(1.0, 0.0));
  let c = hash(i + vec2<f32>(0.0, 1.0));
  let d = hash(i + vec2<f32>(1.0, 1.0));
  let k = a - b - c + d;
  return vec3<f32>(a + (b - a) * u2.x + (c - a) * u2.y + k * u2.x * u2.y, du * vec2<f32>(b - a + k * u2.y, c - a + k * u2.x));
}

fn fbm(pIn: vec2<f32>, octaves: i32) -> f32 {
  var p = pIn;
  let turn = mat2x2<f32>(vec2<f32>(0.8, 0.6), vec2<f32>(-0.6, 0.8));
  var sum = 0.0;
  var amp = 0.5;
  for (var i = 0; i < 4; i = i + 1) {
    if (i >= octaves) {
      break;
    }
    sum += amp * noise(p);
    p = turn * p * 2.03 + vec2<f32>(11.7, 4.3);
    amp *= 0.5;
  }
  return sum;
}

fn openAt(uv: vec2<f32>) -> f32 {
  return smoothstep(0.12, 0.92, length((uv - vec2<f32>(0.5, 0.58)) * vec2<f32>(1.05, 1.55)));
}

fn banksAt(uv: vec2<f32>, time: f32, aspect: f32, drift: f32, coverIn: f32) -> f32 {
  let room = openAt(uv);
  let cover = coverIn * mix(0.58, 1.0, room)
    + 0.07 * smoothstep(0.22, 0.5, abs(uv.x - 0.5))
    + 0.04 * smoothstep(0.4, 0.0, uv.y)
    - 0.05 * smoothstep(0.8, 1.0, uv.y);
  let p = vec2<f32>((uv.x - drift) * aspect, uv.y) * 3.1;
  let warp = vec2<f32>(
    fbm(p * 0.7 + vec2<f32>(0.0, time * 0.014), 2),
    fbm(p * 0.7 + vec2<f32>(5.2, 1.3) - time * 0.011, 2)
  );
  let cumulus = smoothstep(0.99 - cover, 1.21 - cover, fbm(p + 1.8 * warp, 4));
  let s = vec2<f32>((uv.x - drift * 1.25) * aspect, uv.y) * 6.7 + vec2<f32>(31.7, 8.1 + time * 0.006);
  let puffs = 0.62 * smoothstep(1.1 - cover, 1.33 - cover, fbm(s + 0.9 * warp, 2));
  let h = vec2<f32>((uv.x - drift * 0.8) * aspect * 1.3, uv.y * 4.6) + vec2<f32>(time * 0.004, 17.3) + 0.6 * warp;
  let streaks = 0.5 * smoothstep(0.38, 0.68, fbm(h, 2)) * smoothstep(0.2, 0.75, uv.y) * mix(0.45, 1.0, room);
  return 1.0 - (1.0 - cumulus) * (1.0 - puffs) * (1.0 - streaks);
}
`;
const DRAG_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let aspect = u.v0.z;
  let radiusSq = u.v0.w;
  let fromPt = u.v1.xy;
  let toPt = u.v1.zw;
  let value = u.v2.xy;
  let gain = u.v2.z;
  let scale = vec2<f32>(aspect, 1.0);
  let p = (uv - fromPt) * scale;
  let along = (toPt - fromPt) * scale;
  let off = p - along * clamp(dot(p, along) / max(dot(along, along), 1e-8), 0.0, 1.0);
  let v = samp(t0, uv).xy;
  return vec4<f32>(mix(v, value, exp(-dot(off, off) / radiusSq) * gain), 0.0, 1.0);
}`;
const GUST_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let aspect = u.v0.z;
  let pt = u.v1.xy;
  let radius = u.v1.z;
  let strength = u.v1.w;
  let p = (uv - pt) * vec2<f32>(aspect, 1.0);
  let r = length(p);
  let away = p / max(r, 1e-4);
  let ring = (r - radius) / (0.6 * radius);
  let push = away + 0.4 * sin(6.0 * atan2(p.y, p.x)) * vec2<f32>(-away.y, away.x);
  return vec4<f32>(samp(t0, uv).xy + strength * exp(-(ring * ring)) * push, 0.0, 1.0);
}`;
const ADVECT_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let texel = u.v0.xy;
  let wind = u.v0.z;
  let dt = u.v0.w;
  let keep = u.v1.x;
  let back = uv - dt * (samp(t0, uv).xy + vec2<f32>(wind, 0.0)) * texel;
  return vec4<f32>(samp(t0, back).xy * keep, 0.0, 1.0);
}`;
const DENSITY_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let grid = u.v0.zw;
  let wind = u.v1.x;
  let dt = u.v1.y;
  let forget = u.v1.z;
  let thin = u.v1.w;
  let time = u.v2.x;
  let aspect = u.v2.y;
  let drift = u.v2.z;
  let cover = u.v2.w;
  let gather = u.v3.x;
  let dissolve = u.v3.y;
  let v = samp(t0, uv).xy;
  let was = samp(t1, uv - dt * (v + vec2<f32>(wind, 0.0)) * grid);
  let speed = length(v) * grid.y;
  let kept = was.g * forget;
  let stirred = max(kept, smoothstep(0.03, 0.6, speed));
  var d = was.r * (1.0 - thin * (stirred - kept));
  let want = banksAt(uv, time, aspect, drift, cover);
  let calm = (1.0 - stirred) * (1.0 - stirred);
  let rate = select(dissolve, gather, want > d) * (0.3 + 0.7 * calm);
  d += (want - d) * (1.0 - exp(-rate * dt));
  return vec4<f32>(clamp(d, 0.0, 1.0), stirred, 0.0, 1.0);
}`;
const BANKS_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  return vec4<f32>(banksAt(uv, u.v0.z, u.v0.w, u.v1.x, u.v1.y), 0.0, 0.0, 1.0);
}`;
const CARRY_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let grid = u.v0.zw;
  let aspect = u.v1.x;
  let wind = u.v1.y;
  let dt = u.v1.z;
  let keep = u.v2.xy;
  let flow = samp(t0, uv).xy * grid.y;
  let back = uv - dt * (flow + vec2<f32>(wind, 0.0)) * vec2<f32>(1.0 / aspect, 1.0);
  let carried = samp(t1, back) - dt * flow.xyxy;
  return clamp(carried, vec4<f32>(-2.0), vec4<f32>(2.0)) * keep.xxyy;
}`;
const CURL_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let texel = u.v0.xy;
  let vL = uv - vec2<f32>(texel.x, 0.0);
  let vR = uv + vec2<f32>(texel.x, 0.0);
  let vT = uv + vec2<f32>(0.0, texel.y);
  let vB = uv - vec2<f32>(0.0, texel.y);
  let c = samp(t0, vR).y - samp(t0, vL).y - samp(t0, vT).x + samp(t0, vB).x;
  return vec4<f32>(0.5 * c, 0.0, 0.0, 1.0);
}`;
const FORCES_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let texel = u.v0.xy;
  let swirl = u.v0.z;
  let lift = u.v0.w;
  let dt = u.v1.x;
  let vL = uv - vec2<f32>(texel.x, 0.0);
  let vR = uv + vec2<f32>(texel.x, 0.0);
  let vT = uv + vec2<f32>(0.0, texel.y);
  let vB = uv - vec2<f32>(0.0, texel.y);
  let l = samp(t1, vL).x;
  let r = samp(t1, vR).x;
  let t = samp(t1, vT).x;
  let b = samp(t1, vB).x;
  let c = samp(t1, uv).x;
  var force = 0.5 * vec2<f32>(abs(t) - abs(b), abs(r) - abs(l));
  force /= length(force) + 1e-4;
  force *= swirl * c * vec2<f32>(1.0, -1.0);
  let cloud = samp(t2, uv).rg;
  force.y += lift * cloud.r * cloud.g;
  let v = samp(t0, uv).xy + force * dt;
  return vec4<f32>(clamp(v, vec2<f32>(-600.0), vec2<f32>(600.0)), 0.0, 1.0);
}`;
const DIVERGENCE_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let texel = u.v0.xy;
  let vL = uv - vec2<f32>(texel.x, 0.0);
  let vR = uv + vec2<f32>(texel.x, 0.0);
  let vT = uv + vec2<f32>(0.0, texel.y);
  let vB = uv - vec2<f32>(0.0, texel.y);
  let c = samp(t0, uv).xy;
  let l = select(samp(t0, vL).x, -c.x, vL.x < 0.0);
  let r = select(samp(t0, vR).x, -c.x, vR.x > 1.0);
  let t = select(samp(t0, vT).y, -c.y, vT.y > 1.0);
  let b = select(samp(t0, vB).y, -c.y, vB.y < 0.0);
  return vec4<f32>(0.5 * (r - l + t - b), 0.0, 0.0, 1.0);
}`;
const PRESSURE_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let texel = u.v0.xy;
  let keep = u.v0.z;
  let vL = uv - vec2<f32>(texel.x, 0.0);
  let vR = uv + vec2<f32>(texel.x, 0.0);
  let vT = uv + vec2<f32>(0.0, texel.y);
  let vB = uv - vec2<f32>(0.0, texel.y);
  let p = samp(t0, vL).x + samp(t0, vR).x + samp(t0, vT).x + samp(t0, vB).x;
  return vec4<f32>(0.25 * (keep * p - samp(t1, uv).x), 0.0, 0.0, 1.0);
}`;
const GRADIENT_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let texel = u.v0.xy;
  let vL = uv - vec2<f32>(texel.x, 0.0);
  let vR = uv + vec2<f32>(texel.x, 0.0);
  let vT = uv + vec2<f32>(0.0, texel.y);
  let vB = uv - vec2<f32>(0.0, texel.y);
  let l = samp(t0, vL).x;
  let r = samp(t0, vR).x;
  let t = samp(t0, vT).x;
  let b = samp(t0, vB).x;
  let v = samp(t1, uv).xy - 0.5 * vec2<f32>(r - l, t - b);
  return vec4<f32>(v, 0.0, 1.0);
}`;
const DISPLAY_WGSL = CLOUD_COMMON_WGSL + /* wgsl */ `
fn lobes(pIn: vec2<f32>, cell: f32, time: f32) -> vec4<f32> {
  var p = pIn;
  let turn = mat2x2<f32>(vec2<f32>(0.8, 0.6), vec2<f32>(-0.6, 0.8));
  let turnT = mat2x2<f32>(vec2<f32>(0.8, -0.6), vec2<f32>(0.6, 0.8));
  var sum = vec4<f32>(0.0);
  var amp = 0.5;
  var fine = 1.0;
  var home = mat2x2<f32>(vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0));
  for (var i = 0; i < 4; i = i + 1) {
    let keep = 1.0 - smoothstep(0.28, 0.62, cell * fine);
    if (keep > 0.0) {
      let n = sloped(p);
      let side = 2.0 * n.x - 1.0;
      let fold = sqrt(side * side + 0.03);
      if (i < 3) {
        sum += amp * vec4<f32>(mix(0.5, n.x, keep), mix(0.434, fold, keep), keep * fine * 2.0 * side / fold * (home * n.yz));
      } else {
        sum += amp * vec4<f32>(mix(0.5, n.x, keep), 0.7 * mix(0.5, n.x, keep), 0.7 * keep * fine * (home * n.yz));
      }
    } else {
      sum = vec4<f32>(sum.xy + amp * select(vec2<f32>(0.5, 0.35), vec2<f32>(0.5, 0.434), i < 3), sum.zw);
    }
    p = turn * p * 2.03 + vec2<f32>(11.7, 4.3) + time * vec2<f32>(0.03, -0.018);
    home = 0.62 * home * turnT;
    fine *= 2.03;
    amp *= 0.5;
  }
  return sum;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let time = u.v0.z;
  let aspect = u.v0.w;
  let drift = u.v1.x;
  let phase = u.v1.yz;
  let opacity = u.v1.w;
  let uLit = u.v2.xyz;
  let room = u.v2.w;
  let uShade = u.v3.xyz;
  let pixel = vec2<f32>(u.v0.x, u.v0.y);
  let uRim = u.v3.w;
  // Derive rim tint neatly from SUNLIGHT/BLUE_HOUR warmth factor passed in v3.w or reuse uLit/uShade
  let carry = samp(t1, uv);
  let q = vec2<f32>((uv.x - drift) * aspect, uv.y) * 7.0 + vec2<f32>(time * 0.02, 0.0);
  let qa = q + carry.xy * 7.0;
  let qb = q + carry.zw * 7.0;
  let ca = max(length(dpdx(qa)), length(dpdy(qa)));
  let cb = max(length(dpdx(qb)), length(dpdy(qb)));

  let cloud = samp(t0, uv);
  let base = cloud.r;
  if (base < 0.05) {
    return vec4<f32>(0.0);
  }
  let apart = (carry.xy - carry.zw) * 7.0;
  let apartSq = dot(apart, apart);
  let alike = exp(-1.6 * apartSq);
  let w = phase * inverseSqrt(dot(phase, phase) + 2.0 * phase.x * phase.y * alike);
  let MEAN = vec4<f32>(0.4514, 0.3958, 0.0, 0.0);
  var n = lobes(qa, ca, time) - MEAN;
  if (w.y >= 0.015 && apartSq >= 0.0005) {
    n = w.x * n + w.y * (lobes(qb, cb, time) - MEAN);
  }
  let stirred = cloud.g;
  let relief = mix(1.0, 0.35, stirred);
  let shape = relief * 1.25 * n.y + (1.0 - 0.6 * relief) * n.x;
  let h = base + shape * (0.42 + 0.5 * base) * smoothstep(0.0, 0.25, base);
  let edge = mix(0.2, 0.4, stirred);
  let d = smoothstep(0.3 - 0.5 * edge, 0.3 + 0.5 * edge, h);
  if (d <= 0.002) {
    return vec4<f32>(0.0);
  }
  let e = pixel * 7.0;
  let lean = vec2<f32>(
    samp(t0, uv + vec2<f32>(e.x, 0.0)).r - samp(t0, uv - vec2<f32>(e.x, 0.0)).r,
    samp(t0, uv + vec2<f32>(0.0, e.y)).r - samp(t0, uv - vec2<f32>(0.0, e.y)).r
  );
  let normal = normalize(vec3<f32>(-2.2 * lean - 0.5 * relief * n.zw, 1.0));
  let toward = normalize(vec2<f32>(0.5, 1.2) - uv);
  let sun = normalize(vec3<f32>(toward * 0.8, 0.6));
  let diffuse = clamp(dot(normal, sun), 0.0, 1.0);
  var depth = 0.0;
  for (var i = 1; i <= 4; i = i + 1) {
    depth += samp(t0, uv + toward * 0.025 * f32(i)).r;
  }
  let light = exp(-depth * 0.42);
  let lit = clamp(0.54 + 0.42 * smoothstep(0.2, 1.0, light) + 0.35 * (diffuse - 0.6) + 0.24 * relief * shape, 0.0, 1.0);
  let rimCol = vec3<f32>(1.0, mix(0.82, 0.90, uRim), mix(0.62, 0.68, uRim));
  var c = mix(uShade, uLit, lit);
  c += rimCol * pow(1.0 - d, 1.6) * light * (0.3 + 0.5 * diffuse);
  let thick = smoothstep(0.3, 0.9, h);
  let a = d * opacity * mix(0.74, 1.0, thick) * mix(room, 1.0, openAt(uv));
  let grain = (hash(in.pos.xy + fract(time) * 61.0) - 0.5) / 255.0;
  return max(vec4<f32>(c * a, a) + grain, vec4<f32>(0.0));
}`;
// ============================================================================
// GLSL ES 3.00 Shaders (WebGL2 Fallback Backend — 4-octave optimized)
// ============================================================================
const HEAD = "#version 300 es\nprecision highp float;\nprecision highp sampler2D;\n";
const VERTEX = `${HEAD}
in vec2 aPosition;
uniform vec2 uTexel;
out vec2 vUv;
out vec2 vL;
out vec2 vR;
out vec2 vT;
out vec2 vB;
void main() {
  vUv = aPosition * 0.5 + 0.5;
  vL = vUv - vec2(uTexel.x, 0.0);
  vR = vUv + vec2(uTexel.x, 0.0);
  vT = vUv + vec2(0.0, uTexel.y);
  vB = vUv - vec2(0.0, uTexel.y);
  gl_Position = vec4(aPosition, 0.0, 1.0);
}`;
const NOISE = `
float hash(vec2 p) {
  vec3 q = fract(vec3(p.xyx) * 0.1031);
  q += dot(q, q.yzx + 33.33);
  return fract((q.x + q.y) * q.z);
}
float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
vec3 sloped(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  vec2 du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  float k = a - b - c + d;
  return vec3(a + (b - a) * u.x + (c - a) * u.y + k * u.x * u.y, du * vec2(b - a + k * u.y, c - a + k * u.x));
}
const mat2 TURN = mat2(0.8, -0.6, 0.6, 0.8);
float fbm(vec2 p, int octaves) {
  float sum = 0.0;
  float amp = 0.5;
  for (int i = 0; i < 4; i++) {
    if (i >= octaves) break;
    sum += amp * noise(p);
    p = TURN * p * 2.03 + vec2(11.7, 4.3);
    amp *= 0.5;
  }
  return sum;
}
`;
const OPEN = `
float open(vec2 uv) {
  return smoothstep(0.12, 0.92, length((uv - vec2(0.5, 0.58)) * vec2(1.05, 1.55)));
}
`;
const BANKS_AT = `${NOISE}${OPEN}
uniform float uTime;
uniform float uAspect;
uniform float uDrift;
uniform float uCover;
float banks(vec2 uv) {
  float room = open(uv);
  float cover = uCover * mix(0.58, 1.0, room) + 0.07 * smoothstep(0.22, 0.5, abs(uv.x - 0.5)) + 0.04 * smoothstep(0.4, 0.0, uv.y) - 0.05 * smoothstep(0.8, 1.0, uv.y);
  vec2 p = vec2((uv.x - uDrift) * uAspect, uv.y) * 3.1;
  vec2 warp = vec2(fbm(p * 0.7 + vec2(0.0, uTime * 0.014), 2), fbm(p * 0.7 + vec2(5.2, 1.3) - uTime * 0.011, 2));
  float cumulus = smoothstep(0.99 - cover, 1.21 - cover, fbm(p + 1.8 * warp, 4));
  vec2 s = vec2((uv.x - uDrift * 1.25) * uAspect, uv.y) * 6.7 + vec2(31.7, 8.1 + uTime * 0.006);
  float puffs = 0.62 * smoothstep(1.1 - cover, 1.33 - cover, fbm(s + 0.9 * warp, 2));
  vec2 h = vec2((uv.x - uDrift * 0.8) * uAspect * 1.3, uv.y * 4.6) + vec2(uTime * 0.004, 17.3) + 0.6 * warp;
  float streaks = 0.5 * smoothstep(0.38, 0.68, fbm(h, 2)) * smoothstep(0.2, 0.75, uv.y) * mix(0.45, 1.0, room);
  return 1.0 - (1.0 - cumulus) * (1.0 - puffs) * (1.0 - streaks);
}
`;
const DRAG = `${HEAD}
in vec2 vUv;
uniform sampler2D uTarget;
uniform float uAspect;
uniform vec2 uFrom;
uniform vec2 uTo;
uniform vec2 uValue;
uniform float uRadius;
uniform float uGain;
out vec4 color;
void main() {
  vec2 scale = vec2(uAspect, 1.0);
  vec2 p = (vUv - uFrom) * scale;
  vec2 along = (uTo - uFrom) * scale;
  vec2 off = p - along * clamp(dot(p, along) / max(dot(along, along), 1e-8), 0.0, 1.0);
  vec2 v = texture(uTarget, vUv).xy;
  color = vec4(mix(v, uValue, exp(-dot(off, off) / uRadius) * uGain), 0.0, 1.0);
}`;
const GUST = `${HEAD}
in vec2 vUv;
uniform sampler2D uTarget;
uniform float uAspect;
uniform vec2 uPoint;
uniform float uRadius;
uniform float uStrength;
out vec4 color;
void main() {
  vec2 p = (vUv - uPoint) * vec2(uAspect, 1.0);
  float r = length(p);
  vec2 away = p / max(r, 1e-4);
  float ring = (r - uRadius) / (0.6 * uRadius);
  vec2 push = away + 0.4 * sin(6.0 * atan(p.y, p.x)) * vec2(-away.y, away.x);
  color = vec4(texture(uTarget, vUv).xy + uStrength * exp(-ring * ring) * push, 0.0, 1.0);
}`;
const ADVECT = `${HEAD}
in vec2 vUv;
uniform sampler2D uVelocity;
uniform vec2 uTexel;
uniform float uWind;
uniform float uDt;
uniform float uKeep;
out vec4 color;
void main() {
  vec2 back = vUv - uDt * (texture(uVelocity, vUv).xy + vec2(uWind, 0.0)) * uTexel;
  color = vec4(texture(uVelocity, back).xy * uKeep, 0.0, 1.0);
}`;
const DENSITY = `${HEAD}
in vec2 vUv;
uniform sampler2D uVelocity;
uniform sampler2D uDensity;
uniform vec2 uGrid;
uniform float uWind;
uniform float uDt;
uniform float uGather;
uniform float uDissolve;
uniform float uForget;
uniform float uThin;
out vec4 color;
${BANKS_AT}
void main() {
  vec2 v = texture(uVelocity, vUv).xy;
  vec4 was = texture(uDensity, vUv - uDt * (v + vec2(uWind, 0.0)) * uGrid);
  float speed = length(v) * uGrid.y;
  float kept = was.g * uForget;
  float stirred = max(kept, smoothstep(0.03, 0.6, speed));
  float d = was.r * (1.0 - uThin * (stirred - kept));
  float want = banks(vUv);
  float calm = (1.0 - stirred) * (1.0 - stirred);
  float rate = (want > d ? uGather : uDissolve) * (0.3 + 0.7 * calm);
  d += (want - d) * (1.0 - exp(-rate * uDt));
  color = vec4(clamp(d, 0.0, 1.0), stirred, 0.0, 1.0);
}`;
const BANKS = `${HEAD}
in vec2 vUv;
out vec4 color;
${BANKS_AT}
void main() { color = vec4(banks(vUv), 0.0, 0.0, 1.0); }`;
const CARRY = `${HEAD}
in vec2 vUv;
uniform sampler2D uVelocity;
uniform sampler2D uCarry;
uniform vec2 uGrid;
uniform float uAspect;
uniform float uWind;
uniform float uDt;
uniform vec2 uKeep;
out vec4 color;
void main() {
  vec2 flow = texture(uVelocity, vUv).xy * uGrid.y;
  vec2 back = vUv - uDt * (flow + vec2(uWind, 0.0)) * vec2(1.0 / uAspect, 1.0);
  vec4 from = texture(uCarry, back) - uDt * flow.xyxy;
  color = clamp(from, -2.0, 2.0) * uKeep.xxyy;
}`;
const CURL = `${HEAD}
in vec2 vL;
in vec2 vR;
in vec2 vT;
in vec2 vB;
uniform sampler2D uVelocity;
out vec4 color;
void main() {
  float c = texture(uVelocity, vR).y - texture(uVelocity, vL).y - texture(uVelocity, vT).x + texture(uVelocity, vB).x;
  color = vec4(0.5 * c, 0.0, 0.0, 1.0);
}`;
const FORCES = `${HEAD}
in vec2 vUv;
in vec2 vL;
in vec2 vR;
in vec2 vT;
in vec2 vB;
uniform sampler2D uVelocity;
uniform sampler2D uCurl;
uniform sampler2D uDensity;
uniform float uSwirl;
uniform float uLift;
uniform float uDt;
out vec4 color;
void main() {
  float l = texture(uCurl, vL).x;
  float r = texture(uCurl, vR).x;
  float t = texture(uCurl, vT).x;
  float b = texture(uCurl, vB).x;
  float c = texture(uCurl, vUv).x;
  vec2 force = 0.5 * vec2(abs(t) - abs(b), abs(r) - abs(l));
  force /= length(force) + 1e-4;
  force *= uSwirl * c * vec2(1.0, -1.0);
  vec2 cloud = texture(uDensity, vUv).rg;
  force.y += uLift * cloud.r * cloud.g;
  vec2 v = texture(uVelocity, vUv).xy + force * uDt;
  color = vec4(clamp(v, -600.0, 600.0), 0.0, 1.0);
}`;
const DIVERGENCE = `${HEAD}
in vec2 vUv;
in vec2 vL;
in vec2 vR;
in vec2 vT;
in vec2 vB;
uniform sampler2D uVelocity;
out vec4 color;
void main() {
  vec2 c = texture(uVelocity, vUv).xy;
  float l = vL.x < 0.0 ? -c.x : texture(uVelocity, vL).x;
  float r = vR.x > 1.0 ? -c.x : texture(uVelocity, vR).x;
  float t = vT.y > 1.0 ? -c.y : texture(uVelocity, vT).y;
  float b = vB.y < 0.0 ? -c.y : texture(uVelocity, vB).y;
  color = vec4(0.5 * (r - l + t - b), 0.0, 0.0, 1.0);
}`;
const PRESSURE = `${HEAD}
in vec2 vL;
in vec2 vR;
in vec2 vT;
in vec2 vB;
in vec2 vUv;
uniform sampler2D uPressure;
uniform sampler2D uDivergence;
uniform float uKeep;
out vec4 color;
void main() {
  float p = texture(uPressure, vL).x + texture(uPressure, vR).x + texture(uPressure, vT).x + texture(uPressure, vB).x;
  color = vec4(0.25 * (uKeep * p - texture(uDivergence, vUv).x), 0.0, 0.0, 1.0);
}`;
const GRADIENT = `${HEAD}
in vec2 vUv;
in vec2 vL;
in vec2 vR;
in vec2 vT;
in vec2 vB;
uniform sampler2D uPressure;
uniform sampler2D uVelocity;
out vec4 color;
void main() {
  float l = texture(uPressure, vL).x;
  float r = texture(uPressure, vR).x;
  float t = texture(uPressure, vT).x;
  float b = texture(uPressure, vB).x;
  vec2 v = texture(uVelocity, vUv).xy - 0.5 * vec2(r - l, t - b);
  color = vec4(v, 0.0, 1.0);
}`;
const DISPLAY = `${HEAD}
in vec2 vUv;
uniform sampler2D uDensity;
uniform sampler2D uCarry;
uniform float uTime;
uniform float uAspect;
uniform float uDrift;
uniform vec2 uPhase;
uniform vec2 uSun;
uniform vec2 uPixel;
uniform vec3 uLit;
uniform vec3 uShade;
uniform vec3 uRim;
uniform float uOpacity;
uniform float uRoom;
out vec4 color;
${NOISE}${OPEN}
float density(vec2 uv) { return texture(uDensity, uv).r; }
vec4 lobes(vec2 p, float cell) {
  vec4 sum = vec4(0.0);
  float amp = 0.5;
  float fine = 1.0;
  mat2 home = mat2(1.0);
  for (int i = 0; i < 4; i++) {
    float keep = 1.0 - smoothstep(0.28, 0.62, cell * fine);
    if (keep > 0.0) {
      vec3 n = sloped(p);
      float side = 2.0 * n.x - 1.0;
      float fold = sqrt(side * side + 0.03);
      sum += amp * (i < 3 ? vec4(mix(0.5, n.x, keep), mix(0.434, fold, keep), keep * fine * 2.0 * side / fold * (home * n.yz)) : vec4(mix(0.5, n.x, keep), 0.7 * mix(0.5, n.x, keep), 0.7 * keep * fine * (home * n.yz)));
    } else sum.xy += amp * (i < 3 ? vec2(0.5, 0.434) : vec2(0.5, 0.35));
    p = TURN * p * 2.03 + vec2(11.7, 4.3) + uTime * vec2(0.03, -0.018);
    home = 0.62 * home * transpose(TURN);
    fine *= 2.03;
    amp *= 0.5;
  }
  return sum;
}
void main() {
  vec4 cloud = texture(uDensity, vUv);
  vec4 carry = texture(uCarry, vUv);
  vec2 q = vec2((vUv.x - uDrift) * uAspect, vUv.y) * 7.0 + vec2(uTime * 0.02, 0.0);
  vec2 qa = q + carry.xy * 7.0;
  vec2 qb = q + carry.zw * 7.0;
  float ca = max(length(dFdx(qa)), length(dFdy(qa)));
  float cb = max(length(dFdx(qb)), length(dFdy(qb)));
  float base = cloud.r;
  if (base < 0.05) { color = vec4(0.0); return; }
  vec2 apart = (carry.xy - carry.zw) * 7.0;
  float apartSq = dot(apart, apart);
  float alike = exp(-1.6 * apartSq);
  vec2 w = uPhase * inversesqrt(dot(uPhase, uPhase) + 2.0 * uPhase.x * uPhase.y * alike);
  const vec4 MEAN = vec4(0.4514, 0.3958, 0.0, 0.0);
  vec4 n = lobes(qa, ca) - MEAN;
  if (w.y >= 0.015 && apartSq >= 0.0005) n = w.x * n + w.y * (lobes(qb, cb) - MEAN);
  float stirred = cloud.g;
  float relief = mix(1.0, 0.35, stirred);
  float shape = relief * 1.25 * n.y + (1.0 - 0.6 * relief) * n.x;
  float h = base + shape * (0.42 + 0.5 * base) * smoothstep(0.0, 0.25, base);
  float edge = mix(0.2, 0.4, stirred);
  float d = smoothstep(0.3 - 0.5 * edge, 0.3 + 0.5 * edge, h);
  if (d <= 0.002) { color = vec4(0.0); return; }
  vec2 e = uPixel * 7.0;
  vec2 lean = vec2(density(vUv + vec2(e.x, 0.0)) - density(vUv - vec2(e.x, 0.0)), density(vUv + vec2(0.0, e.y)) - density(vUv - vec2(0.0, e.y)));
  vec3 normal = normalize(vec3(-2.2 * lean - 0.5 * relief * n.zw, 1.0));
  vec2 toward = normalize(uSun - vUv);
  vec3 sun = normalize(vec3(toward * 0.8, 0.6));
  float diffuse = clamp(dot(normal, sun), 0.0, 1.0);
  float depth = 0.0;
  for (int i = 1; i <= 4; i++) depth += density(vUv + toward * 0.025 * float(i));
  float light = exp(-depth * 0.42);
  float lit = clamp(0.54 + 0.42 * smoothstep(0.2, 1.0, light) + 0.35 * (diffuse - 0.6) + 0.24 * relief * shape, 0.0, 1.0);
  vec3 c = mix(uShade, uLit, lit);
  c += uRim * pow(1.0 - d, 1.6) * light * (0.3 + 0.5 * diffuse);
  float thick = smoothstep(0.3, 0.9, h);
  float a = d * uOpacity * mix(0.74, 1.0, thick) * mix(uRoom, 1.0, open(vUv));
  float grain = (hash(gl_FragCoord.xy + fract(uTime) * 61.0) - 0.5) / 255.0;
  color = max(vec4(c * a, a) + grain, 0.0);
}`;
// ============================================================================
// WebGPU Fluid + Cloud Renderer (Single CommandEncoder / queue.submit)
// ============================================================================
const GPU_MAX_SLOTS = 32;
const GPU_STRIDE_BYTES = 256;
const GPU_STRIDE_FLOATS = 64;
function gpuRead(d) {
    return d.pairs[d.idx];
}
function gpuWrite(d) {
    return d.pairs[1 - d.idx];
}
function gpuSwap(d) {
    d.idx = 1 - d.idx;
}
class GpuClouds {
    backend = "webgpu";
    device;
    ctx;
    format;
    uniformLayout;
    texLayout;
    sampler;
    uBuffer;
    uBindGroup;
    dummyTex;
    dummyBg;
    pipes;
    stage = new Float32Array(GPU_MAX_SLOTS * GPU_STRIDE_FLOATS);
    targets = null;
    encoder = null;
    slotCount = 0;
    time = 0;
    drift = 0;
    phase = 0;
    energy = 0;
    rested = WISP_LIFE;
    look = SUNLIGHT;
    constructor(device, ctx, format, uniformLayout, texLayout, sampler, uBuffer, uBindGroup, dummyTex, dummyBg, pipes) {
        this.device = device;
        this.ctx = ctx;
        this.format = format;
        this.uniformLayout = uniformLayout;
        this.texLayout = texLayout;
        this.sampler = sampler;
        this.uBuffer = uBuffer;
        this.uBindGroup = uBindGroup;
        this.dummyTex = dummyTex;
        this.dummyBg = dummyBg;
        this.pipes = pipes;
    }
    static async create(canvas) {
        try {
            const gpuApi = navigator.gpu;
            if (!gpuApi)
                return null;
            const adapter = await gpuApi.requestAdapter({ powerPreference: "high-performance" });
            if (!adapter)
                return null;
            const device = await adapter.requestDevice();
            const format = gpuApi.getPreferredCanvasFormat();
            device.pushErrorScope?.("validation");
            const uniformLayout = device.createBindGroupLayout({
                entries: [
                    {
                        binding: 0,
                        visibility: GPUShaderStage.FRAGMENT,
                        buffer: { type: "uniform", hasDynamicOffset: true, minBindingSize: 64 },
                    },
                ],
            });
            const texLayout = device.createBindGroupLayout({
                entries: [
                    { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: "float" } },
                    { binding: 1, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: "float" } },
                    { binding: 2, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: "float" } },
                    { binding: 3, visibility: GPUShaderStage.FRAGMENT, sampler: { type: "filtering" } },
                ],
            });
            const pipeLayout = device.createPipelineLayout({ bindGroupLayouts: [uniformLayout, texLayout] });
            const makePipe = (code, targetFormat) => {
                const mod = device.createShaderModule({ code });
                return device.createRenderPipeline({
                    layout: pipeLayout,
                    vertex: { module: mod, entryPoint: "vs_main" },
                    fragment: {
                        module: mod,
                        entryPoint: "fs_main",
                        targets: [
                            {
                                format: targetFormat,
                                blend: {
                                    color: { srcFactor: "one", dstFactor: "zero", operation: "add" },
                                    alpha: { srcFactor: "one", dstFactor: "zero", operation: "add" },
                                },
                            },
                        ],
                    },
                    primitive: { topology: "triangle-list" },
                });
            };
            const pipes = {
                drag: makePipe(DRAG_WGSL, "rgba16float"),
                gust: makePipe(GUST_WGSL, "rgba16float"),
                advect: makePipe(ADVECT_WGSL, "rgba16float"),
                density: makePipe(DENSITY_WGSL, "rgba16float"),
                banks: makePipe(BANKS_WGSL, "rgba16float"),
                carry: makePipe(CARRY_WGSL, "rgba16float"),
                curl: makePipe(CURL_WGSL, "rgba16float"),
                forces: makePipe(FORCES_WGSL, "rgba16float"),
                divergence: makePipe(DIVERGENCE_WGSL, "rgba16float"),
                pressure: makePipe(PRESSURE_WGSL, "rgba16float"),
                gradient: makePipe(GRADIENT_WGSL, "rgba16float"),
                display: makePipe(DISPLAY_WGSL, format),
            };
            const validationErr = device.popErrorScope ? await device.popErrorScope() : null;
            if (validationErr) {
                device.destroy?.();
                return null;
            }
            const ctx = canvas.getContext("webgpu");
            if (!ctx) {
                device.destroy?.();
                return null;
            }
            ctx.configure({
                device,
                format,
                alphaMode: "premultiplied",
                usage: GPUTextureUsage.RENDER_ATTACHMENT,
            });
            const sampler = device.createSampler({
                addressModeU: "clamp-to-edge",
                addressModeV: "clamp-to-edge",
                minFilter: "linear",
                magFilter: "linear",
            });
            const uBuffer = device.createBuffer({
                size: GPU_MAX_SLOTS * GPU_STRIDE_BYTES,
                usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
            });
            const uBindGroup = device.createBindGroup({
                layout: uniformLayout,
                entries: [{ binding: 0, resource: { buffer: uBuffer, size: 64 } }],
            });
            const dTex = device.createTexture({
                size: [1, 1, 1],
                format: "rgba16float",
                usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.RENDER_ATTACHMENT,
            });
            const dummyTex = { texture: dTex, view: dTex.createView(), w: 1, h: 1 };
            const dummyBg = device.createBindGroup({
                layout: texLayout,
                entries: [
                    { binding: 0, resource: dummyTex.view },
                    { binding: 1, resource: dummyTex.view },
                    { binding: 2, resource: dummyTex.view },
                    { binding: 3, resource: sampler },
                ],
            });
            return new GpuClouds(device, ctx, format, uniformLayout, texLayout, sampler, uBuffer, uBindGroup, dummyTex, dummyBg, pipes);
        }
        catch {
            return null;
        }
    }
    makeTex(w, h, created) {
        const texture = this.device.createTexture({
            size: [w, h, 1],
            format: "rgba16float",
            usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.RENDER_ATTACHMENT,
        });
        created.push(texture);
        return { texture, view: texture.createView(), w, h };
    }
    makeDouble(w, h, created) {
        return { pairs: [this.makeTex(w, h, created), this.makeTex(w, h, created)], idx: 0 };
    }
    makeBg(a, b, c) {
        return this.device.createBindGroup({
            layout: this.texLayout,
            entries: [
                { binding: 0, resource: a.view },
                { binding: 1, resource: b.view },
                { binding: 2, resource: c.view },
                { binding: 3, resource: this.sampler },
            ],
        });
    }
    flush() {
        if (!this.encoder)
            return;
        if (this.slotCount > 0) {
            this.device.queue.writeBuffer(this.uBuffer, 0, this.stage, 0, this.slotCount * GPU_STRIDE_FLOATS);
        }
        this.device.queue.submit([this.encoder.finish()]);
        this.encoder = null;
        this.slotCount = 0;
    }
    clearViews(views, alpha) {
        this.encoder ??= this.device.createCommandEncoder();
        for (const v of views) {
            const pass = this.encoder.beginRenderPass({
                colorAttachments: [
                    {
                        view: v.view,
                        clearValue: { r: 0, g: 0, b: 0, a: alpha },
                        loadOp: "clear",
                        storeOp: "store",
                    },
                ],
            });
            pass.end();
        }
    }
    recordPass(pipe, outView, outW, outH, bg, fillUniforms) {
        if (this.slotCount >= GPU_MAX_SLOTS)
            this.flush();
        this.encoder ??= this.device.createCommandEncoder();
        const base = this.slotCount * GPU_STRIDE_FLOATS;
        const st = this.stage;
        for (let i = 0; i < 16; i++)
            st[base + i] = 0;
        fillUniforms(st, base);
        const pass = this.encoder.beginRenderPass({
            colorAttachments: [
                {
                    view: outView,
                    clearValue: { r: 0, g: 0, b: 0, a: 0 },
                    loadOp: "clear",
                    storeOp: "store",
                },
            ],
        });
        pass.setViewport(0, 0, outW, outH, 0, 1);
        pass.setScissorRect(0, 0, outW, outH);
        pass.setPipeline(pipe);
        pass.setBindGroup(0, this.uBindGroup, [this.slotCount * GPU_STRIDE_BYTES]);
        pass.setBindGroup(1, bg);
        pass.draw(3);
        pass.end();
        this.slotCount++;
    }
    releaseTargets() {
        this.flush();
        if (this.targets) {
            for (const tex of this.targets.textures)
                tex.destroy();
            this.targets = null;
        }
    }
    resize(width, height) {
        const aspect = Math.max(0.3, Math.min(4, width / Math.max(1, height)));
        const airSize = { w: Math.max(24, Math.round(AIR * aspect)), h: AIR };
        const cloudSize = { w: Math.min(720, Math.max(64, Math.round(CLOUD * aspect))), h: CLOUD };
        if (this.targets?.air.pairs[0].w === airSize.w && this.targets.air.pairs[0].h === airSize.h) {
            return true;
        }
        this.releaseTargets();
        const created = [];
        const air = this.makeDouble(airSize.w, airSize.h, created);
        const pressure = this.makeDouble(airSize.w, airSize.h, created);
        const carry = this.makeDouble(airSize.w, airSize.h, created);
        const cloud = this.makeDouble(cloudSize.w, cloudSize.h, created);
        const divergence = this.makeTex(airSize.w, airSize.h, created);
        const curl = this.makeTex(airSize.w, airSize.h, created);
        const dm = this.dummyTex;
        const bgAir = [
            this.makeBg(air.pairs[0], dm, dm),
            this.makeBg(air.pairs[1], dm, dm),
        ];
        const bgCarry = [
            [this.makeBg(air.pairs[0], carry.pairs[0], dm), this.makeBg(air.pairs[0], carry.pairs[1], dm)],
            [this.makeBg(air.pairs[1], carry.pairs[0], dm), this.makeBg(air.pairs[1], carry.pairs[1], dm)],
        ];
        const bgDensity = [
            [this.makeBg(air.pairs[0], cloud.pairs[0], dm), this.makeBg(air.pairs[0], cloud.pairs[1], dm)],
            [this.makeBg(air.pairs[1], cloud.pairs[0], dm), this.makeBg(air.pairs[1], cloud.pairs[1], dm)],
        ];
        const bgForces = [
            [this.makeBg(air.pairs[0], curl, cloud.pairs[0]), this.makeBg(air.pairs[0], curl, cloud.pairs[1])],
            [this.makeBg(air.pairs[1], curl, cloud.pairs[0]), this.makeBg(air.pairs[1], curl, cloud.pairs[1])],
        ];
        const bgPressure = [
            this.makeBg(pressure.pairs[0], divergence, dm),
            this.makeBg(pressure.pairs[1], divergence, dm),
        ];
        const bgGradient = [
            [this.makeBg(pressure.pairs[0], air.pairs[0], dm), this.makeBg(pressure.pairs[0], air.pairs[1], dm)],
            [this.makeBg(pressure.pairs[1], air.pairs[0], dm), this.makeBg(pressure.pairs[1], air.pairs[1], dm)],
        ];
        const bgDisplay = [
            [this.makeBg(cloud.pairs[0], carry.pairs[0], dm), this.makeBg(cloud.pairs[0], carry.pairs[1], dm)],
            [this.makeBg(cloud.pairs[1], carry.pairs[0], dm), this.makeBg(cloud.pairs[1], carry.pairs[1], dm)],
        ];
        this.targets = {
            air,
            pressure,
            carry,
            cloud,
            divergence,
            curl,
            textures: created,
            bgAir,
            bgCarry,
            bgDensity,
            bgForces,
            bgPressure,
            bgGradient,
            bgDisplay,
        };
        this.clearViews([air.pairs[0], air.pairs[1], pressure.pairs[0], pressure.pairs[1], divergence, curl, cloud.pairs[0], cloud.pairs[1]], 1);
        this.clearViews([carry.pairs[0], carry.pairs[1]], 0);
        this.energy = 0;
        this.rested = WISP_LIFE;
        const clOut = gpuWrite(cloud);
        const asp = airSize.w / airSize.h;
        this.recordPass(this.pipes.banks, clOut.view, clOut.w, clOut.h, this.dummyBg, (st, base) => {
            st[base + 2] = this.time;
            st[base + 3] = asp;
            st[base + 4] = this.drift;
            st[base + 5] = this.look.cover;
        });
        gpuSwap(cloud);
        this.flush();
        return true;
    }
    setLook(look) {
        this.look = look;
    }
    aspect() {
        return this.targets ? this.targets.air.pairs[0].w / this.targets.air.pairs[0].h : 1;
    }
    gusting() {
        const t = this.time;
        return 1 + 0.42 * Math.sin(t * 0.131) + 0.27 * Math.sin(t * 0.057 + 1.7) + 0.16 * Math.sin(t * 0.37 + 4.1);
    }
    drag(from, to, vx, vy, radius, gain) {
        const tg = this.targets;
        if (!tg)
            return;
        const airR = gpuRead(tg.air);
        const airW = gpuWrite(tg.air);
        const limit = 520;
        let cx = vx * airR.h;
        let cy = vy * airR.h;
        const speed = Math.hypot(cx, cy);
        if (speed > limit) {
            cx *= limit / speed;
            cy *= limit / speed;
        }
        const asp = this.aspect();
        this.recordPass(this.pipes.drag, airW.view, airW.w, airW.h, tg.bgAir[tg.air.idx], (st, base) => {
            st[base + 2] = asp;
            st[base + 3] = radius * radius;
            st[base + 4] = from.x;
            st[base + 5] = from.y;
            st[base + 6] = to.x;
            st[base + 7] = to.y;
            st[base + 8] = cx;
            st[base + 9] = cy;
            st[base + 10] = gain;
        });
        gpuSwap(tg.air);
        this.energy = 1;
    }
    gust(x, y, radius, strength) {
        const tg = this.targets;
        if (!tg)
            return;
        const airR = gpuRead(tg.air);
        const airW = gpuWrite(tg.air);
        const asp = this.aspect();
        this.recordPass(this.pipes.gust, airW.view, airW.w, airW.h, tg.bgAir[tg.air.idx], (st, base) => {
            st[base + 2] = asp;
            st[base + 4] = x;
            st[base + 5] = y;
            st[base + 6] = radius;
            st[base + 7] = strength * airR.h;
        });
        gpuSwap(tg.air);
        this.energy = 1;
    }
    step(dt) {
        const tg = this.targets;
        if (!tg)
            return;
        const { air, pressure, carry, cloud, divergence, curl } = tg;
        const stepDt = Math.min(Math.max(dt, 0), 1 / 30);
        this.time += stepDt;
        const gusting = this.gusting();
        this.drift += WIND * gusting * stepDt;
        const air0 = air.pairs[0];
        const wind = WIND * gusting * air0.w;
        const airTexelX = 1 / air0.w;
        const airTexelY = 1 / air0.h;
        if (this.energy > REST) {
            const awake = Math.min(1, (this.energy - REST) / (5 * REST));
            this.recordPass(this.pipes.curl, curl.view, curl.w, curl.h, tg.bgAir[air.idx], (st, base) => {
                st[base + 0] = airTexelX;
                st[base + 1] = airTexelY;
            });
            const airW1 = gpuWrite(air);
            this.recordPass(this.pipes.forces, airW1.view, airW1.w, airW1.h, tg.bgForces[air.idx][cloud.idx], (st, base) => {
                st[base + 0] = airTexelX;
                st[base + 1] = airTexelY;
                st[base + 2] = SWIRL * awake;
                st[base + 3] = LIFT * awake;
                st[base + 4] = stepDt;
            });
            gpuSwap(air);
            this.recordPass(this.pipes.divergence, divergence.view, divergence.w, divergence.h, tg.bgAir[air.idx], (st, base) => {
                st[base + 0] = airTexelX;
                st[base + 1] = airTexelY;
            });
            for (let i = 0; i < JACOBI; i++) {
                const pOut = gpuWrite(pressure);
                const keep = i === 0 ? Math.exp(-13 * stepDt) : 1;
                this.recordPass(this.pipes.pressure, pOut.view, pOut.w, pOut.h, tg.bgPressure[pressure.idx], (st, base) => {
                    st[base + 0] = airTexelX;
                    st[base + 1] = airTexelY;
                    st[base + 2] = keep;
                });
                gpuSwap(pressure);
            }
            const airW2 = gpuWrite(air);
            this.recordPass(this.pipes.gradient, airW2.view, airW2.w, airW2.h, tg.bgGradient[pressure.idx][air.idx], (st, base) => {
                st[base + 0] = airTexelX;
                st[base + 1] = airTexelY;
            });
            gpuSwap(air);
            const airW3 = gpuWrite(air);
            const advKeep = Math.exp(-(AIR_FADE + 3 * (1 - awake)) * stepDt);
            this.recordPass(this.pipes.advect, airW3.view, airW3.w, airW3.h, tg.bgAir[air.idx], (st, base) => {
                st[base + 0] = airTexelX;
                st[base + 1] = airTexelY;
                st[base + 2] = wind;
                st[base + 3] = stepDt;
                st[base + 4] = advKeep;
            });
            gpuSwap(air);
            this.energy *= Math.exp(-AIR_FADE * stepDt);
            if (this.energy <= REST) {
                this.clearViews([air.pairs[0], air.pairs[1], pressure.pairs[0], pressure.pairs[1]], 1);
            }
        }
        const before = this.phase;
        this.phase += stepDt / WISP_LIFE;
        const first = Math.floor(this.phase) === Math.floor(before) ? 1 : 0;
        const second = Math.floor(this.phase + 0.5) === Math.floor(before + 0.5) ? 1 : 0;
        if (this.phase >= 2)
            this.phase -= 2;
        this.rested = this.energy > REST ? 0 : this.rested + stepDt;
        const asp = this.aspect();
        if (this.rested < WISP_LIFE + 1) {
            const carryW = gpuWrite(carry);
            this.recordPass(this.pipes.carry, carryW.view, carryW.w, carryW.h, tg.bgCarry[air.idx][carry.idx], (st, base) => {
                st[base + 2] = airTexelX;
                st[base + 3] = airTexelY;
                st[base + 4] = asp;
                st[base + 5] = WIND * gusting * asp;
                st[base + 6] = stepDt;
                st[base + 8] = first;
                st[base + 9] = second;
            });
            gpuSwap(carry);
        }
        const clW = gpuWrite(cloud);
        const forget = Math.exp(-stepDt / MEMORY);
        this.recordPass(this.pipes.density, clW.view, clW.w, clW.h, tg.bgDensity[air.idx][cloud.idx], (st, base) => {
            st[base + 2] = airTexelX;
            st[base + 3] = airTexelY;
            st[base + 4] = wind;
            st[base + 5] = stepDt;
            st[base + 6] = forget;
            st[base + 7] = THIN;
            st[base + 8] = this.time;
            st[base + 9] = asp;
            st[base + 10] = this.drift;
            st[base + 11] = this.look.cover;
            st[base + 12] = GATHER;
            st[base + 13] = DISSOLVE;
        });
        gpuSwap(cloud);
    }
    still() {
        const tg = this.targets;
        if (!tg)
            return;
        this.clearViews([tg.carry.pairs[0], tg.carry.pairs[1]], 0);
        const clW = gpuWrite(tg.cloud);
        const asp = this.aspect();
        this.recordPass(this.pipes.banks, clW.view, clW.w, clW.h, this.dummyBg, (st, base) => {
            st[base + 2] = this.time;
            st[base + 3] = asp;
            st[base + 4] = this.drift;
            st[base + 5] = this.look.cover;
        });
        gpuSwap(tg.cloud);
    }
    draw(width, height) {
        const tg = this.targets;
        if (!tg)
            return;
        const first = 1 - Math.abs(2 * (this.phase % 1) - 1);
        const look = this.look;
        const asp = this.aspect();
        const outView = this.ctx.getCurrentTexture().createView();
        const rimFactor = look === SUNLIGHT ? 1.0 : 0.0;
        this.recordPass(this.pipes.display, outView, width, height, tg.bgDisplay[tg.cloud.idx][tg.carry.idx], (st, base) => {
            st[base + 0] = 1 / width;
            st[base + 1] = 1 / height;
            st[base + 2] = this.time;
            st[base + 3] = asp;
            st[base + 4] = this.drift;
            st[base + 5] = first;
            st[base + 6] = 1 - first;
            st[base + 7] = look.opacity;
            st[base + 8] = look.lit[0];
            st[base + 9] = look.lit[1];
            st[base + 10] = look.lit[2];
            st[base + 11] = look.room;
            st[base + 12] = look.shade[0];
            st[base + 13] = look.shade[1];
            st[base + 14] = look.shade[2];
            st[base + 15] = rimFactor;
        });
        // Submit the entire frame's simulation + display pass in ONE queue.submit!
        this.flush();
    }
    stirred() {
        return this.energy > REST;
    }
    dispose() {
        this.releaseTargets();
        this.uBuffer.destroy();
        this.dummyTex.texture.destroy();
        this.ctx.unconfigure();
    }
}
function swapGl(d) {
    const t = d.read;
    d.read = d.write;
    d.write = t;
}
class GlClouds {
    backend = "webgl2";
    gl;
    progs = null;
    air = null;
    cloud = null;
    carry = null;
    pressure = null;
    divergence = null;
    curl = null;
    textures = [];
    buffers = [];
    shaders = [];
    vao = null;
    quad = null;
    time = 0;
    drift = 0;
    phase = 0;
    energy = 0;
    rested = WISP_LIFE;
    look = SUNLIGHT;
    constructor(gl) {
        this.gl = gl;
    }
    static create(canvas) {
        let gl = null;
        try {
            gl = canvas.getContext("webgl2", {
                alpha: true,
                premultipliedAlpha: true,
                antialias: false,
                depth: false,
                stencil: false,
                preserveDrawingBuffer: false,
                powerPreference: "high-performance",
            });
        }
        catch {
            return null;
        }
        if (gl === null || gl.isContextLost())
            return null;
        if (gl.getExtension("EXT_color_buffer_float") === null && gl.getExtension("EXT_color_buffer_half_float") === null)
            return null;
        const clouds = new GlClouds(gl);
        if (!clouds.build()) {
            clouds.dispose();
            return null;
        }
        return clouds;
    }
    shader(kind, source) {
        const gl = this.gl;
        const shader = gl.createShader(kind);
        if (shader === null)
            return null;
        this.shaders.push(shader);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        return gl.getShaderParameter(shader, gl.COMPILE_STATUS) ? shader : null;
    }
    compileProg(vertex, source) {
        const gl = this.gl;
        const fragment = this.shader(gl.FRAGMENT_SHADER, source);
        const program = gl.createProgram();
        if (fragment === null || program === null)
            return null;
        gl.attachShader(program, vertex);
        gl.attachShader(program, fragment);
        gl.bindAttribLocation(program, 0, "aPosition");
        gl.linkProgram(program);
        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            gl.deleteProgram(program);
            return null;
        }
        const loc = (n) => gl.getUniformLocation(program, n);
        return {
            program,
            uTexel: loc("uTexel"),
            uTarget: loc("uTarget"),
            uAspect: loc("uAspect"),
            uFrom: loc("uFrom"),
            uTo: loc("uTo"),
            uValue: loc("uValue"),
            uRadius: loc("uRadius"),
            uGain: loc("uGain"),
            uPoint: loc("uPoint"),
            uStrength: loc("uStrength"),
            uVelocity: loc("uVelocity"),
            uWind: loc("uWind"),
            uDt: loc("uDt"),
            uKeep: loc("uKeep"),
            uDensity: loc("uDensity"),
            uGrid: loc("uGrid"),
            uGather: loc("uGather"),
            uDissolve: loc("uDissolve"),
            uForget: loc("uForget"),
            uThin: loc("uThin"),
            uTime: loc("uTime"),
            uDrift: loc("uDrift"),
            uCover: loc("uCover"),
            uCarry: loc("uCarry"),
            uCurl: loc("uCurl"),
            uSwirl: loc("uSwirl"),
            uLift: loc("uLift"),
            uPressure: loc("uPressure"),
            uDivergence: loc("uDivergence"),
            uPhase: loc("uPhase"),
            uSun: loc("uSun"),
            uPixel: loc("uPixel"),
            uLit: loc("uLit"),
            uShade: loc("uShade"),
            uRim: loc("uRim"),
            uOpacity: loc("uOpacity"),
            uRoom: loc("uRoom"),
        };
    }
    build() {
        const gl = this.gl;
        const vertex = this.shader(gl.VERTEX_SHADER, VERTEX);
        if (vertex === null)
            return false;
        const drag = this.compileProg(vertex, DRAG);
        const gust = this.compileProg(vertex, GUST);
        const advect = this.compileProg(vertex, ADVECT);
        const density = this.compileProg(vertex, DENSITY);
        const banks = this.compileProg(vertex, BANKS);
        const carry = this.compileProg(vertex, CARRY);
        const curl = this.compileProg(vertex, CURL);
        const forces = this.compileProg(vertex, FORCES);
        const divergence = this.compileProg(vertex, DIVERGENCE);
        const pressure = this.compileProg(vertex, PRESSURE);
        const gradient = this.compileProg(vertex, GRADIENT);
        const display = this.compileProg(vertex, DISPLAY);
        if (!drag || !gust || !advect || !density || !banks || !carry || !curl || !forces || !divergence || !pressure || !gradient || !display) {
            return false;
        }
        this.progs = { drag, gust, advect, density, banks, carry, curl, forces, divergence, pressure, gradient, display };
        this.quad = gl.createBuffer();
        this.vao = gl.createVertexArray();
        gl.bindVertexArray(this.vao);
        gl.bindBuffer(gl.ARRAY_BUFFER, this.quad);
        gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
        gl.enableVertexAttribArray(0);
        gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
        gl.disable(gl.BLEND);
        return true;
    }
    clear(targets, alpha) {
        const gl = this.gl;
        gl.clearColor(0, 0, 0, alpha);
        for (const t of targets) {
            gl.bindFramebuffer(gl.FRAMEBUFFER, t.fbo);
            gl.viewport(0, 0, t.w, t.h);
            gl.clear(gl.COLOR_BUFFER_BIT);
        }
    }
    target(w, h, alpha = 1) {
        const gl = this.gl;
        const tex = gl.createTexture();
        const fbo = gl.createFramebuffer();
        if (!tex || !fbo)
            return null;
        this.textures.push(tex);
        this.buffers.push(fbo);
        gl.bindTexture(gl.TEXTURE_2D, tex);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA16F, w, h, 0, gl.RGBA, gl.HALF_FLOAT, null);
        gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
        gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
        if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE)
            return null;
        const made = { tex, fbo, w, h };
        this.clear([made], alpha);
        return made;
    }
    double(w, h, alpha = 1) {
        const read = this.target(w, h, alpha);
        const write = this.target(w, h, alpha);
        return read === null || write === null ? null : { read, write };
    }
    releaseTargets() {
        for (const t of this.textures)
            this.gl.deleteTexture(t);
        for (const f of this.buffers)
            this.gl.deleteFramebuffer(f);
        this.textures = [];
        this.buffers = [];
        this.air = this.cloud = this.carry = this.pressure = null;
        this.divergence = this.curl = null;
    }
    begin(p, out, texW, texH) {
        const gl = this.gl;
        gl.useProgram(p.program);
        gl.uniform2f(p.uTexel, 1 / texW, 1 / texH);
        gl.bindFramebuffer(gl.FRAMEBUFFER, out?.fbo ?? null);
        if (out !== null)
            gl.viewport(0, 0, out.w, out.h);
    }
    finishDraw() {
        const gl = this.gl;
        gl.bindVertexArray(this.vao);
        gl.drawArrays(gl.TRIANGLES, 0, 3);
    }
    bind(unit, target, loc) {
        const gl = this.gl;
        gl.activeTexture(gl.TEXTURE0 + unit);
        gl.bindTexture(gl.TEXTURE_2D, target.tex);
        gl.uniform1i(loc, unit);
    }
    resize(width, height) {
        const aspect = Math.max(0.3, Math.min(4, width / Math.max(1, height)));
        const air = { w: Math.max(24, Math.round(AIR * aspect)), h: AIR };
        const cloud = { w: Math.min(720, Math.max(64, Math.round(CLOUD * aspect))), h: CLOUD };
        if (this.air?.read.w === air.w && this.air.read.h === air.h)
            return true;
        this.releaseTargets();
        this.air = this.double(air.w, air.h);
        this.pressure = this.double(air.w, air.h);
        this.divergence = this.target(air.w, air.h);
        this.curl = this.target(air.w, air.h);
        this.carry = this.double(air.w, air.h, 0);
        this.cloud = this.double(cloud.w, cloud.h);
        if (this.air === null || this.pressure === null || this.divergence === null || this.curl === null || this.carry === null || this.cloud === null) {
            return false;
        }
        this.energy = 0;
        this.rested = WISP_LIFE;
        if (this.progs) {
            const p = this.progs.banks;
            const out = this.cloud.write;
            this.begin(p, out, out.w, out.h);
            this.gl.uniform1f(p.uTime, this.time);
            this.gl.uniform1f(p.uAspect, this.aspect());
            this.gl.uniform1f(p.uDrift, this.drift);
            this.gl.uniform1f(p.uCover, this.look.cover);
            this.finishDraw();
            swapGl(this.cloud);
        }
        return true;
    }
    setLook(look) {
        this.look = look;
    }
    aspect() {
        return this.air === null ? 1 : this.air.read.w / this.air.read.h;
    }
    gusting() {
        const t = this.time;
        return 1 + 0.42 * Math.sin(t * 0.131) + 0.27 * Math.sin(t * 0.057 + 1.7) + 0.16 * Math.sin(t * 0.37 + 4.1);
    }
    drag(from, to, vx, vy, radius, gain) {
        const air = this.air;
        const p = this.progs?.drag;
        if (air === null || !p)
            return;
        const gl = this.gl;
        const limit = 520;
        let cx = vx * air.read.h;
        let cy = vy * air.read.h;
        const speed = Math.hypot(cx, cy);
        if (speed > limit) {
            cx *= limit / speed;
            cy *= limit / speed;
        }
        this.begin(p, air.write, air.write.w, air.write.h);
        this.bind(0, air.read, p.uTarget);
        gl.uniform1f(p.uAspect, this.aspect());
        gl.uniform2f(p.uFrom, from.x, from.y);
        gl.uniform2f(p.uTo, to.x, to.y);
        gl.uniform2f(p.uValue, cx, cy);
        gl.uniform1f(p.uRadius, radius * radius);
        gl.uniform1f(p.uGain, gain);
        this.finishDraw();
        swapGl(air);
        this.energy = 1;
    }
    gust(x, y, radius, strength) {
        const air = this.air;
        const p = this.progs?.gust;
        if (air === null || !p)
            return;
        const gl = this.gl;
        this.begin(p, air.write, air.write.w, air.write.h);
        this.bind(0, air.read, p.uTarget);
        gl.uniform1f(p.uAspect, this.aspect());
        gl.uniform2f(p.uPoint, x, y);
        gl.uniform1f(p.uRadius, radius);
        gl.uniform1f(p.uStrength, strength * air.read.h);
        this.finishDraw();
        swapGl(air);
        this.energy = 1;
    }
    step(dt) {
        const { air, cloud, carry, pressure, divergence, curl, progs } = this;
        if (air === null || cloud === null || carry === null || pressure === null || divergence === null || curl === null || !progs)
            return;
        const gl = this.gl;
        const stepDt = Math.min(Math.max(dt, 0), 1 / 30);
        this.time += stepDt;
        const gusting = this.gusting();
        this.drift += WIND * gusting * stepDt;
        const wind = WIND * gusting * air.read.w;
        if (this.energy > REST) {
            const awake = Math.min(1, (this.energy - REST) / (5 * REST));
            this.begin(progs.curl, curl, curl.w, curl.h);
            this.bind(0, air.read, progs.curl.uVelocity);
            this.finishDraw();
            this.begin(progs.forces, air.write, air.write.w, air.write.h);
            this.bind(0, air.read, progs.forces.uVelocity);
            this.bind(1, curl, progs.forces.uCurl);
            this.bind(2, cloud.read, progs.forces.uDensity);
            gl.uniform1f(progs.forces.uSwirl, SWIRL * awake);
            gl.uniform1f(progs.forces.uLift, LIFT * awake);
            gl.uniform1f(progs.forces.uDt, stepDt);
            this.finishDraw();
            swapGl(air);
            this.begin(progs.divergence, divergence, divergence.w, divergence.h);
            this.bind(0, air.read, progs.divergence.uVelocity);
            this.finishDraw();
            const pProg = progs.pressure;
            for (let i = 0; i < JACOBI; i++) {
                this.begin(pProg, pressure.write, pressure.write.w, pressure.write.h);
                this.bind(0, pressure.read, pProg.uPressure);
                this.bind(1, divergence, pProg.uDivergence);
                gl.uniform1f(pProg.uKeep, i === 0 ? Math.exp(-13 * stepDt) : 1);
                this.finishDraw();
                swapGl(pressure);
            }
            this.begin(progs.gradient, air.write, air.write.w, air.write.h);
            this.bind(0, pressure.read, progs.gradient.uPressure);
            this.bind(1, air.read, progs.gradient.uVelocity);
            this.finishDraw();
            swapGl(air);
            this.begin(progs.advect, air.write, air.write.w, air.write.h);
            this.bind(0, air.read, progs.advect.uVelocity);
            gl.uniform1f(progs.advect.uWind, wind);
            gl.uniform1f(progs.advect.uDt, stepDt);
            gl.uniform1f(progs.advect.uKeep, Math.exp(-(AIR_FADE + 3 * (1 - awake)) * stepDt));
            this.finishDraw();
            swapGl(air);
            this.energy *= Math.exp(-AIR_FADE * stepDt);
            if (this.energy <= REST)
                this.clear([air.read, air.write, pressure.read, pressure.write], 1);
        }
        const before = this.phase;
        this.phase += stepDt / WISP_LIFE;
        const first = Math.floor(this.phase) === Math.floor(before) ? 1 : 0;
        const second = Math.floor(this.phase + 0.5) === Math.floor(before + 0.5) ? 1 : 0;
        if (this.phase >= 2)
            this.phase -= 2;
        this.rested = this.energy > REST ? 0 : this.rested + stepDt;
        if (this.rested < WISP_LIFE + 1) {
            const cp = progs.carry;
            this.begin(cp, carry.write, carry.write.w, carry.write.h);
            this.bind(0, air.read, cp.uVelocity);
            this.bind(1, carry.read, cp.uCarry);
            gl.uniform2f(cp.uGrid, 1 / air.read.w, 1 / air.read.h);
            gl.uniform1f(cp.uAspect, this.aspect());
            gl.uniform1f(cp.uWind, WIND * gusting * this.aspect());
            gl.uniform1f(cp.uDt, stepDt);
            gl.uniform2f(cp.uKeep, first, second);
            this.finishDraw();
            swapGl(carry);
        }
        const dp = progs.density;
        this.begin(dp, cloud.write, cloud.read.w, cloud.read.h);
        this.bind(0, air.read, dp.uVelocity);
        this.bind(1, cloud.read, dp.uDensity);
        gl.uniform1f(dp.uTime, this.time);
        gl.uniform1f(dp.uAspect, this.aspect());
        gl.uniform1f(dp.uDrift, this.drift);
        gl.uniform1f(dp.uCover, this.look.cover);
        gl.uniform2f(dp.uGrid, 1 / air.read.w, 1 / air.read.h);
        gl.uniform1f(dp.uWind, wind);
        gl.uniform1f(dp.uDt, stepDt);
        gl.uniform1f(dp.uGather, GATHER);
        gl.uniform1f(dp.uDissolve, DISSOLVE);
        gl.uniform1f(dp.uForget, Math.exp(-stepDt / MEMORY));
        gl.uniform1f(dp.uThin, THIN);
        this.finishDraw();
        swapGl(cloud);
    }
    still() {
        const { cloud, carry, progs } = this;
        if (cloud === null || carry === null || !progs)
            return;
        this.clear([carry.read, carry.write], 0);
        const bp = progs.banks;
        this.begin(bp, cloud.write, cloud.write.w, cloud.write.h);
        this.gl.uniform1f(bp.uTime, this.time);
        this.gl.uniform1f(bp.uAspect, this.aspect());
        this.gl.uniform1f(bp.uDrift, this.drift);
        this.gl.uniform1f(bp.uCover, this.look.cover);
        this.finishDraw();
        swapGl(cloud);
    }
    draw(width, height) {
        const { cloud, carry, progs } = this;
        if (cloud === null || carry === null || !progs)
            return;
        const gl = this.gl;
        gl.bindFramebuffer(gl.FRAMEBUFFER, null);
        gl.viewport(0, 0, width, height);
        gl.clearColor(0, 0, 0, 0);
        gl.clear(gl.COLOR_BUFFER_BIT);
        const look = this.look;
        const first = 1 - Math.abs(2 * (this.phase % 1) - 1);
        const dp = progs.display;
        this.begin(dp, null, width, height);
        this.bind(0, cloud.read, dp.uDensity);
        this.bind(1, carry.read, dp.uCarry);
        gl.uniform1f(dp.uTime, this.time);
        gl.uniform1f(dp.uAspect, this.aspect());
        gl.uniform1f(dp.uDrift, this.drift);
        gl.uniform2f(dp.uPhase, first, 1 - first);
        gl.uniform2f(dp.uSun, 0.5, 1.2);
        gl.uniform2f(dp.uPixel, 1 / width, 1 / height);
        gl.uniform3f(dp.uLit, look.lit[0], look.lit[1], look.lit[2]);
        gl.uniform3f(dp.uShade, look.shade[0], look.shade[1], look.shade[2]);
        gl.uniform3f(dp.uRim, look.rim[0], look.rim[1], look.rim[2]);
        gl.uniform1f(dp.uOpacity, look.opacity);
        gl.uniform1f(dp.uRoom, look.room);
        this.finishDraw();
    }
    software() {
        const gl = this.gl;
        const info = gl.getExtension("WEBGL_debug_renderer_info");
        const name = String(gl.getParameter(info === null ? gl.RENDERER : info.UNMASKED_RENDERER_WEBGL) ?? "");
        return /swiftshader|llvmpipe|softpipe|software|basic render/i.test(name);
    }
    stirred() {
        return this.energy > REST;
    }
    dispose() {
        const gl = this.gl;
        this.releaseTargets();
        if (this.progs) {
            gl.deleteProgram(this.progs.drag.program);
            gl.deleteProgram(this.progs.gust.program);
            gl.deleteProgram(this.progs.advect.program);
            gl.deleteProgram(this.progs.density.program);
            gl.deleteProgram(this.progs.banks.program);
            gl.deleteProgram(this.progs.carry.program);
            gl.deleteProgram(this.progs.curl.program);
            gl.deleteProgram(this.progs.forces.program);
            gl.deleteProgram(this.progs.divergence.program);
            gl.deleteProgram(this.progs.pressure.program);
            gl.deleteProgram(this.progs.gradient.program);
            gl.deleteProgram(this.progs.display.program);
            this.progs = null;
        }
        for (const s of this.shaders)
            gl.deleteShader(s);
        if (this.quad !== null)
            gl.deleteBuffer(this.quad);
        if (this.vao !== null)
            gl.deleteVertexArray(this.vao);
        this.shaders = [];
        gl.getExtension("WEBGL_lose_context")?.loseContext();
    }
}
// ============================================================================
// Lifecycle & Tiered Boot (WebGPU -> WebGL2, Zero Per-Frame Layout Reads)
// ============================================================================
const dark = () => getComputedStyle(document.documentElement).colorScheme.includes("dark");
const forced = () => {
    try {
        return localStorage.getItem("baylee-clouds") === "live";
    }
    catch {
        return false;
    }
};
function start(hero) {
    const sky = hero.querySelector(".sky");
    if (!sky || sky.querySelector(".sky-clouds"))
        return;
    const canvas = document.createElement("canvas");
    canvas.className = "sky-clouds";
    sky.querySelector(".sky-painting")?.after(canvas);
    void initAndRun(hero, canvas);
}
async function initAndRun(hero, firstCanvas) {
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    let canvas = firstCanvas;
    let renderer = await GpuClouds.create(canvas);
    let still = reduced;
    if (!renderer) {
        // If GpuClouds acquired a WebGPU context before failing configure(), replace
        // the canvas node so WebGL2 can acquire a clean context.
        const fresh = document.createElement("canvas");
        fresh.className = canvas.className;
        canvas.replaceWith(fresh);
        canvas = fresh;
        const glClouds = GlClouds.create(canvas);
        if (glClouds) {
            renderer = glClouds;
            still = reduced || (glClouds.software() && !forced());
        }
    }
    if (!renderer || !hero.isConnected) {
        renderer?.dispose();
        canvas.dataset.renderer = "none";
        return;
    }
    const clouds = renderer;
    canvas.dataset.shaderBackend = clouds.backend;
    canvas.dataset.renderer = still ? "still" : "live";
    let lost = false;
    let frame = 0;
    let last = performance.now();
    let cssW = canvas.clientWidth || 1280;
    let cssH = canvas.clientHeight || 720;
    let aspect = cssW / Math.max(1, cssH);
    const look = () => clouds.setLook(dark() ? BLUE_HOUR : SUNLIGHT);
    look();
    const applySize = () => {
        const dpr = Math.min(devicePixelRatio || 1, 2);
        const width = Math.max(1, Math.round(Math.min(cssW * dpr * SCALE, MAX_WIDTH)));
        const height = Math.max(1, Math.round((width * cssH) / Math.max(1, cssW)));
        if (canvas.width !== width || canvas.height !== height) {
            canvas.width = width;
            canvas.height = height;
            if (!clouds.resize(width, height)) {
                lost = true;
                canvas.dataset.renderer = "none";
            }
        }
    };
    const paint = () => {
        if (!lost)
            clouds.draw(canvas.width, canvas.height);
    };
    const picture = () => {
        applySize();
        if (!lost) {
            clouds.still();
            paint();
        }
    };
    let trail = [];
    let prev = null;
    let rectLeft = 0;
    let rectTop = 0;
    let rectW = cssW;
    let rectH = cssH;
    const refreshRect = () => {
        const r = canvas.getBoundingClientRect();
        rectLeft = r.left;
        rectTop = r.top;
        rectW = Math.max(1, r.width);
        rectH = Math.max(1, r.height);
    };
    refreshRect();
    const at = (event) => ({
        x: (event.clientX - rectLeft) / rectW,
        y: 1 - (event.clientY - rectTop) / rectH,
        t: event.timeStamp,
    });
    const move = (event) => {
        const p = at(event);
        if (p.x < -0.05 || p.x > 1.05 || p.y < -0.05 || p.y > 1.05) {
            prev = null;
            return;
        }
        if (trail.length < 16)
            trail.push(p);
    };
    const leave = () => {
        prev = null;
        trail = [];
    };
    const press = (event) => {
        if (event.button !== 0)
            return;
        refreshRect();
        const p = at(event);
        clouds.gust(p.x, p.y, 0.05, 1.5);
    };
    const stir = () => {
        for (const p of trail) {
            if (prev !== null && p.t > prev.t) {
                const dt = Math.min((p.t - prev.t) / 1000, 0.1);
                const vx = ((p.x - prev.x) * aspect) / dt;
                const vy = (p.y - prev.y) / dt;
                const swift = Math.min(1, Math.hypot(vx, vy) / SWIFT);
                const reach = REACH * (1 + (SWEEP - 1) * swift * swift * (3 - 2 * swift));
                clouds.drag(prev, p, vx * FORCE, vy * FORCE, reach, 1 - Math.exp(-dt / GRIP));
            }
            prev = p;
        }
        trail = [];
    };
    const paused = () => document.hidden || hero.dataset.away !== undefined || document.querySelector("dialog[open]") !== null;
    const tick = (now) => {
        frame = requestAnimationFrame(tick);
        const dt = (now - last) / 1000;
        const calm = !clouds.stirred() && trail.length === 0;
        if (paused() || lost || dt < (calm ? 1 / 24 : 1 / 60))
            return;
        last = now;
        stir();
        clouds.step(dt);
        paint();
    };
    const themed = () => {
        look();
        if (still)
            picture();
    };
    const leaving = new AbortController();
    const theme = new MutationObserver(themed);
    theme.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", themed, { signal: leaving.signal });
    const watchDpr = () => {
        if (leaving.signal.aborted || typeof matchMedia !== "function")
            return;
        const mql = matchMedia(`(resolution: ${devicePixelRatio || 1}dppx)`);
        mql.addEventListener("change", () => {
            if (leaving.signal.aborted || lost)
                return;
            watchDpr();
            if (still)
                picture();
            else
                applySize();
        }, { once: true, signal: leaving.signal });
    };
    watchDpr();
    const sized = new ResizeObserver((entries) => {
        for (const entry of entries) {
            const cr = entry.contentRect;
            if (cr.width > 0 && cr.height > 0) {
                cssW = cr.width;
                cssH = cr.height;
                aspect = cssW / Math.max(1, cssH);
            }
        }
        refreshRect();
        if (still)
            picture();
        else
            applySize();
    });
    sized.observe(canvas);
    document.addEventListener("baylee:leave", () => {
        leaving.abort();
        theme.disconnect();
        sized.disconnect();
        lost = true;
        cancelAnimationFrame(frame);
        clouds.dispose();
    }, { once: true });
    canvas.addEventListener("webglcontextlost", (event) => {
        event.preventDefault();
        lost = true;
        cancelAnimationFrame(frame);
        canvas.dataset.renderer = "none";
    });
    if (still)
        picture();
    else {
        hero.addEventListener("pointermove", move, { passive: true });
        hero.addEventListener("pointerleave", leave, { passive: true });
        hero.addEventListener("pointerdown", press, { passive: true });
        applySize();
        frame = requestAnimationFrame(tick);
    }
    requestAnimationFrame(() => canvas.classList.add("on"));
}
function page() {
    const hero = document.querySelector(".hero");
    if (!hero || !matchMedia("(hover: hover) and (pointer: fine)").matches)
        return;
    const idle = typeof window.requestIdleCallback === "function" ? window.requestIdleCallback.bind(window) : (cb) => setTimeout(cb, 200);
    idle(() => {
        if (hero.isConnected)
            start(hero);
    });
}
if (document.readyState === "complete")
    page();
else
    addEventListener("load", page, { once: true });
document.addEventListener("baylee:page", page);
export {};
