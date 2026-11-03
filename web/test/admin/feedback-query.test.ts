// The list of reports as the address holds it (`admin/feedback/query.ts`):
// read as Central would take it, written back the same, a change back on
// the first page, the views, and a report's id found in what is pasted.

import { describe, expect, it } from "vitest";

import { idText } from "../../src/account/owner-api";
import { changed, cleanQuery, filterOf, readQuery, reportPath, requestOf, viewOf, writeQuery } from "../../src/admin/feedback/query";
import { readReportId } from "../../src/admin/feedback/Triage";

const ID = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d];
const ADMIN = [0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x99];
const HEX = idText(Uint8Array.from(ID));

describe("the list in the address", () => {
  it("reads what Central would take and drops the rest", () => {
    const q = readQuery("status=new,bogus&kind=crash&priority=none,high&q=%20stack%20&label=Bad%20Label&signature=xyz&assignee=none&from=2026-10-01&to=2026-13-01&record=yes&page=2&limit=100&sort=priority&dir=asc&trash=1");
    expect(q).toEqual({ statuses: ["new"], kinds: ["crash"], priorities: ["high", "none"], q: "stack", assignee: "none", from: "2026-10-01", record: true, page: 2, limit: 100, sort: "priority", asc: true, trash: true });
    expect(readQuery(writeQuery(q))).toEqual(q);
    expect(filterOf(q)).toEqual({ statuses: ["new"], kinds: ["crash"], priorities: ["high", null], q: "stack", from: "2026-10-01", hasRecord: true, unassigned: true, trash: true });
    expect(requestOf(q)).toMatchObject({ sort: "priority", descending: false, offset: 100, limit: 100 });
  });

  it("cleans a search of what Central refuses", () => {
    expect(cleanQuery("a\u202Eb\u0000c")).toBe("a b c");
    expect(cleanQuery("   ")).toBeUndefined();
    expect([...(cleanQuery("x".repeat(300)) ?? "")].length).toBe(200);
  });

  it("goes back to the first page on a change, and knows its view", () => {
    const me = idText(Uint8Array.from(ADMIN));
    const q = changed({ statuses: ["new"], page: 3 }, { kinds: ["bug"] });
    expect(q).toEqual({ statuses: ["new"], kinds: ["bug"] });
    expect(viewOf(q, me)).toBe("inbox");
    expect(viewOf({ statuses: ["triaged", "new", "in_progress"], assignee: me }, me)).toBe("mine");
    expect(viewOf({ statuses: ["new"], trash: true }, me)).toBeNull();
    expect(reportPath(HEX, { statuses: ["new"], page: 2 })).toBe(`/admin/feedback/${HEX}?status=new`);
  });

  it("finds a report's id in what is pasted", () => {
    expect(readReportId(`https://central.invalid/admin/feedback/${HEX}?status=new`)).toBe(HEX);
    expect(readReportId("0192d0c5-7b3e-7a1f-8c2d-3e4f5a6b7c8d")).toBe(HEX);
    expect(readReportId("hello")).toBeNull();
  });
});

