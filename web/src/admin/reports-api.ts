import { answer, bytes, exact, isUuidV7, malformed, poster, safeInteger, type Endpoint } from "../account/api";
import { idText } from "../account/owner-api";
export interface ReportPreview { id: string; received: number; excerpt: string; kind: string; status: string }
export interface Reports { reports: ReportPreview[]; total: number }
const FIELDS = ["id", "created_at", "updated_at", "gateway", "gateway_says", "gateway_version", "channel", "reporter", "kind", "status", "excerpt", "build", "crash", "has_record", "record_complete", "record_origin", "record_bytes", "issue", "assignee", "priority", "labels", "duplicate_of", "signature", "revision", "done_at", "deleted_at", "seen", "notes"];
export function parseReports(raw: unknown): Reports {
  const a = exact(answer(raw, "admin", ["answer"])["answer"], ["answer", "reports", "total"]);
  if (a["answer"] !== "reports" || !Array.isArray(a["reports"]) || a["reports"].length > 50) throw malformed();
  const reports = a["reports"].map((value) => {
    const r = exact(value, FIELDS); const id = bytes(r["id"], 16);
    if (!isUuidV7(id) || typeof r["excerpt"] !== "string" || r["excerpt"].length > 2000 || typeof r["kind"] !== "string" || r["kind"].length > 32 || typeof r["status"] !== "string" || r["status"].length > 32) throw malformed();
    const received = safeInteger(r["created_at"], 0);
    if (received > 8_640_000_000_000) throw malformed();
    return { id: idText(id), received, excerpt: r["excerpt"], kind: r["kind"], status: r["status"] };
  });
  if (new Set(reports.map((r) => r.id)).size !== reports.length) throw malformed();
  return { reports, total: safeInteger(a["total"], 0) };
}
export function reportsApi(endpoint: Endpoint = {}) {
  const post = poster(endpoint);
  return (offset: number): Promise<Reports> => post({ operation: "admin", request: { action: "reports", filter: { statuses: [], kinds: [], priorities: [], q: null, from: null, to: null, has_record: null, channel: null, gateway: null, assignee: null, unassigned: false, label: null, reporter: null, signature: null, duplicate_of: null, trash: false, unseen: false }, sort: "received", descending: true, offset, limit: 50 } }, parseReports);
}
