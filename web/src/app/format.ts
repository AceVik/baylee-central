// Times and dates as the page's language writes them.

import { localeOf, type Lang } from "../i18n";

/** A date and time of day, from Unix seconds. */
export function when(lang: Lang, seconds: number): string {
  return new Date(seconds * 1000).toLocaleString(localeOf(lang), { dateStyle: "medium", timeStyle: "short" });
}

/** A date, from Unix seconds. */
export function day(lang: Lang, seconds: number): string {
  return new Date(seconds * 1000).toLocaleDateString(localeOf(lang), { dateStyle: "medium" });
}

/** A time of day, from Unix seconds. */
export function clock(lang: Lang, seconds: number): string {
  return new Date(seconds * 1000).toLocaleTimeString(localeOf(lang), { hour: "2-digit", minute: "2-digit" });
}
