// The start page's sky, alive: clouds that drift slowly on the wind and that
// the pointer stirs (the old site's, PR #62; owner, 11.11.2026: effects for
// the header). The air is a small real fluid, Stam's stable fluids on the
// GPU: the pointer's motion pushes it along its path, the push rolls up into
// eddies that vorticity confinement keeps, and the clouds are carried by
// that air until it calms and they heal into their banks again. A press
// sends a gust outwards.
//
// It starts once the page has loaded and the browser is idle, so the
// painting stays the largest paint and nothing here delays it; and only
// where a pointer can stir the air (a mouse or a pen), so a phone spends
// neither battery nor a download's worth of work on it. Paused
// while the hero is away (`data-away`), the tab hidden or a dialog open; one
// still picture under reduced motion and on a software renderer, where the
// air would cost a CPU core (tests may ask for it anyway: localStorage
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
const SPLAT = `${HEAD}
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
const ADVECT = `${HEAD}
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
const DENSITY = `${HEAD}
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
const BANKS = `${HEAD}
in vec2 vUv;
out vec4 color;
${BANKS_AT}
void main() { color = vec4(banks(vUv), 0.0, 0.0, 1.0); }`;
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
/** Vorticity confinement: gives back the small swirls the coarse grid smears out. */
const VORTICITY = `${HEAD}
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
/** The previous pressure, faded: a warm start for the Jacobi iterations. */
const FADE = `${HEAD}
in vec2 vUv;
uniform sampler2D uTexture;
uniform float uKeep;
out vec4 color;
void main() { color = vec4(texture(uTexture, vUv).x * uKeep, 0.0, 0.0, 1.0); }`;
const PRESSURE = `${HEAD}
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
 * The clouds as they are seen: the density sharpened into soft edges with
 * billows that ride the flow, shaded like a surface (its normal from the
 * thickness) and by its own depth towards the sun, so tops shine, cores and
 * undersides fall into blue shade and thin edges glow; premultiplied.
 */
const DISPLAY = `${HEAD}
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

// ---- the air and the clouds ------------------------------------------------------

const SUNLIGHT = { lit: [1, 0.99, 0.965], shade: [0.7, 0.76, 0.86], rim: [1, 0.91, 0.7], opacity: 0.9, cover: 0.58 };
const BLUE_HOUR = { lit: [0.74, 0.8, 0.93], shade: [0.2, 0.26, 0.4], rim: [0.98, 0.82, 0.62], opacity: 0.72, cover: 0.54 };
/** The air's grid height, the cloud density's, the pressure iterations. */
const AIR = 112;
const CLOUD = 300;
const JACOBI = 20;
/** The wind, in the hero's widths a second: a bank crosses it in about three minutes. */
const WIND = 0.0055;
/** How fast the air forgets, and how fast clouds heal where it is calm (per second). */
const AIR_FADE = 0.42;
const HEAL = 0.3;
const SWIRL = 30;
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
  /** How much the air still moves, 0 to 1; below a trace the solve is skipped. */
  energy = 0;
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
      splat: SPLAT,
      advect: ADVECT,
      density: DENSITY,
      banks: BANKS,
      curl: CURL,
      vorticity: VORTICITY,
      divergence: DIVERGENCE,
      fade: FADE,
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
  target(w, h) {
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
    gl.viewport(0, 0, w, h);
    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    return ok ? { tex, fbo, w, h } : null;
  }
  double(w, h) {
    const read = this.target(w, h);
    const write = this.target(w, h);
    return read === null || write === null ? null : { read, write };
  }
  releaseTargets() {
    for (const t of this.textures) this.gl.deleteTexture(t);
    for (const f of this.buffers) this.gl.deleteFramebuffer(f);
    this.textures = [];
    this.buffers = [];
    this.air = this.cloud = this.pressure = null;
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
    this.cloud = this.double(cloud.w, cloud.h);
    if (this.air === null || this.pressure === null || this.divergence === null || this.curl === null || this.cloud === null) return false;
    this.energy = 0;
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
  /** Pushes the air at `x, y` (0 to 1, from the bottom left) by `vx, vy` (heights a second) within `radius` (a height's share). */
  push(x, y, vx, vy, radius) {
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
    this.pass("splat", air.write, (u) => {
      this.bind(0, air.read, u("uTarget"));
      gl.uniform1f(u("uAspect"), this.aspect());
      gl.uniform2f(u("uPoint"), x, y);
      gl.uniform3f(u("uValue"), cx, cy, 0);
      gl.uniform1f(u("uRadius"), radius * radius);
    });
    swap(air);
    this.energy = 1;
  }
  /** One step of `dt` seconds. */
  step(dt) {
    const { air, cloud, pressure, divergence, curl } = this;
    if (air === null || cloud === null || pressure === null || divergence === null || curl === null) return;
    const gl = this.gl;
    dt = Math.min(Math.max(dt, 0), 1 / 30);
    this.time += dt;
    this.drift += WIND * dt;
    const wind = WIND * air.read.w;
    if (this.energy > 0.004) {
      this.pass("curl", curl, (u) => this.bind(0, air.read, u("uVelocity")));
      this.pass("vorticity", air.write, (u) => {
        this.bind(0, air.read, u("uVelocity"));
        this.bind(1, curl, u("uCurl"));
        gl.uniform1f(u("uStrength"), SWIRL);
        gl.uniform1f(u("uDt"), dt);
      });
      swap(air);
      this.pass("divergence", divergence, (u) => this.bind(0, air.read, u("uVelocity")));
      this.pass("fade", pressure.write, (u) => {
        this.bind(0, pressure.read, u("uTexture"));
        gl.uniform1f(u("uKeep"), 0.8);
      });
      swap(pressure);
      for (let i = 0; i < JACOBI; i++) {
        this.pass("pressure", pressure.write, (u) => {
          this.bind(0, pressure.read, u("uPressure"));
          this.bind(1, divergence, u("uDivergence"));
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
        gl.uniform2f(u("uWind"), wind, 0);
        gl.uniform1f(u("uDt"), dt);
        gl.uniform1f(u("uKeep"), Math.exp(-AIR_FADE * dt));
      });
      swap(air);
      this.energy *= Math.exp(-AIR_FADE * dt);
      // the air has come to rest: no more solving until something stirs it
      if (this.energy <= 0.004)
        for (const t of [air.read, air.write, pressure.read, pressure.write]) {
          gl.bindFramebuffer(gl.FRAMEBUFFER, t.fbo);
          gl.viewport(0, 0, t.w, t.h);
          gl.clearColor(0, 0, 0, 1);
          gl.clear(gl.COLOR_BUFFER_BIT);
        }
    }
    this.pass(
      "density",
      cloud.write,
      (u) => {
        this.bind(0, air.read, u("uVelocity"));
        this.bind(1, cloud.read, u("uDensity"));
        this.banksUniforms(u);
        gl.uniform2f(u("uGrid"), 1 / air.read.w, 1 / air.read.h);
        gl.uniform2f(u("uWind"), wind, 0);
        gl.uniform1f(u("uDt"), dt);
        gl.uniform1f(u("uHeal"), 1 - Math.exp(-HEAL * dt));
      },
      cloud.read,
    );
    swap(cloud);
  }
  /** The banks as they are now, without the air: the picture for reduced motion. */
  still() {
    const cloud = this.cloud;
    if (cloud === null) return;
    this.pass("banks", cloud.write, (u) => this.banksUniforms(u));
    swap(cloud);
  }
  /** Lights the clouds onto the canvas of `width` × `height` pixels. */
  draw(width, height) {
    const { air, cloud } = this;
    if (air === null || cloud === null) return;
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, width, height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    const look = this.look;
    this.pass(
      "display",
      null,
      (u) => {
        this.bind(0, cloud.read, u("uDensity"));
        this.bind(1, air.read, u("uVelocity"));
        gl.uniform1f(u("uTime"), this.time);
        gl.uniform1f(u("uAspect"), this.aspect());
        gl.uniform1f(u("uDrift"), this.drift);
        gl.uniform2f(u("uSun"), 0.5, 1.2);
        gl.uniform2f(u("uPixel"), 1 / width, 1 / height);
        gl.uniform3f(u("uLit"), ...look.lit);
        gl.uniform3f(u("uShade"), ...look.shade);
        gl.uniform3f(u("uRim"), ...look.rim);
        gl.uniform1f(u("uOpacity"), look.opacity);
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
    return this.energy > 0.004;
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
/** A push's reach, as a share of the hero's height, and its strength. */
const REACH = 0.05;
const FORCE = 0.75;

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
    if (trail.length < 64) trail.push(p);
  };
  const leave = () => {
    prev = null;
    trail = [];
  };
  const press = (event) => {
    if (event.button !== 0) return;
    const p = at(event);
    const aspect = canvas.clientWidth / Math.max(1, canvas.clientHeight);
    // a ring of pushes outwards, alternately turned, so the gust rolls up at its rim
    for (let i = 0; i < 12; i += 1) {
      const a = (i / 12) * Math.PI * 2;
      const dx = Math.cos(a);
      const dy = Math.sin(a);
      const turn = i % 2 === 0 ? 0.35 : -0.35;
      clouds.push(p.x + (dx * 0.03) / aspect, p.y + dy * 0.03, (dx - dy * turn) * 1.4, (dy + dx * turn) * 1.4, REACH * 0.7);
    }
  };

  const stir = () => {
    const aspect = canvas.clientWidth / Math.max(1, canvas.clientHeight);
    for (const p of trail) {
      if (prev !== null && p.t > prev.t) {
        const dt = Math.min((p.t - prev.t) / 1000, 0.1);
        // heights a second: the x distance in widths, times the aspect
        const vx = ((p.x - prev.x) * aspect) / dt;
        const vy = (p.y - prev.y) / dt;
        const span = Math.hypot((p.x - prev.x) * aspect, p.y - prev.y);
        const steps = Math.min(6, Math.max(1, Math.ceil(span / (REACH * 0.6))));
        for (let s = 1; s <= steps; s += 1) {
          const k = s / steps;
          clouds.push(prev.x + (p.x - prev.x) * k, prev.y + (p.y - prev.y) * k, (vx * FORCE) / steps, (vy * FORCE) / steps, REACH);
        }
      }
      prev = p;
    }
    trail = [];
  };

  const paused = () => document.hidden || hero.dataset.away !== undefined || document.querySelector("dialog[open]") !== null;
  const tick = (now) => {
    frame = requestAnimationFrame(tick);
    const dt = (now - last) / 1000;
    // calm air needs no more than thirty pictures a second
    if (paused() || lost || (!clouds.stirred() && trail.length === 0 && dt < 1 / 31)) return;
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
  new MutationObserver(themed).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", themed);
  new ResizeObserver(() => {
    if (still) picture();
    else size();
  }).observe(canvas);
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

const hero = document.querySelector(".hero");
if (hero && matchMedia("(hover: hover) and (pointer: fine)").matches) {
  const idle = window.requestIdleCallback ?? ((run) => setTimeout(run, 200));
  const begin = () => idle(() => start(hero));
  if (document.readyState === "complete") begin();
  else addEventListener("load", begin, { once: true });
}
