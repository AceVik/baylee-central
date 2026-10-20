// GitHub's public API, asked by the visitor's browser: the newest releases
// with their files, the latest CI run on main, the last commit, stars and
// open issues. GitHub allows 60 requests an hour per address without a token
// (there is none in the page), so every answer is kept in this tab's
// sessionStorage for ten minutes. A failed request is `null`, and the page
// then keeps its static text and a link to GitHub.

import { useEffect, useState } from "react";

import { GITHUB_API } from "./config";
import type { Lang } from "./i18n";
import { LOCALE } from "./i18n";

export const CACHE_TTL_MS = 10 * 60 * 1000;
const PREFIX = "baylee-site:";

function cached(key: string, now: number): unknown {
  try {
    const raw = window.sessionStorage.getItem(PREFIX + key);
    if (raw === null) return null;
    const kept = JSON.parse(raw) as { at?: unknown; data?: unknown };
    if (typeof kept.at !== "number" || now - kept.at > CACHE_TTL_MS) return null;
    return kept.data ?? null;
  } catch {
    return null;
  }
}

function keep(key: string, data: unknown, now: number): void {
  try {
    window.sessionStorage.setItem(PREFIX + key, JSON.stringify({ at: now, data }));
  } catch {
    /* full or blocked storage: asked again next time */
  }
}

const inFlight = new Map<string, Promise<unknown>>();

/** GitHub's answer for `path` (under the repository), from this tab's cache when fresh. */
export function github(path: string, key: string): Promise<unknown> {
  const now = Date.now();
  const hit = cached(key, now);
  if (hit !== null) return Promise.resolve(hit);
  const running = inFlight.get(key);
  if (running) return running;
  const asked = fetch(GITHUB_API + path, {
    headers: { Accept: "application/vnd.github+json" },
    credentials: "omit",
    referrerPolicy: "no-referrer",
  })
    .then((r) => (r.ok ? (r.json() as Promise<unknown>) : null))
    .then((data) => {
      if (data !== null) keep(key, data, now);
      return data;
    })
    .catch(() => null)
    .finally(() => inFlight.delete(key));
  inFlight.set(key, asked);
  return asked;
}

/** `undefined` while asking, `null` when GitHub did not answer, else the parsed answer. */
export function useGitHub<T>(path: string, key: string, parse: (raw: unknown) => T | null): T | null | undefined {
  const [value, setValue] = useState<T | null | undefined>(undefined);
  useEffect(() => {
    let live = true;
    void github(path, key).then((raw) => {
      if (live) setValue(raw === null ? null : parse(raw));
    });
    return () => {
      live = false;
    };
  }, [path, key, parse]);
  return value;
}

// ---------------------------------------------------------------- shapes

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}
function str(v: unknown): string | null {
  return typeof v === "string" ? v : null;
}

export interface CiRun {
  conclusion: string;
  url: string;
  sha: string;
  updatedAt: string;
}

export function parseCi(raw: unknown): CiRun | null {
  if (!isObject(raw) || !Array.isArray(raw["workflow_runs"])) return null;
  const run: unknown = raw["workflow_runs"][0];
  if (!isObject(run)) return null;
  const url = str(run["html_url"]);
  const sha = str(run["head_sha"]);
  const updatedAt = str(run["updated_at"]);
  if (url === null || sha === null || updatedAt === null) return null;
  return { conclusion: str(run["conclusion"]) ?? "unknown", url, sha, updatedAt };
}

export interface Commit {
  sha: string;
  url: string;
  subject: string;
  when: string;
}

export function parseCommit(raw: unknown): Commit | null {
  if (!Array.isArray(raw) || !isObject(raw[0])) return null;
  const c = raw[0];
  const inner = c["commit"];
  if (!isObject(inner)) return null;
  const sha = str(c["sha"]);
  const url = str(c["html_url"]);
  const committer = inner["committer"];
  const author = inner["author"];
  const when = (isObject(committer) ? str(committer["date"]) : null) ?? (isObject(author) ? str(author["date"]) : null);
  if (sha === null || url === null || when === null) return null;
  let subject = (str(inner["message"]) ?? "").split("\n")[0] ?? "";
  if (subject.length > 72) subject = `${subject.slice(0, 70)}…`;
  return { sha, url, subject, when };
}

export interface Repo {
  stars: number;
  openIssues: number;
}

export function parseRepo(raw: unknown): Repo | null {
  if (!isObject(raw)) return null;
  const stars = raw["stargazers_count"];
  const openIssues = raw["open_issues_count"];
  if (typeof stars !== "number" || typeof openIssues !== "number") return null;
  return { stars, openIssues };
}

export const CI_PATH = "/actions/workflows/ci.yml/runs?branch=main&event=push&status=completed&per_page=1";
export const COMMIT_PATH = "/commits?sha=main&per_page=1";
export const RELEASES_PATH = "/releases?per_page=6";

// ---------------------------------------------------------------- words

export function dateText(iso: string, lang: Lang): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleDateString(LOCALE[lang], { year: "numeric", month: "long", day: "numeric" });
}

export function relTime(iso: string, lang: Lang, now: number = Date.now()): string {
  const ms = now - new Date(iso).getTime();
  if (Number.isNaN(ms)) return "";
  const mins = Math.max(0, Math.round(ms / 60000));
  if (mins < 60) return lang === "de" ? `vor ${mins} Min.` : `${mins} min ago`;
  const h = Math.round(mins / 60);
  if (h < 48) return lang === "de" ? `vor ${h} Std.` : `${h} h ago`;
  const days = Math.round(h / 24);
  return lang === "de" ? `vor ${days} Tagen` : `${days} days ago`;
}

export function mb(bytes: number, lang: Lang): string {
  const m = bytes / 1048576;
  const s = m >= 100 ? Math.round(m).toString() : m.toFixed(1);
  return `${lang === "de" ? s.replace(".", ",") : s} MB`;
}
