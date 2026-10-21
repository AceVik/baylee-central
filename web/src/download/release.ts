// The latest release on GitHub and its files, sorted per platform:
// installers (`.exe`, `.dmg`, `.AppImage`, `.deb`) first where a release
// carries them, the archives the updater uses otherwise. Only GitHub's own
// download links are taken.

import type { Key } from "../i18n";
import type { Arch, Os } from "./machine";

export interface Asset {
  name: string;
  size: number;
  browser_download_url: string;
}

export interface Release {
  tag_name: string;
  html_url: string;
  published_at: string;
  prerelease: boolean;
  draft: boolean;
  assets: Asset[];
}

export interface Platform {
  id: string;
  os: Os;
  arch: Arch;
  name: Key;
  /** What it is for, where the name does not say. */
  sub: Key | null;
}

/** The six platforms the project builds for, and the one it does not (yet). */
export const PLATFORMS: Platform[] = [
  { id: "windows-x64", os: "windows", arch: "x64", name: "plat.windows", sub: "plat.windowsSub" },
  { id: "windows-arm64", os: "windows", arch: "arm64", name: "plat.windowsArm", sub: "plat.windowsArmSub" },
  { id: "mac-arm64", os: "mac", arch: "arm64", name: "plat.mac", sub: "plat.macSub" },
  { id: "mac-x64", os: "mac", arch: "x64", name: "plat.macIntel", sub: null },
  { id: "linux-x64", os: "linux", arch: "x64", name: "plat.linux", sub: "plat.linuxSub" },
  { id: "linux-arm64", os: "linux", arch: "arm64", name: "plat.linuxArm", sub: "plat.linuxArmSub" },
];

export type Kind = "installer" | "deb" | "archive";

/** File name → platform and kind. Two families: the installers (beta.6 on) and the archives the updater uses. */
export const FILE_RULES: [RegExp, string, Kind, Key][] = [
  [/^Baylee-Setup-.*-x64\.exe$/, "windows-x64", "installer", "file.setup"],
  [/^Baylee-Setup-.*-arm64\.exe$/, "windows-arm64", "installer", "file.setup"],
  [/^Baylee-.*-aarch64\.dmg$/, "mac-arm64", "installer", "file.diskImage"],
  [/^Baylee-.*-x86_64\.dmg$/, "mac-x64", "installer", "file.diskImage"],
  [/^Baylee-.*-x86_64\.AppImage$/, "linux-x64", "installer", "file.appImage"],
  [/^Baylee-.*-aarch64\.AppImage$/, "linux-arm64", "installer", "file.appImage"],
  [/^baylee_.*_amd64\.deb$/, "linux-x64", "deb", "file.deb"],
  [/^baylee_.*_arm64\.deb$/, "linux-arm64", "deb", "file.deb"],
  [/^baylee-client-.*-x86_64-pc-windows-msvc\.zip$/, "windows-x64", "archive", "file.zip"],
  [/^baylee-client-.*-aarch64-pc-windows-msvc\.zip$/, "windows-arm64", "archive", "file.zip"],
  [/^baylee-client-.*-aarch64-apple-darwin\.zip$/, "mac-arm64", "archive", "file.zip"],
  [/^baylee-client-.*-x86_64-apple-darwin\.zip$/, "mac-x64", "archive", "file.zip"],
  [/^baylee-client-.*-x86_64-unknown-linux-gnu\.tar\.gz$/, "linux-x64", "archive", "file.tarGz"],
  [/^baylee-client-.*-aarch64-unknown-linux-gnu\.tar\.gz$/, "linux-arm64", "archive", "file.tarGz"],
];

const KIND_ORDER: Record<Kind, number> = { installer: 0, deb: 1, archive: 2 };

export interface ReleaseFile {
  asset: Asset;
  kind: Kind;
  label: Key;
  sha: Asset | null;
  sig: Asset | null;
}

export type Files = Record<string, ReleaseFile[]>;

export function sortFiles(release: Release): Files {
  const byName = new Map(release.assets.map((a) => [a.name, a]));
  const files: Files = {};
  for (const asset of release.assets) {
    const rule = FILE_RULES.find(([pattern]) => pattern.test(asset.name));
    if (!rule) continue;
    const [, platform, kind, label] = rule;
    (files[platform] ??= []).push({
      asset,
      kind,
      label,
      sha: byName.get(`${asset.name}.sha256`) ?? null,
      sig: byName.get(`${asset.name}.sig`) ?? null,
    });
  }
  for (const list of Object.values(files)) list.sort((x, y) => KIND_ORDER[x.kind] - KIND_ORDER[y.kind]);
  return files;
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function asAsset(v: unknown): Asset | null {
  if (!isObject(v)) return null;
  const { name, size, browser_download_url: url } = v;
  if (typeof name !== "string" || typeof size !== "number" || typeof url !== "string") return null;
  // every download goes straight to GitHub, nowhere else
  if (!url.startsWith("https://github.com/")) return null;
  return { name, size, browser_download_url: url };
}

/** The newest release that is no draft and has files; pre-releases count (the project is in beta). */
export function pickRelease(raw: unknown): Release | null {
  if (!Array.isArray(raw)) return null;
  for (const r of raw) {
    if (!isObject(r) || r["draft"] === true || !Array.isArray(r["assets"])) continue;
    const assets = r["assets"].map(asAsset).filter((a): a is Asset => a !== null);
    const { tag_name: tag, html_url: html, published_at: at } = r;
    if (assets.length === 0 || typeof tag !== "string" || typeof html !== "string" || typeof at !== "string") continue;
    if (!html.startsWith("https://github.com/")) continue;
    return { tag_name: tag, html_url: html, published_at: at, prerelease: r["prerelease"] === true, draft: false, assets };
  }
  return null;
}
