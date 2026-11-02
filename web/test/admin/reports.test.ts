import { describe, expect, it } from "vitest";
import { ApiError } from "../../src/account/api";
import { parseReports } from "../../src/admin/reports-api";
const row = { id: [1,146,208,197,123,62,122,31,140,45,62,79,90,107,124,141], created_at: 1800000000, updated_at: 1800000000, gateway: "test", gateway_says: null, gateway_version: null, channel: "gateway", reporter: "a1", kind: "feedback", status: "new", excerpt: "<script>untrusted text</script>", build: null, crash: null, has_record: false, record_complete: false, record_origin: null, record_bytes: null, issue: null, assignee: null, priority: null, labels: [], duplicate_of: null, signature: null, revision: 0, done_at: null, deleted_at: null, seen: false, notes: 0 };
const response = (reports: unknown[], total: unknown = 1) => ({ result: "admin", answer: { answer: "reports", reports, total } });
describe("notification feedback destination", () => {
  it("reads only the bounded preview fields as plain text", () => {
    expect(parseReports(response([row])).reports[0]?.excerpt).toBe(row.excerpt);
    expect(Object.keys(parseReports(response([row])).reports[0] ?? {})).toEqual(["id", "received", "excerpt", "kind", "status"]);
  });
  it("refuses unbounded lists, duplicates, invalid ids, invalid dates and wrong response types", () => {
    for (const raw of [response(Array.from({ length: 51 }, () => row)), response([row, row]), response([{ ...row, id: [1] }]), response([{ ...row, created_at: 8640000000001 }]), response([{ ...row, excerpt: "x".repeat(2001) }]), response([row], -1), { result: "done" }]) expect(() => parseReports(raw)).toThrow(ApiError);
  });
});
