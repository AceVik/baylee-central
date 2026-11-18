// The start page's sky, alive: clouds that drift on a gusting wind and that
// the pointer stirs (the old site's, PR #62; owner, 11.11.2026: effects for
// the header; 18.11.2026: more of them, lighter, and air that moves like
// air). The air is a small real fluid, Stam's stable fluids on the GPU: the
// pointer drags it along its path, the drag rolls up into eddies that
// vorticity confinement keeps for a while, stirred vapour rises a little,
// and the clouds are carried by that air, thinned where it is fast, until
// it calms and they gather into their banks again. Their fine wisps ride
// the same air (two flow maps, crossed over); in a calm sky they turn over
// slowly by themselves. A press sends a gust outwards.
//
// It starts once the page has loaded and the browser is idle, so the
// painting stays the largest paint and nothing here delays it; and only
// where a pointer can stir the air (a mouse or a pen), so a phone spends
// neither battery nor a download's worth of work on it. Paused while the
// hero is away (`data-away`), the tab hidden or a dialog open; one still
// picture under reduced motion and on a software renderer, where the air
// would cost a CPU core (tests may ask for it anyway: localStorage
// `baylee-clouds` = `live`); the painting alone where WebGL 2 is missing or
// its context was lost. Plain DOM and WebGL: no markup strings, nothing a
// Trusted Types policy would have to vouch for.

// ---- the shaders -------------------------------------------------------------

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
/** Value noise and its fractal sum. */
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
/** The noise (x) and its slope (yz). */
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
  for (int i = 0; i < 7; i++) {
    if (i >= octaves) break;
    sum += amp * noise(p);
    p = TURN * p * 2.03 + vec2(11.7, 4.3);
    amp *= 0.5;
  }
  return sum;
}
`;
/** How much room the words keep: 0 behind them, 1 towards the hero's edges. */
const OPEN = `
float open(vec2 uv) {
  return smoothstep(0.12, 0.92, length((uv - vec2(0.5, 0.58)) * vec2(1.05, 1.55)));
}
`;
/** The clouds the density gathers into: three decks, each on its own share of the wind. */
const BANKS_AT = `${NOISE}${OPEN}
uniform float uTime;
uniform float uAspect;
uniform float uDrift;
uniform float uCover;
float banks(vec2 uv) {
  float room = open(uv);
  float cover = uCover * mix(0.58, 1.0, room) + 0.07 * smoothstep(0.22, 0.5, abs(uv.x - 0.5)) + 0.04 * smoothstep(0.4, 0.0, uv.y) - 0.05 * smoothstep(0.8, 1.0, uv.y);
  // cumulus banks, thinner behind the words, fuller towards the edges and the foot
  vec2 p = vec2((uv.x - uDrift) * uAspect, uv.y) * 3.1;
  vec2 warp = vec2(fbm(p * 0.7 + vec2(0.0, uTime * 0.014), 3), fbm(p * 0.7 + vec2(5.2, 1.3) - uTime * 0.011, 3));
  float cumulus = smoothstep(0.99 - cover, 1.21 - cover, fbm(p + 1.8 * warp, 5));
  // small puffs in front of them, a little faster
  vec2 s = vec2((uv.x - uDrift * 1.25) * uAspect, uv.y) * 6.7 + vec2(31.7, 8.1 + uTime * 0.006);
  float puffs = 0.62 * smoothstep(1.1 - cover, 1.33 - cover, fbm(s + 0.9 * warp, 3));
  // and thin streaks of high cloud behind, drawn out along the wind
  vec2 h = vec2((uv.x - uDrift * 0.8) * uAspect * 1.3, uv.y * 4.6) + vec2(uTime * 0.004, 17.3) + 0.6 * warp;
  float streaks = 0.5 * smoothstep(0.38, 0.68, fbm(h, 2)) * smoothstep(0.2, 0.75, uv.y) * mix(0.45, 1.0, room);
  return 1.0 - (1.0 - cumulus) * (1.0 - puffs) * (1.0 - streaks);
}
`;
/** The pointer's drag: the air along its path takes up a share of its speed. */
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
/** A press: a ring of air thrown outwards, alternately turned, so it rolls up at its rim. */
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
/** Velocity carried by itself and the wind, fading. */
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
/**
 * The density carried by the air (red) and how lately that air was stirred
 * (green): a stir thins the vapour, and it gathers towards the banks
 * again as the memory of the stir fades, so nothing pops back.
 */
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
  // heights a second
  float speed = length(v) * uGrid.y;
  float kept = was.g * uForget;
  float stirred = max(kept, smoothstep(0.03, 0.6, speed));
  // a share is lost to each new stir, however many steps it is told in
  float d = was.r * (1.0 - uThin * (stirred - kept));
  float want = banks(vUv);
  float calm = (1.0 - stirred) * (1.0 - stirred);
  float rate = (want > d ? uGather : uDissolve) * (0.3 + 0.7 * calm);
  d += (want - d) * (1.0 - exp(-rate * uDt));
  color = vec4(clamp(d, 0.0, 1.0), stirred, 0.0, 1.0);
}`;
/** The banks alone: the first density, and the still picture for reduced motion. */
const BANKS = `${HEAD}
in vec2 vUv;
out vec4 color;
${BANKS_AT}
void main() { color = vec4(banks(vUv), 0.0, 0.0, 1.0); }`;
/**
 * Where the vapour at each place came from, twice over (xy and zw, in the
 * hero's heights): the fine wisps are drawn there, so the air stretches and
 * curls them. The two maps start over in turn and the picture crosses from
 * one to the other, so a wisp is drawn out and then replaced, never pulled
 * back. Where the air has rested for a map's life both are nothing again.
 */
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
  // the air in heights a second; the wind carries the map along but not the wisps apart
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
/**
 * What pushes the air besides the pointer: vorticity confinement, which
 * gives back the small swirls the coarse grid smears out, and the lift of
 * vapour that was stirred (the banks at rest stay where they are).
 */
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
/** Divergence, the walls reflecting the flow. */
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
/** One Jacobi iteration; the first takes the last step's pressure, faded by `uKeep`, as its start. */
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
/** Subtract the pressure gradient: what remains does not compress. */
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
/**
 * The clouds as they are seen: the density sharpened into soft edges by
 * wisps that ride the air (drawn where each of the two maps says the vapour
 * came from, crossed over by `uPhase`, and once where the maps agree, as
 * in all of a calm sky; an octave too fine for the pixels, as in air drawn
 * out thin, gives way to its mean), shaded like a surface (its normal from
 * the thickness and the lobes) and by its own depth towards the sun, so
 * tops shine, cores and undersides fall into a light shade and thin edges
 * glow warm; premultiplied, and dithered against bands.
 */
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
/**
 * Five octaves of noise, twice: their plain sum (x) and the sum of their
 * folds (y), round lobes with creases between as a cumulus has them, with
 * the lobes' slope (zw). An octave too fine for the pixels (cell is the
 * lattice cells one covers) gives way to its mean. Each octave drifts
 * slowly against the one before, so the lobes turn over by themselves.
 */
vec4 lobes(vec2 p, float cell) {
  vec4 sum = vec4(0.0);
  float amp = 0.5;
  float fine = 1.0;
  mat2 home = mat2(1.0);
  for (int i = 0; i < 5; i++) {
    float keep = 1.0 - smoothstep(0.28, 0.62, cell * fine);
    if (keep > 0.0) {
      vec3 n = sloped(p);
      float side = 2.0 * n.x - 1.0;
      // the fold, its crease rounded
      float fold = sqrt(side * side + 0.03);
      // the coarse octaves fold, the finest only roughens
      sum += amp * (i < 4 ? vec4(mix(0.5, n.x, keep), mix(0.434, fold, keep), keep * fine * 2.0 * side / fold * (home * n.yz)) : vec4(mix(0.5, n.x, keep), 0.7 * mix(0.5, n.x, keep), 0.7 * keep * fine * (home * n.yz)));
    } else sum.xy += amp * (i < 4 ? vec2(0.5, 0.434) : vec2(0.5, 0.35));
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
  // too thin for any lobe to reach the edge below
  if (base < 0.05) { color = vec4(0.0); return; }
  // the two maps show the same wisps where they agree, as wherever the air has rested: drawn once there,
  // and elsewhere weighed so that their sum keeps its contrast
  vec2 apart = (carry.xy - carry.zw) * 7.0;
  float alike = exp(-1.6 * dot(apart, apart));
  vec2 w = uPhase * inversesqrt(dot(uPhase, uPhase) + 2.0 * uPhase.x * uPhase.y * alike);
  const vec4 MEAN = vec4(0.4825, 0.4178, 0.0, 0.0);
  vec4 n = lobes(qa, ca) - MEAN;
  if (dot(apart, apart) > 0.0) n = w.x * n + w.y * (lobes(qb, cb) - MEAN);
  // calm cloud is lobed, stirred vapour drawn out and soft
  float stirred = cloud.g;
  float relief = mix(1.0, 0.35, stirred);
  float shape = relief * 1.25 * n.y + (1.0 - 0.6 * relief) * n.x;
  // the lobes reach out of what vapour there is: none where there is none
  float h = base + shape * (0.42 + 0.5 * base) * smoothstep(0.0, 0.25, base);
  float edge = mix(0.2, 0.4, stirred);
  float d = smoothstep(0.3 - 0.5 * edge, 0.3 + 0.5 * edge, h);
  if (d <= 0.002) { color = vec4(0.0); return; }
  // a surface: its broad slope from the carried density, its relief from the lobes
  vec2 e = uPixel * 7.0;
  vec2 lean = vec2(density(vUv + vec2(e.x, 0.0)) - density(vUv - vec2(e.x, 0.0)), density(vUv + vec2(0.0, e.y)) - density(vUv - vec2(0.0, e.y)));
  vec3 normal = normalize(vec3(-2.2 * lean - 0.5 * relief * n.zw, 1.0));
  vec2 toward = normalize(uSun - vUv);
  vec3 sun = normalize(vec3(toward * 0.8, 0.6));
  float diffuse = clamp(dot(normal, sun), 0.0, 1.0);
  float depth = 0.0;
  for (int i = 1; i <= 5; i++) depth += density(vUv + toward * 0.021 * float(i));
  float light = exp(-depth * 0.34);
  // the body of a cloud is light: only what faces away, the creases and the undersides fall into the shade
  float lit = clamp(0.54 + 0.42 * smoothstep(0.2, 1.0, light) + 0.35 * (diffuse - 0.6) + 0.24 * relief * shape, 0.0, 1.0);
  vec3 c = mix(uShade, uLit, lit);
  // thin vapour lets the light through: a warm rim towards the sun
  c += uRim * pow(1.0 - d, 1.6) * light * (0.3 + 0.5 * diffuse);
  float thick = smoothstep(0.3, 0.9, h);
  float a = d * uOpacity * mix(0.74, 1.0, thick) * mix(uRoom, 1.0, open(vUv));
  float grain = (hash(gl_FragCoord.xy + fract(uTime) * 61.0) - 0.5) / 255.0;
  color = max(vec4(c * a, a) + grain, 0.0);
}`;

// ---- the air and the clouds ------------------------------------------------------

/**
 * A look: the colours of lit and shaded cloud and of the rim towards the
 * sun, how much of the painting a thick cloud hides, the share of that left
 * behind the words, and how much of the sky the banks cover.
 */
const SUNLIGHT = { lit: [1, 0.99, 0.955], shade: [0.7, 0.75, 0.88], rim: [1, 0.9, 0.68], opacity: 0.9, room: 0.8, cover: 0.6 };
const BLUE_HOUR = { lit: [0.97, 0.97, 1], shade: [0.58, 0.64, 0.83], rim: [1, 0.82, 0.62], opacity: 0.8, room: 0.7, cover: 0.58 };
/** The air's grid height, the cloud density's, the pressure iterations. */
const AIR = 112;
const CLOUD = 300;
const JACOBI = 18;
/** The wind, in the hero's widths a second: a bank crosses it in about three minutes. It comes in gusts. */
const WIND = 0.0055;
/** How long a map of where the wisps came from lasts, in seconds. */
const WISP_LIFE = 7;
/** How fast the air forgets (per second), and below which share of a stir it rests. */
const AIR_FADE = 0.42;
const REST = 0.012;
/** How fast clouds gather where the air is calm and how fast stray vapour dissolves (per second). */
const GATHER = 0.3;
const DISSOLVE = 0.24;
/** How long the vapour remembers a stir (seconds), and the share of it a stir out of the calm thins away. */
const MEMORY = 2.4;
const THIN = 0.28;
/** Vorticity confinement, and the lift of stirred vapour (cells a second squared). */
const SWIRL = 30;
const LIFT = 4;
function swap(d) {
  const t = d.read;
  d.read = d.write;
  d.write = t;
}
class Clouds {
  gl;
  programs = new Map();
  air = null;
  cloud = null;
  carry = null;
  pressure = null;
  divergence = null;
  curl = null;
  textures = [];
  buffers = [];
  shapes = [];
  vao = null;
  quad = null;
  time = 0;
  drift = 0;
  /** Where the two maps of the wisps are in their lives: the first at its fraction, the second half a life on. */
  phase = 0;
  /** How much the air still moves, 0 to 1; below `REST` the solve is skipped. */
  energy = 0;
  /** How long the air has rested, in seconds: after a map's life neither map holds anything, and they rest too. */
  rested = WISP_LIFE;
  look = SUNLIGHT;
  constructor(gl) {
    this.gl = gl;
  }
  /** The clouds on `canvas`, or null where WebGL 2 or a renderable half-float texture is missing. */
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
        powerPreference: "low-power",
      });
    } catch {
      return null;
    }
    if (gl === null || gl.isContextLost()) return null;
    if (gl.getExtension("EXT_color_buffer_float") === null && gl.getExtension("EXT_color_buffer_half_float") === null) return null;
    const clouds = new Clouds(gl);
    if (!clouds.build()) {
      clouds.dispose();
      return null;
    }
    return clouds;
  }
  build() {
    const gl = this.gl;
    const sources = {
      drag: DRAG,
      gust: GUST,
      advect: ADVECT,
      density: DENSITY,
      banks: BANKS,
      carry: CARRY,
      curl: CURL,
      forces: FORCES,
      divergence: DIVERGENCE,
      pressure: PRESSURE,
      gradient: GRADIENT,
      display: DISPLAY,
    };
    const vertex = this.shader(gl.VERTEX_SHADER, VERTEX);
    if (vertex === null) return false;
    for (const [name, source] of Object.entries(sources)) {
      const fragment = this.shader(gl.FRAGMENT_SHADER, source);
      const program = gl.createProgram();
      if (fragment === null) return false;
      gl.attachShader(program, vertex);
      gl.attachShader(program, fragment);
      gl.bindAttribLocation(program, 0, "aPosition");
      gl.linkProgram(program);
      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
        gl.deleteProgram(program);
        return false;
      }
      const uniforms = new Map();
      const count = gl.getProgramParameter(program, gl.ACTIVE_UNIFORMS);
      for (let i = 0; i < count; i++) {
        const info = gl.getActiveUniform(program, i);
        const at = info === null ? null : gl.getUniformLocation(program, info.name);
        if (info !== null && at !== null) uniforms.set(info.name, at);
      }
      this.programs.set(name, { program, uniforms });
    }
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
  shader(kind, source) {
    const gl = this.gl;
    const shader = gl.createShader(kind);
    if (shader === null) return null;
    this.shapes.push(shader);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    return gl.getShaderParameter(shader, gl.COMPILE_STATUS) ? shader : null;
  }
  /** Empties `targets`: all four channels to nothing but the last, which is `alpha`. */
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
    const ok = gl.checkFramebufferStatus(gl.FRAMEBUFFER) === gl.FRAMEBUFFER_COMPLETE;
    if (!ok) return null;
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
    for (const t of this.textures) this.gl.deleteTexture(t);
    for (const f of this.buffers) this.gl.deleteFramebuffer(f);
    this.textures = [];
    this.buffers = [];
    this.air = this.cloud = this.carry = this.pressure = null;
    this.divergence = this.curl = null;
  }
  /** Grids for a canvas of this shape; the clouds start as the banks. False when a texture cannot be drawn to. */
  resize(width, height) {
    const aspect = Math.max(0.3, Math.min(4, width / Math.max(1, height)));
    const air = { w: Math.max(24, Math.round(AIR * aspect)), h: AIR };
    const cloud = { w: Math.min(900, Math.max(64, Math.round(CLOUD * aspect))), h: CLOUD };
    if (this.air?.read.w === air.w && this.air.read.h === air.h) return true;
    this.releaseTargets();
    this.air = this.double(air.w, air.h);
    this.pressure = this.double(air.w, air.h);
    this.divergence = this.target(air.w, air.h);
    this.curl = this.target(air.w, air.h);
    // all four of its channels are distances: none of them starts at one
    this.carry = this.double(air.w, air.h, 0);
    this.cloud = this.double(cloud.w, cloud.h);
    if (this.air === null || this.pressure === null || this.divergence === null || this.curl === null || this.carry === null || this.cloud === null)
      return false;
    this.energy = 0;
    this.rested = WISP_LIFE;
    this.pass("banks", this.cloud.write, (u) => this.banksUniforms(u));
    swap(this.cloud);
    return true;
  }
  setLook(look) {
    this.look = look;
  }
  aspect() {
    return this.air === null ? 1 : this.air.read.w / this.air.read.h;
  }
  use(name) {
    const p = this.programs.get(name);
    if (p === undefined) throw new Error(`no program ${name}`);
    this.gl.useProgram(p.program);
    return p;
  }
  /** Draws `name` into `out` (null: the canvas); `set` gives its uniforms. */
  pass(name, out, set, size) {
    const gl = this.gl;
    const p = this.use(name);
    const grid = size ?? out ?? { w: 1, h: 1 };
    const at = (n) => p.uniforms.get(n) ?? null;
    gl.uniform2f(at("uTexel"), 1 / grid.w, 1 / grid.h);
    set(at, p);
    gl.bindFramebuffer(gl.FRAMEBUFFER, out?.fbo ?? null);
    if (out !== null) gl.viewport(0, 0, out.w, out.h);
    gl.bindVertexArray(this.vao);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
  bind(unit, target, at) {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + unit);
    gl.bindTexture(gl.TEXTURE_2D, target.tex);
    gl.uniform1i(at, unit);
  }
  banksUniforms(u) {
    const gl = this.gl;
    gl.uniform1f(u("uTime"), this.time);
    gl.uniform1f(u("uAspect"), this.aspect());
    gl.uniform1f(u("uDrift"), this.drift);
    gl.uniform1f(u("uCover"), this.look.cover);
  }
  /** The wind's strength now, about one: slow gusts over slower ones. */
  gusting() {
    const t = this.time;
    return 1 + 0.42 * Math.sin(t * 0.131) + 0.27 * Math.sin(t * 0.057 + 1.7) + 0.16 * Math.sin(t * 0.37 + 4.1);
  }
  /**
   * Drags the air along the pointer's path from `from` to `to` (0 to 1, from
   * the bottom left) towards `vx, vy` (heights a second) within `radius` (a
   * height's share): `gain` is the share of that speed the air on the path
   * takes up, so a drag never makes it faster than what drags it.
   */
  drag(from, to, vx, vy, radius, gain) {
    const air = this.air;
    if (air === null) return;
    const gl = this.gl;
    const limit = 520;
    let cx = vx * air.read.h;
    let cy = vy * air.read.h;
    const speed = Math.hypot(cx, cy);
    if (speed > limit) {
      cx *= limit / speed;
      cy *= limit / speed;
    }
    this.pass("drag", air.write, (u) => {
      this.bind(0, air.read, u("uTarget"));
      gl.uniform1f(u("uAspect"), this.aspect());
      gl.uniform2f(u("uFrom"), from.x, from.y);
      gl.uniform2f(u("uTo"), to.x, to.y);
      gl.uniform2f(u("uValue"), cx, cy);
      gl.uniform1f(u("uRadius"), radius * radius);
      gl.uniform1f(u("uGain"), gain);
    });
    swap(air);
    this.energy = 1;
  }
  /** Throws a ring of air outwards from `x, y`, `radius` (a height's share) wide, at `strength` heights a second. */
  gust(x, y, radius, strength) {
    const air = this.air;
    if (air === null) return;
    const gl = this.gl;
    this.pass("gust", air.write, (u) => {
      this.bind(0, air.read, u("uTarget"));
      gl.uniform1f(u("uAspect"), this.aspect());
      gl.uniform2f(u("uPoint"), x, y);
      gl.uniform1f(u("uRadius"), radius);
      gl.uniform1f(u("uStrength"), strength * air.read.h);
    });
    swap(air);
    this.energy = 1;
  }
  /** One step of `dt` seconds. */
  step(dt) {
    const { air, cloud, carry, pressure, divergence, curl } = this;
    if (air === null || cloud === null || carry === null || pressure === null || divergence === null || curl === null) return;
    const gl = this.gl;
    dt = Math.min(Math.max(dt, 0), 1 / 30);
    this.time += dt;
    const gusting = this.gusting();
    this.drift += WIND * gusting * dt;
    // in the air's cells a second
    const wind = WIND * gusting * air.read.w;
    if (this.energy > REST) {
      // what drives the air lets go, and the air is braked, as it comes to rest: nothing is left to stop with a jolt
      const awake = Math.min(1, (this.energy - REST) / (5 * REST));
      this.pass("curl", curl, (u) => this.bind(0, air.read, u("uVelocity")));
      this.pass("forces", air.write, (u) => {
        this.bind(0, air.read, u("uVelocity"));
        this.bind(1, curl, u("uCurl"));
        this.bind(2, cloud.read, u("uDensity"));
        gl.uniform1f(u("uSwirl"), SWIRL * awake);
        gl.uniform1f(u("uLift"), LIFT * awake);
        gl.uniform1f(u("uDt"), dt);
      });
      swap(air);
      this.pass("divergence", divergence, (u) => this.bind(0, air.read, u("uVelocity")));
      for (let i = 0; i < JACOBI; i++) {
        this.pass("pressure", pressure.write, (u) => {
          this.bind(0, pressure.read, u("uPressure"));
          this.bind(1, divergence, u("uDivergence"));
          gl.uniform1f(u("uKeep"), i === 0 ? Math.exp(-13 * dt) : 1);
        });
        swap(pressure);
      }
      this.pass("gradient", air.write, (u) => {
        this.bind(0, pressure.read, u("uPressure"));
        this.bind(1, air.read, u("uVelocity"));
      });
      swap(air);
      this.pass("advect", air.write, (u) => {
        this.bind(0, air.read, u("uVelocity"));
        gl.uniform1f(u("uWind"), wind);
        gl.uniform1f(u("uDt"), dt);
        gl.uniform1f(u("uKeep"), Math.exp(-(AIR_FADE + 3 * (1 - awake)) * dt));
      });
      swap(air);
      this.energy *= Math.exp(-AIR_FADE * dt);
      // the air has come to rest: no more solving until something stirs it
      if (this.energy <= REST) this.clear([air.read, air.write, pressure.read, pressure.write], 1);
    }
    // the two maps of the wisps: each starts over as its life ends, when the picture shows the other alone
    const before = this.phase;
    this.phase += dt / WISP_LIFE;
    const first = Math.floor(this.phase) === Math.floor(before) ? 1 : 0;
    const second = Math.floor(this.phase + 0.5) === Math.floor(before + 0.5) ? 1 : 0;
    if (this.phase >= 2) this.phase -= 2;
    this.rested = this.energy > REST ? 0 : this.rested + dt;
    if (this.rested < WISP_LIFE + 1) {
      this.pass("carry", carry.write, (u) => {
        this.bind(0, air.read, u("uVelocity"));
        this.bind(1, carry.read, u("uCarry"));
        gl.uniform2f(u("uGrid"), 1 / air.read.w, 1 / air.read.h);
        gl.uniform1f(u("uAspect"), this.aspect());
        gl.uniform1f(u("uWind"), WIND * gusting * this.aspect());
        gl.uniform1f(u("uDt"), dt);
        gl.uniform2f(u("uKeep"), first, second);
      });
      swap(carry);
    }
    this.pass(
      "density",
      cloud.write,
      (u) => {
        this.bind(0, air.read, u("uVelocity"));
        this.bind(1, cloud.read, u("uDensity"));
        this.banksUniforms(u);
        gl.uniform2f(u("uGrid"), 1 / air.read.w, 1 / air.read.h);
        gl.uniform1f(u("uWind"), wind);
        gl.uniform1f(u("uDt"), dt);
        gl.uniform1f(u("uGather"), GATHER);
        gl.uniform1f(u("uDissolve"), DISSOLVE);
        gl.uniform1f(u("uForget"), Math.exp(-dt / MEMORY));
        gl.uniform1f(u("uThin"), THIN);
      },
      cloud.read,
    );
    swap(cloud);
  }
  /** The banks as they are now, without the air: the picture for reduced motion. */
  still() {
    const { cloud, carry } = this;
    if (cloud === null || carry === null) return;
    this.clear([carry.read, carry.write], 0);
    this.pass("banks", cloud.write, (u) => this.banksUniforms(u));
    swap(cloud);
  }
  /** Lights the clouds onto the canvas of `width` × `height` pixels. */
  draw(width, height) {
    const { cloud, carry } = this;
    if (cloud === null || carry === null) return;
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, width, height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    const look = this.look;
    // each map counts most in the middle of its life and not at all where it starts over
    const first = 1 - Math.abs(2 * (this.phase % 1) - 1);
    this.pass(
      "display",
      null,
      (u) => {
        this.bind(0, cloud.read, u("uDensity"));
        this.bind(1, carry.read, u("uCarry"));
        gl.uniform1f(u("uTime"), this.time);
        gl.uniform1f(u("uAspect"), this.aspect());
        gl.uniform1f(u("uDrift"), this.drift);
        gl.uniform2f(u("uPhase"), first, 1 - first);
        gl.uniform2f(u("uSun"), 0.5, 1.2);
        gl.uniform2f(u("uPixel"), 1 / width, 1 / height);
        gl.uniform3f(u("uLit"), ...look.lit);
        gl.uniform3f(u("uShade"), ...look.shade);
        gl.uniform3f(u("uRim"), ...look.rim);
        gl.uniform1f(u("uOpacity"), look.opacity);
        gl.uniform1f(u("uRoom"), look.room);
      },
      { w: width, h: height },
    );
  }
  /** Whether the GPU is a CPU in disguise (SwiftShader, llvmpipe, Windows' basic renderer): no live air there. */
  software() {
    const gl = this.gl;
    const info = gl.getExtension("WEBGL_debug_renderer_info");
    const name = String(gl.getParameter(info === null ? gl.RENDERER : info.UNMASKED_RENDERER_WEBGL) ?? "");
    return /swiftshader|llvmpipe|softpipe|software|basic render/i.test(name);
  }
  /** Whether the air still moves (the loop may then run at full rate). */
  stirred() {
    return this.energy > REST;
  }
  dispose() {
    const gl = this.gl;
    this.releaseTargets();
    for (const { program } of this.programs.values()) gl.deleteProgram(program);
    for (const s of this.shapes) gl.deleteShader(s);
    if (this.quad !== null) gl.deleteBuffer(this.quad);
    if (this.vao !== null) gl.deleteVertexArray(this.vao);
    this.programs.clear();
    this.shapes = [];
  }
}

// ---- the canvas in the hero ----------------------------------------------------

/** The canvas's pixels per CSS pixel, and the widest it gets: the clouds are soft. */
const SCALE = 0.6;
const MAX_WIDTH = 1280;
/**
 * A drag's reach, as a share of the hero's height: this for a slow pointer,
 * up to `SWEEP` times it for one that crosses `SWIFT` heights a second. The
 * air takes up `FORCE` of the pointer's speed, within `GRIP` seconds.
 */
const REACH = 0.034;
const SWEEP = 2.6;
const SWIFT = 3;
const FORCE = 0.8;
const GRIP = 0.045;

const dark = () => getComputedStyle(document.documentElement).colorScheme.includes("dark");
const forced = () => {
  try {
    return localStorage.getItem("baylee-clouds") === "live";
  } catch {
    return false;
  }
};

/** Puts the clouds into the hero's sky. */
function start(hero) {
  const sky = hero.querySelector(".sky");
  if (!sky || sky.querySelector(".sky-clouds")) return;
  const canvas = document.createElement("canvas");
  canvas.className = "sky-clouds";
  sky.querySelector(".sky-painting")?.after(canvas);
  run(hero, canvas);
}

function run(hero, canvas) {
  const clouds = Clouds.create(canvas);
  if (clouds === null) {
    canvas.dataset.renderer = "none";
    return;
  }
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches || (clouds.software() && !forced());
  canvas.dataset.renderer = still ? "still" : "live";

  let lost = false;
  let frame = 0;
  let last = performance.now();
  const look = () => clouds.setLook(dark() ? BLUE_HOUR : SUNLIGHT);
  look();

  const size = () => {
    const width = Math.max(1, Math.round(Math.min(canvas.clientWidth * Math.min(devicePixelRatio || 1, 2) * SCALE, MAX_WIDTH)));
    const height = Math.max(1, Math.round((width * canvas.clientHeight) / Math.max(1, canvas.clientWidth)));
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
    if (!lost) clouds.draw(canvas.width, canvas.height);
  };
  const picture = () => {
    size();
    if (!lost) {
      clouds.still();
      paint();
    }
  };

  // the pointer's path since the last frame, in the hero's 0..1 (from the bottom left)
  let trail = [];
  let prev = null;
  const at = (event) => {
    const r = canvas.getBoundingClientRect();
    return {
      x: (event.clientX - r.left) / Math.max(1, r.width),
      y: 1 - (event.clientY - r.top) / Math.max(1, r.height),
      t: event.timeStamp,
    };
  };
  const move = (event) => {
    const p = at(event);
    if (p.x < -0.05 || p.x > 1.05 || p.y < -0.05 || p.y > 1.05) {
      prev = null;
      return;
    }
    if (trail.length < 16) trail.push(p);
  };
  const leave = () => {
    prev = null;
    trail = [];
  };
  const press = (event) => {
    if (event.button !== 0) return;
    const p = at(event);
    clouds.gust(p.x, p.y, 0.05, 1.5);
  };

  const stir = () => {
    const aspect = canvas.clientWidth / Math.max(1, canvas.clientHeight);
    for (const p of trail) {
      if (prev !== null && p.t > prev.t) {
        const dt = Math.min((p.t - prev.t) / 1000, 0.1);
        // heights a second: the x distance in widths, times the aspect
        const vx = ((p.x - prev.x) * aspect) / dt;
        const vy = (p.y - prev.y) / dt;
        // a swift stroke moves more air than a slow one
        const swift = Math.min(1, Math.hypot(vx, vy) / SWIFT);
        const reach = REACH * (1 + (SWEEP - 1) * swift * swift * (3 - 2 * swift));
        // what the air takes up of it depends on the time the stroke took, not on how many events told of it
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
    // calm air needs no more than thirty pictures a second, stirred air no
    // more than a hundred: a faster screen (120 Hz and up) shows every
    // second frame, so the air moves there as it does at sixty
    const calm = !clouds.stirred() && trail.length === 0;
    if (paused() || lost || dt < (calm ? 1 / 31 : 1 / 100)) return;
    last = now;
    size();
    stir();
    clouds.step(dt);
    paint();
  };

  const themed = () => {
    look();
    if (still) picture();
  };
  // The sky goes with its page (`nav.js` puts another in place without a
  // new document): the loop stops, what watched the document lets go, and
  // the context is given back, since a browser keeps only so many.
  const leaving = new AbortController();
  const theme = new MutationObserver(themed);
  theme.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", themed, { signal: leaving.signal });
  const sized = new ResizeObserver(() => {
    if (still) picture();
    else size();
  });
  sized.observe(canvas);
  document.addEventListener(
    "baylee:leave",
    () => {
      leaving.abort();
      theme.disconnect();
      sized.disconnect();
      lost = true;
      cancelAnimationFrame(frame);
      canvas.getContext("webgl2")?.getExtension("WEBGL_lose_context")?.loseContext();
    },
    { once: true },
  );
  canvas.addEventListener("webglcontextlost", (event) => {
    // the painting alone from here on: the page is read once, a context
    // given back is not worth a second start
    event.preventDefault();
    lost = true;
    cancelAnimationFrame(frame);
    canvas.dataset.renderer = "none";
  });

  if (still) picture();
  else {
    hero.addEventListener("pointermove", move, { passive: true });
    hero.addEventListener("pointerleave", leave, { passive: true });
    hero.addEventListener("pointerdown", press, { passive: true });
    size();
    frame = requestAnimationFrame(tick);
  }
  // it fades in over the painting once the first picture is drawn
  requestAnimationFrame(() => canvas.classList.add("on"));
}

/** The sky of the page on screen, once the browser is idle; each page that has a hero starts its own. */
function page() {
  const hero = document.querySelector(".hero");
  if (!hero || !matchMedia("(hover: hover) and (pointer: fine)").matches) return;
  const idle = window.requestIdleCallback ?? ((run) => setTimeout(run, 200));
  idle(() => {
    if (hero.isConnected) start(hero);
  });
}

if (document.readyState === "complete") page();
else addEventListener("load", page, { once: true });
document.addEventListener("baylee:page", page);
