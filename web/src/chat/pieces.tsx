// Small parts of the messenger: avatars, a row's menu and the times it shows.
import { useEffect, useRef, type ReactNode } from "react";

import { Icon } from "../components/icons";
import { localeOf, t, type Lang } from "../i18n";
import { hue, initials } from "./conversations";

export function Avatar({ name, seed, group = false, size = "md" }: { name: string; seed: string; group?: boolean; size?: "sm" | "md" | "lg" }) {
  return <span className="chat-avatar" data-hue={hue(seed)} data-size={size} aria-hidden="true">
    {group ? <Icon name="people" className="chat-avatar-icon" /> : initials(name)}
  </span>;
}

/** A small menu of a row's rarer actions; it closes on a choice or a press outside. */
export function Menu({ label, children }: { label: string; children: ReactNode }) {
  const ref = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    const menu = ref.current;
    if (menu === null) return;
    const away = (e: PointerEvent) => { if (menu.open && !menu.contains(e.target as Node)) menu.open = false; };
    const escape = (e: KeyboardEvent) => { if (e.key === "Escape" && menu.open) { menu.open = false; menu.querySelector("summary")?.focus(); } };
    const chosen = (e: MouseEvent) => { if ((e.target as Element).closest(".chat-menu-pop button") !== null) menu.open = false; };
    document.addEventListener("pointerdown", away);
    menu.addEventListener("keydown", escape);
    menu.addEventListener("click", chosen);
    return () => { document.removeEventListener("pointerdown", away); menu.removeEventListener("keydown", escape); menu.removeEventListener("click", chosen); };
  }, []);
  return <details ref={ref} className="chat-menu">
    <summary className="chat-icon-button" aria-label={label} title={label}><Icon name="more" /></summary>
    <div className="chat-menu-pop">{children}</div>
  </details>;
}

const DAY = 86_400_000;
const midnight = (at: number) => { const d = new Date(at); d.setHours(0, 0, 0, 0); return d.getTime(); };

/** A list row's time: the hour today, the weekday this week, else the date. */
export function shortTime(at: number, lang: Lang, now = Date.now()): string {
  const days = Math.round((midnight(now) - midnight(at)) / DAY);
  if (days <= 0) return new Intl.DateTimeFormat(localeOf(lang), { timeStyle: "short" }).format(at);
  if (days < 7) return new Intl.DateTimeFormat(localeOf(lang), { weekday: "short" }).format(at);
  return new Intl.DateTimeFormat(localeOf(lang), { dateStyle: "short" }).format(at);
}

/** A divider between days: today, yesterday, or the date. */
export function dayLabel(at: number, lang: Lang, now = Date.now()): string {
  const days = Math.round((midnight(now) - midnight(at)) / DAY);
  if (days <= 0) return t(lang, "chat.today");
  if (days === 1) return t(lang, "chat.yesterday");
  return new Intl.DateTimeFormat(localeOf(lang), { dateStyle: "long" }).format(at);
}

export const clock = (at: number, lang: Lang): string => new Intl.DateTimeFormat(localeOf(lang), { timeStyle: "short" }).format(at);
export const sameDay = (a: number, b: number): boolean => midnight(a) === midnight(b);
