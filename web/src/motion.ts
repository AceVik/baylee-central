// Motion, kept small: sections that come up as they are read, numbers that
// count up, and the hero's painting that drifts with the scroll. Transform
// and opacity only, at most once a frame, and nothing at all under
// `prefers-reduced-motion` (WCAG 2.3.3).

import { useCallback, useEffect, useRef, useState } from "react";

export function reducedMotion(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Marks every `.reveal` under `root` that starts below the fold, and shows it
 * (`data-in`) as it comes into view; what is on screen already is never
 * hidden, so nothing blinks. Content that arrives later (a page loaded
 * lazily, a list fetched) is picked up as it is added. React never writes
 * these attributes, so a re-render keeps them.
 */
export function useReveal(root: { current: HTMLElement | null }): void {
  useEffect(() => {
    const host = root.current;
    if (!host) return;
    const still = reducedMotion() || typeof IntersectionObserver !== "function";
    const watch = still
      ? null
      : new IntersectionObserver(
          (entries) => {
            for (const entry of entries) {
              if (!entry.isIntersecting) continue;
              const item = entry.target as HTMLElement;
              item.dataset["in"] = "";
              watch?.unobserve(item);
            }
          },
          { rootMargin: "0px 0px -8% 0px", threshold: 0.01 },
        );
    const scan = () => {
      const fold = window.innerHeight;
      for (const item of host.querySelectorAll<HTMLElement>(".reveal:not([data-in]):not([data-wait])")) {
        if (!watch || item.getBoundingClientRect().top < fold) {
          item.dataset["in"] = "";
        } else {
          item.dataset["wait"] = "";
          watch.observe(item);
        }
      }
    };
    scan();
    const added = new MutationObserver(scan);
    added.observe(host, { childList: true, subtree: true });
    return () => {
      added.disconnect();
      watch?.disconnect();
    };
  }, [root]);
}

/** `value`, counted up from zero over ~0.7 s the first time it is set. */
export function useCountUp(value: number | null): number | null {
  const [shown, setShown] = useState<number | null>(value);
  const first = useRef(true);
  useEffect(() => {
    if (value === null) return;
    if (!first.current || reducedMotion()) {
      setShown(value);
      return;
    }
    first.current = false;
    const start = performance.now();
    const length = 700;
    let frame = 0;
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / length);
      const eased = 1 - (1 - t) ** 3;
      setShown(Math.round(value * eased));
      if (t < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [value]);
  return shown;
}

/**
 * A ref for a layer that moves at `speed` of the scroll while the hero is on
 * screen: one transform a frame at most, nothing under reduced motion.
 */
export function useParallax(speed: number): (el: HTMLElement | null) => (() => void) | undefined {
  return useCallback(
    (el: HTMLElement | null) => {
      if (!el || reducedMotion()) return undefined;
      let frame = 0;
      const draw = () => {
        frame = 0;
        const y = window.scrollY;
        if (y > window.innerHeight * 1.2) return;
        el.style.transform = `translate3d(0, ${(y * speed).toFixed(1)}px, 0)`;
      };
      const onScroll = () => {
        if (!frame) frame = requestAnimationFrame(draw);
      };
      window.addEventListener("scroll", onScroll, { passive: true });
      draw();
      return () => {
        window.removeEventListener("scroll", onScroll);
        if (frame) cancelAnimationFrame(frame);
      };
    },
    [speed],
  );
}
