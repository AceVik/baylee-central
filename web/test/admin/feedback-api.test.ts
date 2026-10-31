// The feedback section on the wire (central-api `reports` and `admin`): the
// requests in Central's exact shapes (a filter with only its set parts, a
// change that tells "leave" from "clear"), the answers checked against
// their shapes and bounds, the part route's headers and byte bound; the
// GitHub links and a reporter's alias.

import { describe, expect, it } from "vitest";

import { ApiError } from "../../src/account/api";
import { idText } from "../../src/account/owner-api";
import { alias } from "../../src/admin/feedback/alias";
import { parseReports, readBounded, reportsApi, wireChange, wireFilter, type ReportFilter } from "../../src/admin/feedback/api";
import { issueUrl, newIssueUrl, parseIssue, plainBuild } from "../../src/admin/feedback/github";
import { serve } from "../support/network";

const ID = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d];
const ADMIN = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x99];
const HEX = idText(Uint8Array.from(ID));
const admin = (answer: unknown) => ({ result: "admin", answer });

const summary = (extra: Record<string, unknown> = {}) => ({
  id: ID,
  created_at: 1_800_000_000,
  updated_at: 1_800_000_100,
  gateway: "Official",
  gateway_says: "Sanctuary",
  gateway_version: "0.1.1-beta.9b",
  channel: "gateway",
  reporter: "5bdc0a17",
  kind: "bug",
  status: "new",
  excerpt: "The stack\nwent wrong",
  build: "0.1.1-beta.9b",
  crash: null,
  has_record: true,
  record_complete: false,
  record_origin: "gateway",
  record_bytes: 4096,
  issue: null,
  assignee: null,
  priority: null,
  labels: ["rules", "ui"],
  duplicate_of: null,
  signature: null,
  revision: 3,
  done_at: null,
  deleted_at: null,
  seen: false,
  notes: 0,
  ...extra,
});

const detail = (extra: Record<string, unknown> = {}) => ({
  summary: summary(),
  text: "The stack\nwent wrong",
  game_id: "0192d0c5-7b3e-7a1f",
  gateway_url: "https://gateway.invalid",
  commit: "3f9a1c7e21",
  crash_location: null,
  client: { bytes: 2_000, received: true },
  record: { bytes: 4_096, received: false },
  notes: [{ id: ID, at: 1_800_000_200, author: ADMIN, text: "Seen it on Windows too." }],
  audit: [{ at: 1_800_000_000, actor: null, action: "report.filed", detail: "by Official" }],
  ...extra,
});

describe("the answers", () => {
  it("take a page of reports, every field in its bounds", () => {
    const page = parseReports.page(admin({ answer: "reports", reports: [summary()], total: 12 }));
    expect(page.total).toBe(12);
    expect(page.reports[0]?.labels).toEqual(["rules", "ui"]);
    for (const bad of [
      summary({ status: "closed" }),
      summary({ kind: "chat" }),
      summary({ reporter: "not hex" }),
      summary({ labels: ["Bad Label"] }),
      summary({ signature: "ABC" }),
      summary({ issue: 0 }),
      summary({ excerpt: "x".repeat(301) }),
      summary({ excerpt: "a\u0007bell" }),
      summary({ extra: 1 }),
      summary({ id: ID.slice(0, 15) }),
    ]) {
      expect(() => parseReports.page(admin({ answer: "reports", reports: [bad], total: 1 }))).toThrow(ApiError);
    }
    // no more rows than were asked for
    expect(() => parseReports.page(admin({ answer: "reports", reports: [summary(), summary()], total: 2 }), 1)).toThrow(ApiError);
  });

  it("take one report with its notes and history, and the parts' states", () => {
    const r = parseReports.report(admin({ answer: "report", report: detail() }));
    expect(r.client).toEqual({ bytes: 2_000, received: true });
    expect(r.notes[0]?.text).toBe("Seen it on Windows too.");
    expect(r.audit[0]?.action).toBe("report.filed");
    for (const bad of [detail({ client: { bytes: 3 * 1024 * 1024, received: true } }), detail({ audit: [{ at: 1, actor: null, action: "drop table", detail: "" }] }), detail({ notes: [{ id: ID, at: 1, author: ADMIN, text: "" }] })]) {
      expect(() => parseReports.report(admin({ answer: "report", report: bad }))).toThrow(ApiError);
    }
  });

  it("take facets, neighbours, groups, the statistics, administrators and views", () => {
    const f = parseReports.facets(
      admin({
        answer: "report_facets",
        facets: {
          statuses: [{ value: "new", count: 2 }],
          kinds: [{ value: "crash", count: 1 }],
          priorities: [{ value: null, count: 2 }],
          assignees: [{ value: null, count: 1 }, { value: ADMIN, count: 1 }],
          labels: [{ value: "ui", count: 1 }],
          gateways: [{ value: "Official", count: 2 }],
          channels: [{ value: "gateway", count: 2 }],
          reporters: [{ value: "5bdc", count: 2 }],
          total: 2,
        },
      }),
    );
    expect(f.priorities[0]?.value).toBeNull();
    expect(f.assignees.map((a) => (a.value === null ? null : idText(a.value)))).toEqual([null, idText(Uint8Array.from(ADMIN))]);
    expect(parseReports.neighbours(admin({ answer: "neighbours", neighbours: { previous: null, next: ID, position: 1, total: 4 } })).position).toBe(1);
    expect(
      parseReports.crashGroups(admin({ answer: "crash_groups", groups: [{ signature: "a".repeat(32), count: 3, open: 2, first: 1, last: 2, latest: ID, message: "index out of range", location: "stack.rs" }] }))[0]?.open,
    ).toBe(2);
    expect(parseReports.stats(admin({ answer: "report_stats", days: [{ day: "2026-10-31", kind: "bug", count: 4 }] }))[0]?.count).toBe(4);
    expect(() => parseReports.stats(admin({ answer: "report_stats", days: [{ day: "2026-13-01", kind: "bug", count: 4 }] }))).toThrow(ApiError);
    expect(parseReports.admins(admin({ answer: "report_admins", admins: [{ id: ADMIN, display_name: "Lantern", tag: 0x3fa2 }] }))[0]?.tag).toBe(0x3fa2);
    expect(parseReports.views(admin({ answer: "report_views", views: [{ id: ID, name: "Mine", query: "status=new" }] }))[0]?.query).toBe("status=new");
    expect(() => parseReports.views(admin({ answer: "report_views", views: [{ id: ID, name: "Mine", query: "q=a b" }] }))).toThrow(ApiError);
    expect(parseReports.changed(admin({ answer: "changed", changed: 3 }))).toBe(3);
  });
});

describe("the requests", () => {
  it("send a filter's set parts only, in Central's names", () => {
    const f: ReportFilter = { statuses: ["new"], priorities: [null, "high"], unassigned: true, hasRecord: false, kinds: [] };
    expect(wireFilter(f)).toEqual({ statuses: ["new"], priorities: [null, "high"], unassigned: true, has_record: false });
    expect(wireFilter({})).toEqual({});
  });

  it("tell a part left out from a part cleared", () => {
    expect(wireChange({ assignee: null, status: "triaged" })).toEqual({ status: "triaged", assignee: null });
    expect(wireChange({ duplicateOf: Uint8Array.from(ID) })).toEqual({ duplicate_of: ID });
    expect(wireChange({ labelsAdd: ["ui"], labelsRemove: [] })).toEqual({ labels_add: ["ui"] });
    expect(wireChange({ issue: null, priority: "low" })).toEqual({ issue: null, priority: "low" });
  });

  it("are admin requests tagged by action, the revision named", async () => {
    const calls = serve((c) => {
      const r = (c.body as { request: { action: string } }).request;
      if (r.action === "set_report") return { body: admin({ answer: "report", report: detail() }) };
      if (r.action === "reports_bulk") return { body: admin({ answer: "changed", changed: 1 }) };
      return { body: { result: "done" } };
    });
    const api = reportsApi();
    await api.setReport(Uint8Array.from(ID), { status: "resolved" }, 3);
    await api.bulk([Uint8Array.from(ID)], { do: "purge" });
    await api.trash(Uint8Array.from(ID));
    await api.setSeen([Uint8Array.from(ID)], true);
    expect(calls.map((c) => c.body)).toEqual([
      { operation: "admin", request: { action: "set_report", report: ID, change: { status: "resolved" }, expected_revision: 3 } },
      { operation: "admin", request: { action: "reports_bulk", reports: [ID], bulk: { do: "purge" } } },
      { operation: "admin", request: { action: "trash_report", report: ID } },
      { operation: "admin", request: { action: "set_reports_seen", reports: [ID], seen: true } },
    ]);
  });

  it("read a part from its own route with the site's headers, as gzip, and no further than asked", async () => {
    const gzip = new Uint8Array([0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let origin: string | null = "client";
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetch = async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), init });
      const headers: Record<string, string> = { "content-type": "application/gzip" };
      if (origin !== null) headers["x-record-origin"] = origin;
      return new Response(gzip, { status: 200, headers });
    };
    const api = reportsApi({ base: "/api", fetch });
    expect((await api.part(Uint8Array.from(ID), "record", 1024)).origin).toBe("client");
    expect(calls[0]?.url).toBe(`/api/v1/reports/${HEX}/record`);
    expect(calls[0]?.init?.method).toBe("POST");
    expect(calls[0]?.init?.headers).toEqual({ "Content-Type": "application/json", "X-Baylee-Central": "1" });
    expect(calls[0]?.init?.body).toBeUndefined();
    expect(calls[0]?.init?.credentials).toBe("same-origin");
    expect(calls[0]?.init?.redirect).toBe("error");
    // the client part names no origin; past the bound nothing is kept
    await expect(api.part(Uint8Array.from(ID), "client", 1024)).rejects.toThrow(ApiError);
    origin = null;
    expect((await api.part(Uint8Array.from(ID), "client", 1024)).gzip.length).toBe(gzip.length);
    await expect(api.part(Uint8Array.from(ID), "client", 8)).rejects.toThrow(ApiError);
  });

  it("stop reading a body past its bound", async () => {
    const big = new Response(new Uint8Array(100));
    await expect(readBounded(big, 99)).rejects.toThrow(ApiError);
    expect((await readBounded(new Response(new Uint8Array(100)), 100)).length).toBe(100);
  });
});

describe("GitHub", () => {
  it("files a new issue with the administrator's words, the category and a build that reads as a version", () => {
    expect(newIssueUrl({ summary: "  ", category: "bug", build: "1.0" })).toBeNull();
    const url = new URL(newIssueUrl({ summary: "The stack resolves twice\nafter a counter", category: "crash", build: "0.1.1-beta.9b" }) ?? "");
    expect(url.origin + url.pathname).toBe("https://github.com/AceVik/baylee/issues/new");
    expect(url.searchParams.get("title")).toBe("Crash: The stack resolves twice");
    expect(url.searchParams.get("body")).toContain("- Build: 0.1.1-beta.9b");
    expect(plainBuild("1.0 see evil.example")).toBeNull();
  });

  it("links an issue by its number only", () => {
    expect(issueUrl(311)).toBe("https://github.com/AceVik/baylee/issues/311");
    expect(issueUrl(0)).toBeNull();
    expect(parseIssue("#311")).toBe(311);
    expect(parseIssue("https://github.com/AceVik/baylee/issues/42")).toBe(42);
    expect(parseIssue("https://github.com/someone/else/issues/42")).toBeNull();
  });
});

describe("a reporter's alias", () => {
  it("is the same for one pseudonym and says nothing beyond its hex", () => {
    expect(alias("5bdc0a17ff")).toEqual(alias("5bdc0a17aa"));
    expect(alias("5bdc0a17").short).toBe("5bdc");
    expect(alias("5bdc0a17").name).not.toBe(alias("9e2f0011").name);
  });
});
