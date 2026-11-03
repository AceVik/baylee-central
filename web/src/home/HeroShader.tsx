// The hero's sky: clouds that drift slowly on the wind and that the pointer
// stirs. The air is a real (if small) fluid, Stam's stable fluids on the GPU
// (clouds/fluid.ts): the pointer's motion pushes it along its path, the push
// rolls up into eddies that vorticity confinement keeps, and the clouds are
// carried by that air until it calms and they heal into their banks again. A
// press sends a gust outwards. Paused while the hero is away (`data-away`),
// the tab hidden or a dialog open; one still picture under reduced motion
// and on a software renderer, where the air would cost a CPU core (tests
// may ask for it anyway: localStorage `baylee-clouds` = `live`); the
// painting alone where WebGL 2 is missing or its context was lost.

import { useEffect, useRef, useState } from "react";

import { reducedMotion } from "../motion";
import { BLUE_HOUR, Clouds, SUNLIGHT } from "./clouds/fluid";

/** The canvas's pixels per CSS pixel, and the widest it gets: the clouds are soft. */
const SCALE = 0.6;
const MAX_WIDTH = 1280;
/** A push's reach, as a share of the hero's height, and its strength. */
const REACH = 0.05;
const FORCE = 0.75;

const dark = () => getComputedStyle(document.documentElement).colorScheme.includes("dark");
const forced = () => { try { return localStorage.getItem("baylee-clouds") === "live"; } catch { return false; } };

export function HeroShader() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [generation, restore] = useState(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    const hero = canvas?.closest<HTMLElement>(".hero");
    if (!canvas || !hero) return;
    canvas.dataset["generation"] = String(generation);
    const clouds = Clouds.create(canvas);
    if (clouds === null) { canvas.dataset["renderer"] = "none"; return; }
    const still = reducedMotion() || (clouds.software() && !forced());
    canvas.dataset["renderer"] = still ? "still" : "live";

    let lost = false;
    let frame = 0;
    let last = performance.now();
    const look = () => clouds.setLook(dark() ? BLUE_HOUR : SUNLIGHT);
    look();

    const size = () => {
      const width = Math.max(1, Math.round(Math.min(canvas.clientWidth * Math.min(devicePixelRatio || 1, 2) * SCALE, MAX_WIDTH)));
      const height = Math.max(1, Math.round(width * canvas.clientHeight / Math.max(1, canvas.clientWidth)));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width; canvas.height = height;
        if (!clouds.resize(width, height)) { lost = true; canvas.dataset["renderer"] = "none"; }
      }
    };
    const paint = () => { if (!lost) clouds.draw(canvas.width, canvas.height); };
    const picture = () => { size(); if (!lost) { clouds.still(); paint(); } };

    // the pointer's path since the last frame, in the hero's 0..1 (from the bottom left)
    let trail: { x: number; y: number; t: number }[] = [];
    let prev: { x: number; y: number; t: number } | null = null;
    const at = (e: PointerEvent) => {
      const r = canvas.getBoundingClientRect();
      return { x: (e.clientX - r.left) / Math.max(1, r.width), y: 1 - (e.clientY - r.top) / Math.max(1, r.height), t: e.timeStamp };
    };
    const move = (e: PointerEvent) => {
      const p = at(e);
      if (p.x < -0.05 || p.x > 1.05 || p.y < -0.05 || p.y > 1.05) { prev = null; return; }
      if (trail.length < 64) trail.push(p);
    };
    const leave = () => { prev = null; trail = []; };
    const press = (e: PointerEvent) => {
      if (e.button !== 0) return;
      const p = at(e);
      const aspect = canvas.clientWidth / Math.max(1, canvas.clientHeight);
      // a ring of pushes outwards, alternately turned, so the gust rolls up at its rim
      for (let i = 0; i < 12; i++) {
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
          for (let s = 1; s <= steps; s++) {
            const k = s / steps;
            clouds.push(prev.x + (p.x - prev.x) * k, prev.y + (p.y - prev.y) * k, (vx * FORCE) / steps, (vy * FORCE) / steps, REACH);
          }
        }
        prev = p;
      }
      trail = [];
    };

    const paused = () => document.hidden || hero.dataset["away"] !== undefined || document.querySelector("dialog[open]") !== null;
    const tick = (now: number) => {
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

    const scheme = matchMedia("(prefers-color-scheme: dark)");
    const themed = () => { look(); if (still) picture(); };
    const watchTheme = new MutationObserver(themed);
    watchTheme.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    scheme.addEventListener("change", themed);
    const resized = new ResizeObserver(() => { if (still) picture(); else size(); });
    resized.observe(canvas);
    const contextLost = (e: Event) => { e.preventDefault(); lost = true; cancelAnimationFrame(frame); canvas.dataset["renderer"] = "none"; };
    const contextRestored = () => restore((n) => n + 1);
    canvas.addEventListener("webglcontextlost", contextLost);
    canvas.addEventListener("webglcontextrestored", contextRestored);

    if (still) picture();
    else {
      hero.addEventListener("pointermove", move, { passive: true });
      hero.addEventListener("pointerleave", leave, { passive: true });
      hero.addEventListener("pointerdown", press, { passive: true });
      size();
      frame = requestAnimationFrame(tick);
    }
    return () => {
      cancelAnimationFrame(frame);
      hero.removeEventListener("pointermove", move);
      hero.removeEventListener("pointerleave", leave);
      hero.removeEventListener("pointerdown", press);
      scheme.removeEventListener("change", themed);
      watchTheme.disconnect();
      resized.disconnect();
      canvas.removeEventListener("webglcontextlost", contextLost);
      canvas.removeEventListener("webglcontextrestored", contextRestored);
      if (!lost) clouds.dispose();
    };
  }, [generation]);

  return (
    <div className="sky-shader" aria-hidden="true">
      <div className="shader-beams" />
      <div className="shader-motes" />
      <canvas ref={canvasRef} className="shader-canvas" />
    </div>
  );
}
