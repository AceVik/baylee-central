// A synthetic release on GitHub's shape: installers, a package, archives
// with checksums and signatures, and one file no rule claims.

import type { Release } from "../../src/download/release";

export function asset(name: string, size = 50 * 1048576) {
  return { name, size, browser_download_url: `https://github.com/AceVik/baylee/releases/download/v1/${name}` };
}

export const release: Release = {
  tag_name: "v0.1.0-beta.7",
  html_url: "https://github.com/AceVik/baylee/releases/tag/v0.1.0-beta.7",
  published_at: "2026-10-12T10:00:00Z",
  prerelease: true,
  draft: false,
  assets: [
    asset("baylee-client-0.1.0-x86_64-pc-windows-msvc.zip"),
    asset("baylee-client-0.1.0-x86_64-pc-windows-msvc.zip.sha256", 100),
    asset("baylee-client-0.1.0-x86_64-pc-windows-msvc.zip.sig", 100),
    asset("Baylee-Setup-0.1.0-x64.exe"),
    asset("Baylee-Setup-0.1.0-x64.exe.sha256", 100),
    asset("baylee_0.1.0_amd64.deb"),
    asset("Baylee-0.1.0-x86_64.AppImage"),
    asset("Baylee-0.1.0-aarch64.dmg"),
    asset("unrelated.txt"),
  ],
};
