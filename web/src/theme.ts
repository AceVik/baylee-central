// The colour scheme a visitor chose: the device's (no attribute), or light or
// dark (`data-theme` on <html>, which tokens.css reads). public/boot.js
// applies the stored choice before the first paint and `?theme=` over it for
// one visit; the privacy notice says the browser keeps this choice
// ("language and display"), and nothing else about it leaves the page.

import { useCallback, useEffect, useState } from "react";

export type Theme = "system" | "light" | "dark";

/** Where the choice is kept; boot.js reads the same key. */
export const THEME_KEY = "baylee-site-theme";

/** The order a click goes through. */
export const NEXT_THEME: Readonly<Record<Theme, Theme>> = { system: "light", light: "dark", dark: "system" };

function shown(): Theme {
  const v = document.documentElement.getAttribute("data-theme");
  return v === "light" || v === "dark" ? v : "system";
}

function apply(theme: Theme): void {
  if (theme === "system") document.documentElement.removeAttribute("data-theme");
  else document.documentElement.setAttribute("data-theme", theme);
  const meta = document.querySelector<HTMLMetaElement>('meta[name="color-scheme"]');
  if (meta) meta.content = theme === "system" ? "dark light" : theme;
}

function resolveCurrent(): "light" | "dark" {
  const attr = document.documentElement.getAttribute("data-theme");
  if (attr === "light" || attr === "dark") return attr;
  if (typeof window.matchMedia === "function" && window.matchMedia("(prefers-color-scheme: light)").matches) {
    return "light";
  }
  return "dark";
}

function remember(theme: Theme): void {
  try {
    if (theme === "system") window.localStorage.removeItem(THEME_KEY);
    else window.localStorage.setItem(THEME_KEY, theme);
  } catch {
    /* blocked storage: the choice lasts this visit */
  }
}

/** The scheme in force and a way to choose another; another tab's choice is followed. */
export function useTheme(): [Theme, (theme: Theme) => void] {
  const [theme, setTheme] = useState<Theme>(shown);
  const choose = useCallback((next: Theme) => {
    apply(next);
    remember(next);
    setTheme(next);
  }, []);
  useEffect(() => {
    const follow = (e: StorageEvent) => {
      if (e.key !== THEME_KEY) return;
      const next = e.newValue === "light" || e.newValue === "dark" ? e.newValue : "system";
      apply(next);
      setTheme(next);
    };
    window.addEventListener("storage", follow);
    return () => window.removeEventListener("storage", follow);
  }, []);
  return [theme, choose];
}

/** The resolved light or dark appearance in effect right now, following both manual choice and live OS changes. */
export function useResolvedTheme(): "light" | "dark" {
  const [resolved, setResolved] = useState<"light" | "dark">(resolveCurrent);
  useEffect(() => {
    const update = () => setResolved(resolveCurrent());
    const media = typeof window.matchMedia === "function" ? window.matchMedia("(prefers-color-scheme: light)") : null;
    media?.addEventListener("change", update);
    window.addEventListener("storage", update);
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    update();
    return () => {
      media?.removeEventListener("change", update);
      window.removeEventListener("storage", update);
      observer.disconnect();
    };
  }, []);
  return resolved;
}
