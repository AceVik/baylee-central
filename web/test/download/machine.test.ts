import { describe, expect, it } from "vitest";

import { archNote, guessFromUa, platformFor, refine } from "../../src/download/machine";

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
