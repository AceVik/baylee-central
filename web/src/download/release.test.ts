import { describe, expect, it } from "vitest";

import { archNote, guessFromUa, pickRelease, platformFor, refine, sortFiles, type Release } from "./release";

const UA = {
  safariMac: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15",
  firefoxWin: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0",
  chromeLinuxArm: "Mozilla/5.0 (X11; Linux aarch64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36",
  firefoxLinux: "Mozilla/5.0 (X11; Linux; rv:140.0) Gecko/20100101 Firefox/140.0",
  android: "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Mobile Safari/537.36",
  iphone: "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1",
};

function nav(userAgent: string, platform = "", maxTouchPoints = 0) {
  return { userAgent, platform, maxTouchPoints };
}

describe("the machine", () => {
  it("says what it cannot know instead of guessing", () => {
    // Safari on a Mac never says which chip: Apple silicon is offered, and the page says it is unsure
    const mac = guessFromUa(nav(UA.safariMac, "MacIntel"));
    expect(platformFor(mac)?.id).toBe("mac-arm64");
    expect(archNote(mac)).toBe("mac-unsure");
    // Firefox on Windows cannot tell ARM from x64
    const win = guessFromUa(nav(UA.firefoxWin, "Win32"));
    expect(platformFor(win)?.id).toBe("windows-x64");
    expect(archNote(win)).toBe("windows-unsure");
    // a Linux user agent without a CPU
    const linux = guessFromUa(nav(UA.firefoxLinux, "Linux"));
    expect(platformFor(linux)?.id).toBe("linux-x64");
    expect(archNote(linux)).toBe("linux-unsure");
    expect(guessFromUa(nav(UA.chromeLinuxArm)).arch).toBe("arm64");
  });

  it("offers phones and tablets no file", () => {
    for (const m of [guessFromUa(nav(UA.android)), guessFromUa(nav(UA.iphone)), guessFromUa(nav(UA.safariMac, "MacIntel", 5))]) {
      expect(m.mobile).toBe(true);
      expect(platformFor(m)).toBeNull();
      expect(archNote(m)).toBe("mobile");
    }
    expect(archNote(guessFromUa(nav("curl/8")))).toBe("unknown");
  });

  it("trusts client hints for the architecture", () => {
    const appleSilicon = refine(guessFromUa(nav(UA.safariMac)), { platform: "macOS", architecture: "arm", bitness: "64" });
    expect(appleSilicon.sure).toBe(true);
    expect(archNote(appleSilicon)).toBeNull();
    const intelMac = refine(guessFromUa(nav(UA.safariMac)), { platform: "macOS", architecture: "x86", bitness: "64" });
    expect(platformFor(intelMac)?.id).toBe("mac-x64");
    expect(archNote(intelMac)).toBe("mac-intel");
    const winArm = refine(guessFromUa(nav(UA.firefoxWin)), { platform: "Windows", architecture: "arm", bitness: "64" });
    expect(platformFor(winArm)?.id).toBe("windows-arm64");
    expect(archNote(winArm)).toBeNull();
  });
});

function asset(name: string, size = 50 * 1048576) {
  return { name, size, browser_download_url: `https://github.com/AceVik/baylee/releases/download/v1/${name}` };
}

const release: Release = {
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

describe("the release's files", () => {
  it("sorts installers before packages before archives, with their checksums", () => {
    const files = sortFiles(release);
    expect(files["windows-x64"]?.map((f) => f.kind)).toEqual(["installer", "archive"]);
    expect(files["windows-x64"]?.[0]?.sha?.name).toBe("Baylee-Setup-0.1.0-x64.exe.sha256");
    expect(files["windows-x64"]?.[1]?.sig?.name).toBe("baylee-client-0.1.0-x86_64-pc-windows-msvc.zip.sig");
    expect(files["linux-x64"]?.map((f) => f.kind)).toEqual(["installer", "deb"]);
    expect(files["mac-arm64"]?.map((f) => f.asset.name)).toEqual(["Baylee-0.1.0-aarch64.dmg"]);
    expect(files["mac-x64"]).toBeUndefined();
  });

  it("takes the newest release that is no draft, and only GitHub's own download links", () => {
    const draft = { ...release, draft: true, tag_name: "v9" };
    const empty = { ...release, assets: [], tag_name: "v8" };
    expect(pickRelease([draft, empty, release])?.tag_name).toBe("v0.1.0-beta.7");
    const elsewhere = { ...release, assets: [{ ...asset("Baylee-Setup-1-x64.exe"), browser_download_url: "https://evil.invalid/x.exe" }] };
    expect(pickRelease([elsewhere])).toBeNull();
    expect(pickRelease({ message: "API rate limit exceeded" })).toBeNull();
  });
});
