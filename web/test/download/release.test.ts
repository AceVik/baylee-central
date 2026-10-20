import { describe, expect, it } from "vitest";

import { pickRelease, sortFiles } from "../../src/download/release";
import { asset, release } from "../fixtures/release";

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
