// Interactive atmospheric clouds, aerodynamic cloud-slicing & fine wind-vortex
// physics for the hero header:
// - Composite deformable clouds (`CloudParcel` sub-lobes bound to drifting
//   cloud anchors by soft restoration springs).
// - Cloud slicing ("Wolken, die vom Zeiger durch den Luftstrom zerschnitten
//   werden"): when the pointer flies through a cloud bank, its high-speed
//   airstream cleaves a parted channel along the flight path, shearing the
//   left and right cloud halves into counter-rotating Kármán trailing vortices
//   before the cloud gently heals back together.
// - Fine sheared mist filaments (`WakeWisp`): soft, elongated volumetric
//   vapor filaments shed along both rims of the cleaved channel and advected
//   by the rotational Lamb-Oseen vortex field without any hard geometric strokes.
// - Pauses automatically when scrolled out of view (`data-away`), when a modal
//   `<dialog open>` is active, when the tab is hidden, or under
//   `prefers-reduced-motion: reduce`.

import { useEffect, useRef } from "react";

import { reducedMotion } from "../motion";

interface CloudParcel {
  // Anchor center of the parent cloud bank
  anchorX: number;
  anchorY: number;
  // Rest offset of this sub-parcel relative to the anchor
  restOx: number;
  restOy: number;
  // Current displaced offset from the anchor (sheared/sliced by airflow)
  dx: number;
  dy: number;
  vx: number;
  vy: number;
  baseSpeed: number;
  radius: number;
  aspect: number;
  rot: number;
  vrot: number;
  baseAlpha: number;
  // 0..1 temporary thinning when the airstream slices right through the parcel core
  cleave: number;
  layer: number;
  phase: number;
  spriteIdx: number;
}

interface VortexEddy {
  x: number;
  y: number;
  vx: number;
  vy: number;
  radius: number;
  omega: number; // signed circulation strength
  life: number;
  decay: number;
}

interface WakeWisp {
  x: number;
  y: number;
  vx: number;
  vy: number;
  radius: number;
  aspect: number;
  rot: number;
  vrot: number;
  alpha: number;
  life: number;
  decay: number;
  spriteIdx: number;
}

interface SunMote {
  x: number;
  y: number;
  vx: number;
  vy: number;
  size: number;
  alpha: number;
  phase: number;
  warm: boolean;
}

/** Pre-render soft volumetric cloud-lobe sprites once. */
function buildCloudSprites(): HTMLCanvasElement[] {
  if (typeof document === "undefined") return [];
  const configs = [
    {
      lobes: [
        [0, 0, 1.0],
        [-0.36, 0.04, 0.84],
        [0.38, 0.03, 0.84],
        [-0.16, -0.1, 0.78],
        [0.18, 0.08, 0.76],
      ] as const,
      tint: [255, 251, 243] as const,
      mid: [255, 238, 194] as const,
    },
    {
      lobes: [
        [0, 0.02, 0.96],
        [-0.42, 0.02, 0.8],
        [0.4, 0.06, 0.78],
        [0.04, -0.12, 0.82],
      ] as const,
      tint: [244, 249, 255] as const,
      mid: [255, 242, 204] as const,
    },
    {
      lobes: [
        [-0.2, 0.01, 0.92],
        [0.22, 0.02, 0.9],
        [-0.46, 0.05, 0.72],
        [0.46, 0.05, 0.72],
        [0, -0.09, 0.8],
      ] as const,
      tint: [255, 248, 235] as const,
      mid: [255, 231, 174] as const,
    },
  ];

  const size = 256;
  const half = size / 2;
  const sprites: HTMLCanvasElement[] = [];

  for (const cfg of configs) {
    const off = document.createElement("canvas");
    off.width = size;
    off.height = size;
    let ctx: CanvasRenderingContext2D | null = null;
    try {
      ctx = off.getContext("2d");
    } catch {
      ctx = null;
    }
    if (!ctx) continue;

    for (const [ox, oy, scale] of cfg.lobes) {
      const cx = half + ox * half * 0.54;
      const cy = half + oy * half * 0.44;
      const r = half * 0.64 * scale;
      const g = ctx.createRadialGradient(cx, cy, r * 0.01, cx, cy, r);
      const [tr, tg, tb] = cfg.tint;
      const [mr, mg, mb] = cfg.mid;
      g.addColorStop(0, `rgba(${tr}, ${tg}, ${tb}, 0.28)`);
      g.addColorStop(0.38, `rgba(${mr}, ${mg}, ${mb}, 0.16)`);
      g.addColorStop(0.72, `rgba(${tr}, ${tg}, ${tb}, 0.06)`);
      g.addColorStop(1, `rgba(${tr}, ${tg}, ${tb}, 0)`);
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(cx, cy, r, 0, Math.PI * 2);
      ctx.fill();
    }
    sprites.push(off);
  }
  return sprites;
}

export function HeroShader() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (reducedMotion()) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const hero = canvas.closest<HTMLElement>(".hero");
    if (!hero) return;

    let ctx: CanvasRenderingContext2D | null = null;
    try {
      ctx = canvas.getContext("2d");
    } catch {
      ctx = null;
    }
    if (!ctx) return;

    const sprites = buildCloudSprites();
    if (sprites.length === 0) return;

    let width = Math.max(1, hero.clientWidth || 1280);
    let height = Math.max(1, hero.clientHeight || 640);

    const syncSize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 1.75);
      width = Math.max(1, hero.clientWidth || 1280);
      height = Math.max(1, hero.clientHeight || 640);
      canvas.width = Math.round(width * dpr);
      canvas.height = Math.round(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    syncSize();

    // Build composite deformable clouds: each cloud bank is composed of 5 sub-parcels
    // that drift together but can be independently sliced apart and curled by the cursor's airstream.
    const bankCount = width < 640 ? 9 : 14;
    const parcels: CloudParcel[] = [];
    const subOffsets: readonly (readonly [number, number, number])[] = [
      [-0.65, 0.04, 0.82],
      [-0.28, -0.06, 0.96],
      [0.0, 0.02, 1.05],
      [0.32, -0.05, 0.94],
      [0.68, 0.05, 0.8],
    ];

    for (let b = 0; b < bankCount; b++) {
      const layer = b % 3;
      const layerScale = layer === 0 ? 0.82 : layer === 1 ? 1.08 : 1.34;
      const anchorX = ((b * 137.5) % 100) * 0.01 * width;
      const anchorY = (0.08 + ((b * 61.8) % 84) * 0.01) * height;
      const baseSpeed = (0.075 + (b % 5) * 0.024) * layerScale;
      const bankRadius = (78 + (b % 5) * 16) * layerScale;

      for (let s = 0; s < subOffsets.length; s++) {
        const sub = subOffsets[s];
        if (!sub) continue;
        const [oxNorm, oyNorm, rScale] = sub;
        parcels.push({
          anchorX,
          anchorY,
          restOx: oxNorm * bankRadius * 1.35,
          restOy: oyNorm * bankRadius * 0.75,
          dx: 0,
          dy: 0,
          vx: 0,
          vy: 0,
          baseSpeed,
          radius: bankRadius * rScale,
          aspect: 1.85 + ((b + s) % 3) * 0.25,
          rot: (((b + s) % 5) - 2) * 0.02,
          vrot: 0,
          baseAlpha: layer === 0 ? 0.22 : layer === 1 ? 0.28 : 0.34,
          cleave: 0,
          layer,
          phase: b * 1.17 + s * 0.65,
          spriteIdx: (b + s) % sprites.length,
        });
      }
    }

    const moteCount = width < 640 ? 18 : 28;
    const motes: SunMote[] = Array.from({ length: moteCount }, (_, i) => ({
      x: ((i * 97.3) % 100) * 0.01 * width,
      y: ((i * 53.7) % 100) * 0.01 * height,
      vx: 0,
      vy: 0,
      size: 1.3 + (i % 4) * 0.6,
      alpha: 0.24 + (i % 5) * 0.08,
      phase: i * 0.83,
      warm: i % 4 !== 0,
    }));

    const eddies: VortexEddy[] = [];
    const wisps: WakeWisp[] = [];

    const pointer = {
      x: width * 0.5,
      y: height * 0.45,
      vx: 0,
      vy: 0,
      speed: 0,
      active: false,
      lastTime: 0,
      stepDist: 0,
      flip: 1,
    };

    /** Evaluate the rotational Lamb-Oseen vortex velocity field at (x, y). */
    const sampleTurbulence = (x: number, y: number): { ux: number; uy: number; curl: number } => {
      let ux = 0;
      let uy = 0;
      let curl = 0;
      for (const e of eddies) {
        const dx = x - e.x;
        const dy = y - e.y;
        const rSq = dx * dx + dy * dy;
        const maxRSq = e.radius * e.radius * 3.6;
        if (rSq >= maxRSq || rSq < 1) continue;
        const r = Math.sqrt(rSq);
        const normR = r / e.radius;
        const swirl = e.omega * e.life * normR * Math.exp(-normR * normR * 0.88);
        const tx = -dy / r;
        const ty = dx / r;
        // Gentle inward entrainment into the low-pressure vortex core
        const suction = -0.14 * Math.abs(swirl) * Math.exp(-normR * normR * 1.4);
        const rx = dx / r;
        const ry = dy / r;
        ux += tx * swirl * e.radius * 0.22 + rx * suction * e.radius * 0.08 + e.vx * 0.22 * Math.exp(-normR * normR);
        uy += ty * swirl * e.radius * 0.22 + ry * suction * e.radius * 0.08 + e.vy * 0.22 * Math.exp(-normR * normR);
        curl += swirl * 0.1;
      }
      return { ux, uy, curl };
    };

    const spawnWakeWisp = (
      cx: number,
      cy: number,
      vx: number,
      vy: number,
      spin: number,
      speedNorm: number,
      ribbon = false,
    ) => {
      if (wisps.length >= 42) wisps.shift();
      wisps.push({
        x: cx,
        y: cy,
        vx,
        vy,
        radius: ribbon ? 34 + speedNorm * 18 : 48 + speedNorm * 22,
        aspect: ribbon ? 2.55 + Math.min(speedNorm * 0.45, 0.9) : 1.75 + (Math.abs(spin) % 0.45),
        rot: Math.atan2(vy, vx) + spin * (ribbon ? 0.22 : 0.08),
        vrot: spin * (0.02 + speedNorm * 0.014),
        alpha: ribbon
          ? 0.14 + Math.min(speedNorm * 0.05, 0.11)
          : 0.12 + Math.min(speedNorm * 0.05, 0.11),
        life: 1,
        decay: ribbon ? 0.016 + Math.random() * 0.005 : 0.013 + Math.random() * 0.005,
        spriteIdx: Math.floor(Math.random() * sprites.length),
      });
    };

    /**
     * Slice through any cloud parcels intersecting the pointer's high-speed airstream:
     * parcels right on the flight line are parted laterally to the left/right rims and
     * rolled into counter-rotating vortices, creating a realistic "cut through the cloud".
     */
    const cleaveCloudsAlongFlight = (
      nx: number,
      ny: number,
      tx: number,
      ty: number,
      speedNorm: number,
    ) => {
      const perpX = -ty;
      const perpY = tx;
      const cleaveRadius = 62 + speedNorm * 30;
      const cleaveRadiusSq = cleaveRadius * cleaveRadius;

      for (const p of parcels) {
        const px = p.anchorX + p.restOx + p.dx;
        const py = p.anchorY + p.restOy + p.dy;
        const relX = px - nx;
        const relY = py - ny;
        const distSq = relX * relX + relY * relY;
        const influenceR = cleaveRadius + p.radius * 0.58;
        if (distSq > influenceR * influenceR) continue;

        // Signed lateral distance from the pointer's flight vector
        const sideDist = relX * perpX + relY * perpY;
        const alongDist = relX * tx + relY * ty;
        if (Math.abs(alongDist) > cleaveRadius * 1.4) continue;

        const sideSign = sideDist >= 0 ? 1 : -1;
        const latNorm = Math.abs(sideDist) / Math.max(1, cleaveRadius);
        const envelope = Math.exp(-latNorm * latNorm * 1.55);

        // 1. Thin out the center of the cut while parting the two halves to either side of the airstream
        if (distSq < cleaveRadiusSq) {
          p.cleave = Math.min(0.76, p.cleave + envelope * 0.38 * Math.min(speedNorm, 1.8));
        }

        // 2. Shear the severed halves forward in the slipstream and outward into the vortex rims
        const shearForward = (1.15 + speedNorm * 1.3) * envelope;
        const partOutward = (1.45 + speedNorm * 1.55) * envelope * (1 - latNorm * 0.38);
        p.vx += tx * shearForward + perpX * sideSign * partOutward;
        p.vy += ty * shearForward + perpY * sideSign * partOutward;
        // 3. Spin the two halves in opposite directions (counter-rotating Kármán roll-up)
        p.vrot += sideSign * (0.009 + speedNorm * 0.007) * envelope;
      }
    };

    const onPointerMove = (e: PointerEvent) => {
      const rect = hero.getBoundingClientRect();
      const nx = e.clientX - rect.left;
      const ny = e.clientY - rect.top;
      const now = performance.now();

      let step = 0;
      if (pointer.active && pointer.lastTime > 0) {
        const dt = Math.max(8, Math.min(64, now - pointer.lastTime));
        const dx = nx - pointer.x;
        const dy = ny - pointer.y;
        step = Math.hypot(dx, dy);
        const rawVx = (dx / dt) * 16.67;
        const rawVy = (dy / dt) * 16.67;
        pointer.vx = pointer.vx * 0.4 + clamp(rawVx, -46, 46) * 0.6;
        pointer.vy = pointer.vy * 0.4 + clamp(rawVy, -46, 46) * 0.6;
      } else {
        pointer.vx = 0;
        pointer.vy = 0;
      }
      pointer.x = nx;
      pointer.y = ny;
      pointer.speed = Math.hypot(pointer.vx, pointer.vy);
      pointer.active = true;
      pointer.lastTime = now;
      pointer.stepDist += step;

      if (pointer.speed > 0.85) {
        const speedNorm = Math.min(pointer.speed / 16, 2.5);
        const invSpd = 1 / (pointer.speed || 1);
        const tx = pointer.vx * invSpd;
        const ty = pointer.vy * invSpd;

        // Continuously slice through clouds along the cursor's flight path
        cleaveCloudsAlongFlight(nx, ny, tx, ty, speedNorm);

        if (pointer.stepDist >= 18) {
          pointer.stepDist = 0;
          const perpX = -ty;
          const perpY = tx;
          const offset = 20 + speedNorm * 14;
          const radius = 64 + speedNorm * 38;
          const omega = 0.11 + speedNorm * 0.15;
          pointer.flip *= -1;

          // Shed counter-rotating trailing vortices behind the cursor
          if (eddies.length >= 38) eddies.splice(0, 2);
          eddies.push(
            {
              x: nx - tx * 12 + perpX * offset,
              y: ny - ty * 12 + perpY * offset,
              vx: pointer.vx * 0.15 + perpX * 0.3,
              vy: pointer.vy * 0.15 + perpY * 0.3,
              radius,
              omega,
              life: 1,
              decay: 0.011,
            },
            {
              x: nx - tx * 12 - perpX * offset,
              y: ny - ty * 12 - perpY * offset,
              vx: pointer.vx * 0.15 - perpX * 0.3,
              vy: pointer.vy * 0.15 - perpY * 0.3,
              radius,
              omega: -omega,
              life: 1,
              decay: 0.011,
            },
          );

          // Spawn sheared mist filaments along both cleaved rims that curl into the vortex wake
          spawnWakeWisp(
            nx - tx * 8 + perpX * (offset * 0.72),
            ny - ty * 8 + perpY * (offset * 0.72),
            pointer.vx * 0.18 + perpX * 0.38,
            pointer.vy * 0.18 + perpY * 0.38,
            1,
            speedNorm,
            true,
          );
          spawnWakeWisp(
            nx - tx * 8 - perpX * (offset * 0.72),
            ny - ty * 8 - perpY * (offset * 0.72),
            pointer.vx * 0.18 - perpX * 0.38,
            pointer.vy * 0.18 - perpY * 0.38,
            -1,
            speedNorm,
            true,
          );
        }
      }
    };

    const onPointerLeave = () => {
      pointer.active = false;
      pointer.vx *= 0.4;
      pointer.vy *= 0.4;
      pointer.speed = 0;
    };

    const onPointerDown = (e: PointerEvent) => {
      const rect = hero.getBoundingClientRect();
      const cx = e.clientX - rect.left;
      const cy = e.clientY - rect.top;
      if (cx < 0 || cy < 0 || cx > rect.width || cy > rect.height) return;

      // Symmetrical vortex ring burst that parts nearby clouds into rolling eddies
      const pairs = 6;
      for (let i = 0; i < pairs; i++) {
        const angle = (Math.PI * 2 * i) / pairs;
        const dirX = Math.cos(angle);
        const dirY = Math.sin(angle);
        const spin = i % 2 === 0 ? 1 : -1;
        if (eddies.length >= 38) eddies.shift();
        eddies.push({
          x: cx + dirX * 22,
          y: cy + dirY * 22,
          vx: dirX * 1.15,
          vy: dirY * 1.15,
          radius: 78,
          omega: spin * 0.22,
          life: 1,
          decay: 0.011,
        });
        spawnWakeWisp(cx + dirX * 16, cy + dirY * 16, dirX * 0.9, dirY * 0.9, spin, 1.25, true);
      }
    };

    const onResize = () => syncSize();

    hero.addEventListener("pointermove", onPointerMove, { passive: true });
    hero.addEventListener("pointerleave", onPointerLeave, { passive: true });
    hero.addEventListener("pointerdown", onPointerDown, { passive: true });
    window.addEventListener("resize", onResize, { passive: true });

    let rafId = 0;
    let tick = 0;

    const renderFrame = () => {
      rafId = window.requestAnimationFrame(renderFrame);
      if (
        document.hidden ||
        hero.dataset["away"] !== undefined ||
        document.querySelector("dialog[open]") !== null
      ) {
        return;
      }

      tick += 0.016;
      ctx.clearRect(0, 0, width, height);

      // Update active vortex eddies
      for (let i = eddies.length - 1; i >= 0; i--) {
        const e = eddies[i];
        if (!e) continue;
        e.x += e.vx;
        e.y += e.vy;
        e.vx *= 0.965;
        e.vy *= 0.965;
        e.radius += 0.38;
        e.life -= e.decay;
        if (e.life <= 0) {
          eddies.splice(i, 1);
        }
      }

      // 1. Update & draw composite deformable cloud parcels (slicing + vortex curling + spring healing)
      for (const p of parcels) {
        p.anchorX += p.baseSpeed;
        const pad = p.radius * p.aspect * 1.8;
        if (p.anchorX - pad > width) p.anchorX = -pad;
        else if (p.anchorX + pad < 0) p.anchorX = width + pad;

        const curX = p.anchorX + p.restOx + p.dx;
        const curY = p.anchorY + p.restOy + p.dy;
        const turb = sampleTurbulence(curX, curY);
        const layerFactor = 0.72 + p.layer * 0.22;

        // Vortex turbulence + gentle harmonic spring restoring sheared sub-parcels back to the cloud spine
        const springK = 0.018;
        p.vx = (p.vx + turb.ux * 0.16 * layerFactor - p.dx * springK) * 0.91;
        p.vy = (p.vy + turb.uy * 0.16 * layerFactor - p.dy * springK) * 0.91;
        p.dx += p.vx;
        p.dy += p.vy;

        p.vrot = (p.vrot + clamp(turb.curl * 0.06, -0.012, 0.012)) * 0.92;
        p.rot = (p.rot + p.vrot) * 0.985;
        p.cleave *= 0.968; // parted cloud channel slowly heals back together

        const sprite = sprites[p.spriteIdx];
        if (!sprite) continue;

        const breathY = Math.sin(tick * 0.45 + p.phase) * 2.2;
        const speedShear = Math.min(Math.hypot(p.vx, p.vy) * 0.06, 0.38);
        const effectiveAlpha = Math.max(0.04, p.baseAlpha * (1 - p.cleave) + speedShear * 0.12);

        ctx.save();
        ctx.translate(p.anchorX + p.restOx + p.dx, p.anchorY + p.restOy + p.dy + breathY);
        ctx.rotate(p.rot);
        ctx.scale(p.aspect + speedShear * 0.45, Math.max(0.68, 1 - speedShear * 0.22));
        ctx.globalAlpha = Math.min(0.58, effectiveAlpha);
        ctx.drawImage(sprite, -p.radius, -p.radius, p.radius * 2, p.radius * 2);
        ctx.restore();
      }

      // 2. Draw soft volumetric wake wisps curling inside the vortex cores
      for (let i = wisps.length - 1; i >= 0; i--) {
        const w = wisps[i];
        if (!w) continue;
        w.life -= w.decay;
        if (w.life <= 0) {
          wisps.splice(i, 1);
          continue;
        }
        const turb = sampleTurbulence(w.x, w.y);
        w.vx = w.vx * 0.94 + turb.ux * 0.28;
        w.vy = w.vy * 0.94 + turb.uy * 0.28;
        w.x += w.vx;
        w.y += w.vy;
        w.rot += w.vrot + clamp(turb.curl * 0.08, -0.025, 0.025);
        w.radius += 0.4;

        const sprite = sprites[w.spriteIdx];
        if (!sprite) continue;

        const envelope = Math.sin(w.life * Math.PI);
        ctx.save();
        ctx.translate(w.x, w.y);
        ctx.rotate(w.rot);
        ctx.scale(w.aspect, 1);
        ctx.globalAlpha = w.alpha * envelope;
        ctx.drawImage(sprite, -w.radius, -w.radius, w.radius * 2, w.radius * 2);
        ctx.restore();
      }

      // 3. Draw ambient golden pollen & sanctuary motes swirling gently in the eddies
      for (const m of motes) {
        const turb = sampleTurbulence(m.x, m.y);
        m.vx = m.vx * 0.92 + turb.ux * 0.24;
        m.vy = m.vy * 0.92 + turb.uy * 0.24;
        m.x += Math.cos(tick * 0.45 + m.phase) * 0.18 + m.vx;
        m.y += -0.18 + Math.sin(tick * 0.65 + m.phase) * 0.14 + m.vy;

        if (m.x < 0) m.x = width;
        else if (m.x > width) m.x = 0;
        if (m.y < 0) m.y = height;
        else if (m.y > height) m.y = 0;

        const twinkle = 0.65 + 0.35 * Math.sin(tick * 1.8 + m.phase);
        const a = m.alpha * twinkle;
        ctx.fillStyle = m.warm
          ? `rgba(255, 232, 162, ${a.toFixed(3)})`
          : `rgba(186, 224, 255, ${a.toFixed(3)})`;
        ctx.beginPath();
        ctx.arc(m.x, m.y, m.size, 0, Math.PI * 2);
        ctx.fill();
      }
    };

    rafId = window.requestAnimationFrame(renderFrame);

    return () => {
      window.cancelAnimationFrame(rafId);
      hero.removeEventListener("pointermove", onPointerMove);
      hero.removeEventListener("pointerleave", onPointerLeave);
      hero.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("resize", onResize);
    };
  }, []);

  return (
    <div className="sky-shader" aria-hidden="true">
      <div className="shader-beams" />
      <div className="shader-motes" />
      <canvas ref={canvasRef} className="shader-canvas" />
    </div>
  );
}

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}
