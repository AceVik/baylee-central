// The latest release, matched to the visitor's machine. What the browser
// cannot tell is said beside the button instead of guessed: an Intel Mac
// from Safari (no client hints), Windows on ARM from Firefox, any CPU the
// user agent leaves out. Installers (`.exe`, `.dmg`, `.AppImage`, `.deb`)
// come first where a release carries them, the archives otherwise.

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

export type Os = "windows" | "mac" | "linux" | "android" | "ios" | "chromeos";
export type Arch = "x64" | "arm64" | "x86";

export interface Machine {
  os: Os | null;
  arch: Arch | null;
  /** The architecture was read (client hints), not guessed from the user agent. */
  sure: boolean;
  mobile: boolean;
}

export interface Platform {
  id: string;
  os: Os;
  arch: Arch;
  name: [string, string];
  sub: [string, string];
}

/** The six platforms the project builds for, and the one it does not (yet). */
export const PLATFORMS: Platform[] = [
  { id: "windows-x64", os: "windows", arch: "x64", name: ["Windows", "Windows"], sub: ["most PCs: 64-bit Intel or AMD", "die meisten PCs: 64-Bit Intel oder AMD"] },
  { id: "windows-arm64", os: "windows", arch: "arm64", name: ["Windows on ARM", "Windows auf ARM"], sub: ["Snapdragon laptops and the like", "Snapdragon-Laptops und ähnliche"] },
  { id: "mac-arm64", os: "mac", arch: "arm64", name: ["macOS", "macOS"], sub: ["Apple silicon: M1 and later", "Apple Silicon: M1 und neuer"] },
  { id: "mac-x64", os: "mac", arch: "x64", name: ["macOS on Intel", "macOS auf Intel"], sub: ["", ""] },
  { id: "linux-x64", os: "linux", arch: "x64", name: ["Linux", "Linux"], sub: ["x86-64", "x86-64"] },
  { id: "linux-arm64", os: "linux", arch: "arm64", name: ["Linux on ARM", "Linux auf ARM"], sub: ["ARM64 / aarch64", "ARM64 / aarch64"] },
];

export type Kind = "installer" | "deb" | "archive";

/** File name → platform and kind. Two families: the installers (beta.6 on) and the archives the updater uses. */
export const FILE_RULES: [RegExp, string, Kind, [string, string]][] = [
  [/^Baylee-Setup-.*-x64\.exe$/, "windows-x64", "installer", ["Setup", "Setup"]],
  [/^Baylee-Setup-.*-arm64\.exe$/, "windows-arm64", "installer", ["Setup", "Setup"]],
  [/^Baylee-.*-aarch64\.dmg$/, "mac-arm64", "installer", ["Disk image", "Disk-Image"]],
  [/^Baylee-.*-x86_64\.dmg$/, "mac-x64", "installer", ["Disk image", "Disk-Image"]],
  [/^Baylee-.*-x86_64\.AppImage$/, "linux-x64", "installer", ["AppImage", "AppImage"]],
  [/^Baylee-.*-aarch64\.AppImage$/, "linux-arm64", "installer", ["AppImage", "AppImage"]],
  [/^baylee_.*_amd64\.deb$/, "linux-x64", "deb", ["Debian / Ubuntu package", "Debian-/Ubuntu-Paket"]],
  [/^baylee_.*_arm64\.deb$/, "linux-arm64", "deb", ["Debian / Ubuntu package", "Debian-/Ubuntu-Paket"]],
  [/^baylee-client-.*-x86_64-pc-windows-msvc\.zip$/, "windows-x64", "archive", ["Archive (zip)", "Archiv (zip)"]],
  [/^baylee-client-.*-aarch64-pc-windows-msvc\.zip$/, "windows-arm64", "archive", ["Archive (zip)", "Archiv (zip)"]],
  [/^baylee-client-.*-aarch64-apple-darwin\.zip$/, "mac-arm64", "archive", ["Archive (zip)", "Archiv (zip)"]],
  [/^baylee-client-.*-x86_64-apple-darwin\.zip$/, "mac-x64", "archive", ["Archive (zip)", "Archiv (zip)"]],
  [/^baylee-client-.*-x86_64-unknown-linux-gnu\.tar\.gz$/, "linux-x64", "archive", ["Archive (tar.gz)", "Archiv (tar.gz)"]],
  [/^baylee-client-.*-aarch64-unknown-linux-gnu\.tar\.gz$/, "linux-arm64", "archive", ["Archive (tar.gz)", "Archiv (tar.gz)"]],
];

const KIND_ORDER: Record<Kind, number> = { installer: 0, deb: 1, archive: 2 };

export interface ReleaseFile {
  asset: Asset;
  kind: Kind;
  label: [string, string];
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

// ---------------------------------------------------------------- the machine

export interface Navigatorish {
  userAgent: string;
  platform: string;
  maxTouchPoints: number;
}

export interface HighEntropy {
  platform?: string;
  architecture?: string;
  bitness?: string;
}

/** What the user agent says, honestly: `sure` stays false. */
export function guessFromUa(nav: Navigatorish): Machine {
  const ua = nav.userAgent;
  const m: Machine = { os: null, arch: null, sure: false, mobile: false };
  if (/Android/i.test(ua)) {
    m.os = "android";
    m.mobile = true;
  } else if (/iPhone|iPad|iPod/i.test(ua) || (nav.platform === "MacIntel" && nav.maxTouchPoints > 1)) {
    m.os = "ios";
    m.mobile = true;
  } else if (/Windows/i.test(ua)) {
    m.os = "windows";
    m.arch = /ARM64|aarch64/i.test(ua) ? "arm64" : "x64";
  } else if (/Mac OS X|Macintosh/i.test(ua)) {
    m.os = "mac";
  } else if (/CrOS/i.test(ua)) {
    m.os = "chromeos";
    m.mobile = true;
  } else if (/Linux|X11/i.test(ua)) {
    m.os = "linux";
    m.arch = /aarch64|arm64|armv8/i.test(ua) ? "arm64" : /x86_64|x64|amd64/i.test(ua) ? "x64" : null;
  }
  return m;
}

/** The user agent's guess, corrected by client hints where the browser offers them. */
export function refine(guess: Machine, hints: HighEntropy | null): Machine {
  if (!hints) return guess;
  const m = { ...guess };
  const p = (hints.platform ?? "").toLowerCase();
  if (p === "windows") m.os = "windows";
  else if (p === "macos") m.os = "mac";
  else if (p === "linux") m.os = "linux";
  else if (p === "android") {
    m.os = "android";
    m.mobile = true;
  } else if (p === "chrome os" || p === "chromeos") {
    m.os = "chromeos";
    m.mobile = true;
  }
  if (hints.architecture === "arm") {
    m.arch = "arm64";
    m.sure = true;
  } else if (hints.architecture === "x86") {
    m.arch = hints.bitness === "64" ? "x64" : "x86";
    m.sure = hints.bitness === "64";
  }
  return m;
}

interface UaData {
  getHighEntropyValues?: (hints: string[]) => Promise<HighEntropy>;
}

export async function detect(): Promise<Machine> {
  const guess = guessFromUa(navigator);
  const uad = (navigator as Navigator & { userAgentData?: UaData }).userAgentData;
  if (!uad?.getHighEntropyValues) return guess;
  try {
    return refine(guess, await uad.getHighEntropyValues(["architecture", "bitness", "platform"]));
  } catch {
    return guess;
  }
}

/** The platform the big button offers: the one this machine is, or the usual one for its system. */
export function platformFor(machine: Machine | null): Platform | null {
  if (!machine?.os || machine.mobile) return null;
  for (const p of PLATFORMS) {
    if (p.os !== machine.os) continue;
    if (machine.arch ? p.arch === machine.arch : p.id === "mac-arm64" || p.id === "linux-x64" || p.id === "windows-x64") {
      return p;
    }
  }
  return null;
}

export type ArchNote =
  | "unknown"
  | "mobile"
  | "mac-unsure"
  | "mac-intel"
  | "windows-unsure"
  | "linux-unsure"
  | null;

/** What could not be told about this machine, to be said plainly beside the button. */
export function archNote(machine: Machine | null): ArchNote {
  if (!machine?.os) return "unknown";
  if (machine.mobile) return "mobile";
  if (machine.os === "mac" && !machine.sure) return "mac-unsure";
  if (machine.os === "mac" && machine.arch === "x64") return "mac-intel";
  if (machine.os === "windows" && !machine.sure) return "windows-unsure";
  if (machine.os === "linux" && !machine.arch) return "linux-unsure";
  return null;
}
