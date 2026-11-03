// The hero's clouds on the GPU (fluid.ts): Stam's stable fluids on a coarse
// grid (advection, vorticity confinement, a pressure projection) carry a
// finer cloud density, which relaxes back to slowly drifting cumulus banks
// where the air is calm; the last pass lights it towards the sun. GLSL ES
// 3.00 for WebGL 2. Velocities are in grid cells per second.

const HEAD = "#version 300 es\nprecision highp float;\nprecision highp sampler2D;\n";

export const VERTEX = `${HEAD}
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

/** The cumulus banks the density relaxes to. */
const BANKS_AT = `${NOISE}
uniform float uTime;
uniform float uAspect;
uniform float uDrift;
uniform float uCover;
/** Cumulus banks drifting with the wind, thinner behind the words, fuller towards the edges and the foot. */
float banks(vec2 uv) {
  vec2 p = vec2((uv.x - uDrift) * uAspect, uv.y) * 3.1;
  vec2 warp = vec2(fbm(p * 0.7 + vec2(0.0, uTime * 0.011), 4), fbm(p * 0.7 + vec2(5.2, 1.3) - uTime * 0.009, 4));
  float n = fbm(p + 1.8 * warp, 6);
  vec2 c = (uv - vec2(0.5, 0.58)) * vec2(1.05, 1.55);
  float open = smoothstep(0.12, 0.92, length(c));
  float cover = uCover * mix(0.6, 1.0, open) + 0.1 * smoothstep(0.4, 0.0, uv.y) - 0.08 * smoothstep(0.8, 1.0, uv.y);
  return smoothstep(1.0 - cover, 1.22 - cover, n);
}
`;

export const SPLAT = `${HEAD}
in vec2 vUv;
uniform sampler2D uTarget;
uniform float uAspect;
uniform vec2 uPoint;
uniform vec3 uValue;
uniform float uRadius;
out vec4 color;
void main() {
  vec2 p = vUv - uPoint;
  p.x *= uAspect;
  vec3 base = texture(uTarget, vUv).xyz;
  color = vec4(base + exp(-dot(p, p) / uRadius) * uValue, 1.0);
}`;

/** Velocity carried by itself and the wind, fading. */
export const ADVECT = `${HEAD}
in vec2 vUv;
uniform sampler2D uVelocity;
uniform vec2 uTexel;
uniform vec2 uWind;
uniform float uDt;
uniform float uKeep;
out vec4 color;
void main() {
  vec2 back = vUv - uDt * (texture(uVelocity, vUv).xy + uWind) * uTexel;
  color = vec4(texture(uVelocity, back).xy * uKeep, 0.0, 1.0);
}`;

/** The density carried by the air, healing towards the banks where the air is calm. */
export const DENSITY = `${HEAD}
in vec2 vUv;
uniform sampler2D uVelocity;
uniform sampler2D uDensity;
uniform vec2 uGrid;
uniform vec2 uWind;
uniform float uDt;
uniform float uHeal;
out vec4 color;
${BANKS_AT}
void main() {
  vec2 v = texture(uVelocity, vUv).xy;
  vec2 back = vUv - uDt * (v + uWind) * uGrid;
  float d = texture(uDensity, back).r;
  float calm = 1.0 - smoothstep(4.0, 40.0, length(v));
  d = mix(d, banks(vUv), uHeal * (0.25 + 0.75 * calm));
  color = vec4(clamp(d, 0.0, 1.0), 0.0, 0.0, 1.0);
}`;

/** The banks alone: the first density, and the still picture for reduced motion. */
export const BANKS = `${HEAD}
in vec2 vUv;
out vec4 color;
${BANKS_AT}
void main() { color = vec4(banks(vUv), 0.0, 0.0, 1.0); }`;

export const CURL = `${HEAD}
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

/** Vorticity confinement: gives back the small swirls the coarse grid smears out. */
export const VORTICITY = `${HEAD}
in vec2 vUv;
in vec2 vL;
in vec2 vR;
in vec2 vT;
in vec2 vB;
uniform sampler2D uVelocity;
uniform sampler2D uCurl;
uniform float uStrength;
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
  force *= uStrength * c * vec2(1.0, -1.0);
  vec2 v = texture(uVelocity, vUv).xy + force * uDt;
  color = vec4(clamp(v, -600.0, 600.0), 0.0, 1.0);
}`;

/** Divergence, the walls reflecting the flow. */
export const DIVERGENCE = `${HEAD}
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

/** The previous pressure, faded: a warm start for the Jacobi iterations. */
export const FADE = `${HEAD}
in vec2 vUv;
uniform sampler2D uTexture;
uniform float uKeep;
out vec4 color;
void main() { color = vec4(texture(uTexture, vUv).x * uKeep, 0.0, 0.0, 1.0); }`;

export const PRESSURE = `${HEAD}
in vec2 vL;
in vec2 vR;
in vec2 vT;
in vec2 vB;
in vec2 vUv;
uniform sampler2D uPressure;
uniform sampler2D uDivergence;
out vec4 color;
void main() {
  float p = texture(uPressure, vL).x + texture(uPressure, vR).x + texture(uPressure, vT).x + texture(uPressure, vB).x;
  color = vec4(0.25 * (p - texture(uDivergence, vUv).x), 0.0, 0.0, 1.0);
}`;

/** Subtract the pressure gradient: what remains does not compress. */
export const GRADIENT = `${HEAD}
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
 * The clouds as they are seen: the density sharpened into soft edges with
 * billows that ride the flow, shaded like a surface (its normal from the
 * thickness) and by its own depth towards the sun, so tops shine, cores and
 * undersides fall into blue shade and thin edges glow; premultiplied.
 */
export const DISPLAY = `${HEAD}
in vec2 vUv;
uniform sampler2D uDensity;
uniform sampler2D uVelocity;
uniform float uTime;
uniform float uAspect;
uniform float uDrift;
uniform vec2 uSun;
uniform vec2 uPixel;
uniform vec3 uLit;
uniform vec3 uShade;
uniform vec3 uRim;
uniform float uOpacity;
out vec4 color;
${NOISE}
float density(vec2 uv) { return texture(uDensity, uv).r; }
float billows(vec2 q) { return 0.7 * fbm(q, 5) + 0.3 * (1.0 - abs(2.0 * fbm(q * 1.7 + 7.3, 3) - 1.0)); }
void main() {
  vec2 v = texture(uVelocity, vUv).xy;
  vec2 q = vec2((vUv.x - uDrift) * uAspect, vUv.y) * 7.0 + vec2(uTime * 0.025, 0.0) - v * 0.01;
  float base = density(vUv);
  float h = base + (billows(q) - 0.5) * (0.3 + 0.5 * base);
  float d = smoothstep(0.17, 0.6, h);
  if (d <= 0.002) { color = vec4(0.0); return; }
  // a soft surface from the carried density alone, the billows only in its edge
  vec2 e = uPixel * 7.0;
  float gx = density(vUv + vec2(e.x, 0.0)) - density(vUv - vec2(e.x, 0.0));
  float gy = density(vUv + vec2(0.0, e.y)) - density(vUv - vec2(0.0, e.y));
  vec3 n = normalize(vec3(-gx, -gy, 0.35));
  vec2 toward = normalize(uSun - vUv);
  vec3 sun = normalize(vec3(toward * 0.75, 0.66));
  float diffuse = clamp(dot(n, sun), 0.0, 1.0);
  float depth = 0.0;
  for (int i = 1; i <= 6; i++) depth += density(vUv + toward * 0.018 * float(i));
  float light = exp(-depth * 0.3);
  float lit = clamp((0.42 + 0.58 * diffuse) * (0.35 + 0.65 * light) + 0.12 * (billows(q * 1.3) - 0.5), 0.0, 1.0);
  vec3 c = mix(uShade, uLit, lit);
  c += uRim * pow(1.0 - d, 1.8) * light * 0.5;
  float a = d * uOpacity;
  color = vec4(c * a, a);
}`;
