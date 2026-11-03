// The hero's clouds on WebGL 2: the air on a coarse grid, the cloud density on
// a finer one, both in half-float textures (shaders.ts). `step` moves the air
// (pointer pushes, swirls kept by vorticity confinement, a pressure solve so
// it does not compress) and carries the clouds with it; `draw` lights them.
// Air that nobody stirs comes to rest, and then only the clouds drift.
import { ADVECT, BANKS, CURL, DENSITY, DISPLAY, DIVERGENCE, FADE, GRADIENT, PRESSURE, SPLAT, VERTEX, VORTICITY } from "./shaders";

type Vec3 = readonly [number, number, number];
/** The colours and amounts of one colour scheme. */
export interface Look { lit: Vec3; shade: Vec3; rim: Vec3; opacity: number; cover: number }

export const SUNLIGHT: Look = { lit: [1, 0.99, 0.965], shade: [0.7, 0.76, 0.86], rim: [1, 0.91, 0.7], opacity: 0.9, cover: 0.58 };
export const BLUE_HOUR: Look = { lit: [0.74, 0.8, 0.93], shade: [0.2, 0.26, 0.4], rim: [0.98, 0.82, 0.62], opacity: 0.72, cover: 0.54 };

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

interface Target { tex: WebGLTexture; fbo: WebGLFramebuffer; w: number; h: number }
interface Double { read: Target; write: Target }
interface Program { program: WebGLProgram; uniforms: Map<string, WebGLUniformLocation> }

function swap(d: Double) { const t = d.read; d.read = d.write; d.write = t; }

export class Clouds {
  private programs = new Map<string, Program>();
  private air: Double | null = null;
  private cloud: Double | null = null;
  private pressure: Double | null = null;
  private divergence: Target | null = null;
  private curl: Target | null = null;
  private textures: WebGLTexture[] = [];
  private buffers: WebGLFramebuffer[] = [];
  private shapes: WebGLShader[] = [];
  private vao: WebGLVertexArrayObject | null = null;
  private quad: WebGLBuffer | null = null;
  private time = 0;
  private drift = 0;
  /** How much the air still moves, 0 to 1; below a trace the solve is skipped. */
  private energy = 0;
  private look: Look = SUNLIGHT;

  private constructor(private gl: WebGL2RenderingContext) {}

  /** The clouds on `canvas`, or null where WebGL 2 or a renderable half-float texture is missing. */
  static create(canvas: HTMLCanvasElement): Clouds | null {
    let gl: WebGL2RenderingContext | null = null;
    try { gl = canvas.getContext("webgl2", { alpha: true, premultipliedAlpha: true, antialias: false, depth: false, stencil: false, preserveDrawingBuffer: false, powerPreference: "low-power" }); }
    catch { return null; }
    if (gl === null || gl.isContextLost()) return null;
    if (gl.getExtension("EXT_color_buffer_float") === null && gl.getExtension("EXT_color_buffer_half_float") === null) return null;
    const clouds = new Clouds(gl);
    if (!clouds.build()) { clouds.dispose(); return null; }
    return clouds;
  }

  private build(): boolean {
    const gl = this.gl;
    const sources: Record<string, string> = { splat: SPLAT, advect: ADVECT, density: DENSITY, banks: BANKS, curl: CURL, vorticity: VORTICITY, divergence: DIVERGENCE, fade: FADE, pressure: PRESSURE, gradient: GRADIENT, display: DISPLAY };
    const vertex = this.shader(gl.VERTEX_SHADER, VERTEX);
    if (vertex === null) return false;
    for (const [name, source] of Object.entries(sources)) {
      const fragment = this.shader(gl.FRAGMENT_SHADER, source);
      const program = gl.createProgram();
      if (fragment === null) return false;
      gl.attachShader(program, vertex); gl.attachShader(program, fragment);
      gl.bindAttribLocation(program, 0, "aPosition");
      gl.linkProgram(program);
      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) { gl.deleteProgram(program); return false; }
      const uniforms = new Map<string, WebGLUniformLocation>();
      const count = gl.getProgramParameter(program, gl.ACTIVE_UNIFORMS) as number;
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

  private shader(kind: number, source: string): WebGLShader | null {
    const gl = this.gl;
    const shader = gl.createShader(kind);
    if (shader === null) return null;
    this.shapes.push(shader);
    gl.shaderSource(shader, source); gl.compileShader(shader);
    return gl.getShaderParameter(shader, gl.COMPILE_STATUS) ? shader : null;
  }

  private target(w: number, h: number): Target | null {
    const gl = this.gl;
    const tex = gl.createTexture();
    const fbo = gl.createFramebuffer();
    this.textures.push(tex); this.buffers.push(fbo);
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA16F, w, h, 0, gl.RGBA, gl.HALF_FLOAT, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    const ok = gl.checkFramebufferStatus(gl.FRAMEBUFFER) === gl.FRAMEBUFFER_COMPLETE;
    gl.viewport(0, 0, w, h); gl.clearColor(0, 0, 0, 1); gl.clear(gl.COLOR_BUFFER_BIT);
    return ok ? { tex, fbo, w, h } : null;
  }

  private double(w: number, h: number): Double | null {
    const read = this.target(w, h);
    const write = this.target(w, h);
    return read === null || write === null ? null : { read, write };
  }

  private releaseTargets() {
    for (const t of this.textures) this.gl.deleteTexture(t);
    for (const f of this.buffers) this.gl.deleteFramebuffer(f);
    this.textures = []; this.buffers = [];
    this.air = this.cloud = this.pressure = null; this.divergence = this.curl = null;
  }

  /** Grids for a canvas of this shape; the clouds start as the banks. False when a texture cannot be drawn to. */
  resize(width: number, height: number): boolean {
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

  setLook(look: Look) { this.look = look; }

  private aspect(): number { return this.air === null ? 1 : this.air.read.w / this.air.read.h; }

  private use(name: string): Program {
    const p = this.programs.get(name);
    if (p === undefined) throw new Error(`no program ${name}`);
    this.gl.useProgram(p.program);
    return p;
  }

  /** Draws `name` into `out` (null: the canvas); `set` gives its uniforms. */
  private pass(name: string, out: Target | null, set: (u: (n: string) => WebGLUniformLocation | null, p: Program) => void, size?: { w: number; h: number }) {
    const gl = this.gl;
    const p = this.use(name);
    const grid = size ?? out ?? { w: 1, h: 1 };
    const at = (n: string) => p.uniforms.get(n) ?? null;
    gl.uniform2f(at("uTexel"), 1 / grid.w, 1 / grid.h);
    set(at, p);
    gl.bindFramebuffer(gl.FRAMEBUFFER, out?.fbo ?? null);
    if (out !== null) gl.viewport(0, 0, out.w, out.h);
    gl.bindVertexArray(this.vao);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  private bind(unit: number, target: Target, at: WebGLUniformLocation | null) {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + unit);
    gl.bindTexture(gl.TEXTURE_2D, target.tex);
    gl.uniform1i(at, unit);
  }

  private banksUniforms(u: (n: string) => WebGLUniformLocation | null) {
    const gl = this.gl;
    gl.uniform1f(u("uTime"), this.time);
    gl.uniform1f(u("uAspect"), this.aspect());
    gl.uniform1f(u("uDrift"), this.drift);
    gl.uniform1f(u("uCover"), this.look.cover);
  }

  /** Pushes the air at `x, y` (0 to 1, from the bottom left) by `vx, vy` (heights a second) within `radius` (a height's share). */
  push(x: number, y: number, vx: number, vy: number, radius: number) {
    const air = this.air;
    if (air === null) return;
    const gl = this.gl;
    const limit = 520;
    let cx = vx * air.read.h;
    let cy = vy * air.read.h;
    const speed = Math.hypot(cx, cy);
    if (speed > limit) { cx *= limit / speed; cy *= limit / speed; }
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
  step(dt: number) {
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
        this.bind(0, air.read, u("uVelocity")); this.bind(1, curl, u("uCurl"));
        gl.uniform1f(u("uStrength"), SWIRL); gl.uniform1f(u("uDt"), dt);
      });
      swap(air);
      this.pass("divergence", divergence, (u) => this.bind(0, air.read, u("uVelocity")));
      this.pass("fade", pressure.write, (u) => { this.bind(0, pressure.read, u("uTexture")); gl.uniform1f(u("uKeep"), 0.8); });
      swap(pressure);
      for (let i = 0; i < JACOBI; i++) {
        this.pass("pressure", pressure.write, (u) => { this.bind(0, pressure.read, u("uPressure")); this.bind(1, divergence, u("uDivergence")); });
        swap(pressure);
      }
      this.pass("gradient", air.write, (u) => { this.bind(0, pressure.read, u("uPressure")); this.bind(1, air.read, u("uVelocity")); });
      swap(air);
      this.pass("advect", air.write, (u) => {
        this.bind(0, air.read, u("uVelocity"));
        gl.uniform2f(u("uWind"), wind, 0); gl.uniform1f(u("uDt"), dt); gl.uniform1f(u("uKeep"), Math.exp(-AIR_FADE * dt));
      });
      swap(air);
      this.energy *= Math.exp(-AIR_FADE * dt);
      // the air has come to rest: no more solving until something stirs it
      if (this.energy <= 0.004) for (const t of [air.read, air.write, pressure.read, pressure.write]) {
        gl.bindFramebuffer(gl.FRAMEBUFFER, t.fbo); gl.viewport(0, 0, t.w, t.h); gl.clearColor(0, 0, 0, 1); gl.clear(gl.COLOR_BUFFER_BIT);
      }
    }
    this.pass("density", cloud.write, (u) => {
      this.bind(0, air.read, u("uVelocity")); this.bind(1, cloud.read, u("uDensity"));
      this.banksUniforms(u);
      gl.uniform2f(u("uGrid"), 1 / air.read.w, 1 / air.read.h);
      gl.uniform2f(u("uWind"), wind, 0);
      gl.uniform1f(u("uDt"), dt);
      gl.uniform1f(u("uHeal"), 1 - Math.exp(-HEAL * dt));
    }, cloud.read);
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
  draw(width: number, height: number) {
    const { air, cloud } = this;
    if (air === null || cloud === null) return;
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, width, height);
    gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT);
    const look = this.look;
    this.pass("display", null, (u) => {
      this.bind(0, cloud.read, u("uDensity")); this.bind(1, air.read, u("uVelocity"));
      gl.uniform1f(u("uTime"), this.time);
      gl.uniform1f(u("uAspect"), this.aspect());
      gl.uniform1f(u("uDrift"), this.drift);
      gl.uniform2f(u("uSun"), 0.5, 1.2);
      gl.uniform2f(u("uPixel"), 1 / width, 1 / height);
      gl.uniform3f(u("uLit"), ...look.lit);
      gl.uniform3f(u("uShade"), ...look.shade);
      gl.uniform3f(u("uRim"), ...look.rim);
      gl.uniform1f(u("uOpacity"), look.opacity);
    }, { w: width, h: height });
  }

  /** Whether the GPU is a CPU in disguise (SwiftShader, llvmpipe, Windows' basic renderer): no live air there. */
  software(): boolean {
    const gl = this.gl;
    const info = gl.getExtension("WEBGL_debug_renderer_info");
    const name = String(gl.getParameter(info === null ? gl.RENDERER : info.UNMASKED_RENDERER_WEBGL) ?? "");
    return /swiftshader|llvmpipe|softpipe|software|basic render/i.test(name);
  }

  /** Whether the air still moves (the loop may then run at full rate). */
  stirred(): boolean { return this.energy > 0.004; }

  dispose() {
    const gl = this.gl;
    this.releaseTargets();
    for (const { program } of this.programs.values()) gl.deleteProgram(program);
    for (const s of this.shapes) gl.deleteShader(s);
    if (this.quad !== null) gl.deleteBuffer(this.quad);
    if (this.vao !== null) gl.deleteVertexArray(this.vao);
    this.programs.clear(); this.shapes = [];
  }
}
