import { describe, expect, it } from "vitest";

import { dateText, mb, parseCi, parseCommit, relTime } from "../src/github";

describe("GitHub's numbers in words", () => {
  it("writes a file's size as the language writes a number: a decimal below 100, no grouping", () => {
    expect(mb(12.34 * 1048576, "en")).toBe("12.3 MB");
    expect(mb(12.34 * 1048576, "de")).toBe("12,3 MB");
    expect(mb(0.05 * 1048576, "de")).toBe("0,1 MB");
    expect(mb(1234.4 * 1048576, "en")).toBe("1234 MB");
    expect(mb(150.5 * 1048576, "de")).toBe("151 MB");
  });

  it("says how long ago in minutes, hours, then days", () => {
    const now = Date.parse("2026-10-21T12:00:00Z");
    const ago = (ms: number) => new Date(now - ms).toISOString();
    expect(relTime(ago(5 * 60_000), "en", now)).toBe("5 min ago");
    expect(relTime(ago(5 * 60_000), "de", now)).toBe("vor 5 Min.");
    expect(relTime(ago(3 * 3_600_000), "en", now)).toBe("3 h ago");
    expect(relTime(ago(3 * 3_600_000), "de", now)).toBe("vor 3 Std.");
    expect(relTime(ago(4 * 86_400_000), "en", now)).toBe("4 days ago");
    expect(relTime(ago(4 * 86_400_000), "de", now)).toBe("vor 4 Tagen");
    expect(relTime("not a date", "en", now)).toBe("");
  });

  it("writes a release's date as the language does", () => {
    expect(dateText("2026-10-20T10:00:00Z", "en")).toBe("20 October 2026");
    expect(dateText("2026-10-20T10:00:00Z", "de")).toBe("20. Oktober 2026");
  });
});

const run = (html_url: string) => ({ workflow_runs: [{ conclusion: "success", html_url, head_sha: "4e984e8", updated_at: "2026-10-20T08:00:00Z" }] });
const commit = (html_url: string) => [{ sha: "4e984e8", html_url, commit: { message: "A commit", committer: { date: "2026-10-20T07:30:00Z" } } }];

describe("GitHub's links", () => {
  it("takes a CI run's or a commit's page only on github.com", () => {
    expect(parseCi(run("https://github.com/AceVik/baylee/actions/runs/1"))?.url).toBe("https://github.com/AceVik/baylee/actions/runs/1");
    expect(parseCommit(commit("https://github.com/AceVik/baylee/commit/4e984e8"))?.url).toBe("https://github.com/AceVik/baylee/commit/4e984e8");
    for (const bad of ["javascript:alert(1)", "https://github.com.example/", "http://github.com/AceVik", "data:text/html,x"]) {
      expect(parseCi(run(bad))).toBeNull();
      expect(parseCommit(commit(bad))).toBeNull();
    }
  });
});
