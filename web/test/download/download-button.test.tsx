// The big download button, given a release and a machine: never the
// browser this test runs in, whose user agent names the host's system
// (jsdom writes "linux" on a Linux runner and "darwin" on a Mac).

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { DownloadButton } from "../../src/download/DownloadButton";
import { platformFor, type Machine } from "../../src/download/machine";
import { sortFiles, type Release } from "../../src/download/release";
import type { ReleaseState } from "../../src/download/useRelease";
import { LangProvider } from "../../src/i18n";
import { release } from "../fixtures/release";

function state(machine: Machine | null, r: Release | null | undefined = release): ReleaseState {
  return { release: r, files: r ? sortFiles(r) : {}, machine, mine: platformFor(machine) };
}

function show(s: ReleaseState, lang: "en" | "de" = "en") {
  return render(
    <LangProvider initial={lang}>
      <DownloadButton state={s} />
    </LangProvider>,
  );
}

const LINUX: Machine = { os: "linux", arch: "x64", sure: true, mobile: false };

describe("the download button", () => {
  it("offers this machine's installer, straight from GitHub", () => {
    show(state(LINUX));
    const button = screen.getByRole("link", { name: "Download for Linux" });
    expect(button.getAttribute("href")).toBe("https://github.com/AceVik/baylee/releases/download/v1/Baylee-0.1.0-x86_64.AppImage");
    expect(screen.getByText(/AppImage/)).toBeTruthy();
    expect(screen.queryByText(/could not tell your system/)).toBeNull();
  });

  it("says what it could not tell, and offers the release instead of a guess", () => {
    show(state({ os: null, arch: null, sure: false, mobile: false }));
    expect(screen.getByText(/could not tell your system/)).toBeTruthy();
    expect(screen.getByRole("link", { name: "Download Baylee" }).getAttribute("href")).toBe(release.html_url);
  });

  it("says a Mac's chip is unknown rather than guessing it", () => {
    show(state({ os: "mac", arch: null, sure: false, mobile: false }), "de");
    expect(screen.getByRole("link", { name: "Download für macOS" }).getAttribute("href")).toMatch(/aarch64\.dmg$/);
    expect(screen.getByText(/verrät nicht, ob dein Mac/)).toBeTruthy();
  });

  it("points to the releases page while GitHub has not answered", () => {
    show(state(LINUX, null));
    expect(screen.getByRole("link", { name: "Download Baylee" }).getAttribute("href")).toBe("https://github.com/AceVik/baylee/releases");
  });
});
