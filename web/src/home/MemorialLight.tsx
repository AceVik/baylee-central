import { useEffect, useRef, useState } from "react";
import fragment from "./memorial-light.frag?raw";

// One small WebGL surface on the masthead. Text is ordinary, selectable HTML.
// CSS supplies the same warm atmosphere when WebGL is unavailable or lost.
export function MemorialLight() {
  const ref = useRef<HTMLCanvasElement>(null);
  const made = useRef<WebGLRenderingContext | null>(null);
  const [generation, restore] = useState(0);
  // closing the album gives the GPU's context back at once, not when the page is collected
  useEffect(() => () => made.current?.getExtension("WEBGL_lose_context")?.loseContext(), []);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    canvas.dataset["generation"] = String(generation);
    let gl: WebGLRenderingContext | null;
    try { gl = canvas.getContext("webgl", { alpha: true, antialias: false, depth: false, powerPreference: "low-power" }); }
    catch { return; }
    if (!gl) return;
    made.current = gl;
    const shaders: WebGLShader[] = [];
    const shader = (kind: number, source: string) => {
      const value = gl.createShader(kind);
      if (!value) return null;
      shaders.push(value);
      gl.shaderSource(value, source); gl.compileShader(value);
      return gl.getShaderParameter(value, gl.COMPILE_STATUS) ? value : null;
    };
    const vertex = shader(gl.VERTEX_SHADER, "attribute vec2 position; void main(){gl_Position=vec4(position,0.0,1.0);}");
    const pixel = shader(gl.FRAGMENT_SHADER, fragment);
    const program = gl.createProgram();
    const buffer = gl.createBuffer();
    const release = () => {
      shaders.forEach(value => gl.deleteShader(value));
      gl.deleteProgram(program); gl.deleteBuffer(buffer);
    };
    if (!vertex || !pixel || !program || !buffer) { release(); return; }
    gl.attachShader(program, vertex); gl.attachShader(program, pixel); gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) { release(); return; }
    const activateProgram = gl.useProgram.bind(gl);
    activateProgram(program); gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const position = gl.getAttribLocation(program, "position");
    gl.enableVertexAttribArray(position); gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
    const clock = gl.getUniformLocation(program, "time");
    const resolution = gl.getUniformLocation(program, "resolution");
    const motion = matchMedia("(prefers-reduced-motion: reduce)");
    let frame = 0, last = 0, elapsed = 0, lost = false;
    canvas.dataset["renderer"] = "webgl";
    const draw = () => {
      if (lost) return;
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.uniform2f(resolution, canvas.width, canvas.height);
      gl.uniform1f(clock, motion.matches ? 0 : elapsed);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    };
    const tick = (now: number) => {
      if (now - last >= 50) { elapsed += Math.min((now - last) / 1000, 0.1); last = now; draw(); }
      frame = requestAnimationFrame(tick);
    };
    const resume = () => {
      cancelAnimationFrame(frame); last = performance.now();
      if (!document.hidden) draw();
      if (!motion.matches && !document.hidden && !lost) frame = requestAnimationFrame(tick);
    };
    const resize = new ResizeObserver(() => {
      canvas.width = Math.max(1, Math.min(960, canvas.clientWidth));
      canvas.height = Math.max(1, Math.min(300, canvas.clientHeight));
      draw();
    });
    const contextLost = (event: Event) => {
      event.preventDefault(); lost = true; cancelAnimationFrame(frame); canvas.dataset["renderer"] = "css";
    };
    const contextRestored = () => restore(value => value + 1);
    resize.observe(canvas); motion.addEventListener("change", resume);
    document.addEventListener("visibilitychange", resume);
    canvas.addEventListener("webglcontextlost", contextLost);
    canvas.addEventListener("webglcontextrestored", contextRestored);
    resume();
    return () => {
      cancelAnimationFrame(frame); resize.disconnect(); release();
      motion.removeEventListener("change", resume); document.removeEventListener("visibilitychange", resume);
      canvas.removeEventListener("webglcontextlost", contextLost); canvas.removeEventListener("webglcontextrestored", contextRestored);
    };
  }, [generation]);
  return <canvas ref={ref} className="album-light" aria-hidden="true" />;
}
