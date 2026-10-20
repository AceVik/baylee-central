// The visitor's machine, honestly: what the user agent says, corrected by
// client hints where the browser offers them. What cannot be told (an Intel
// Mac from Safari, Windows on ARM from Firefox, any CPU the user agent leaves
// out) is named as such, for the page to say beside the button, never
// guessed.

import { PLATFORMS, type Platform } from "./release";

export type Os = "windows" | "mac" | "linux" | "android" | "ios" | "chromeos";

export type Arch = "x64" | "arm64" | "x86";

export interface Machine {
  os: Os | null;
  arch: Arch | null;
  /** The architecture was read (client hints), not guessed from the user agent. */
  sure: boolean;
  mobile: boolean;
}

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
