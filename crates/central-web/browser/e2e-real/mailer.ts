// A stand-in for baylee-mailer (docs/email.md, docs/operations.md §"Mail")
// for local runs against a real Central only: it takes `POST /v1/send` as
// the mailer's API.md describes it, with Central's bearer token, keeps the
// mails in memory, and hands them to the test that asks for one address's
// mails (`GET /mails?to=…`). Nothing is delivered anywhere. Loopback only;
// nothing here is for production.
//
//   node e2e-real/mailer.ts --token-file <file> [--port 47925] [--keep <dir>]
//
// The token file holds the synthetic token Central's `mail` block names; it
// is read once and never printed. With --keep every mail is also written
// to <dir> as a .txt and an .html file, so that a person can look at what
// Central sends (the owner's local preview); the files hold the codes of
// synthetic accounts, so delete the folder with the run.

import { randomUUID } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createServer, type IncomingMessage } from "node:http";
import { join } from "node:path";

function arg(name: string, fallback: string): string {
  const at = process.argv.indexOf(`--${name}`);
  return at > 0 && process.argv[at + 1] !== undefined
    ? (process.argv[at + 1] as string)
    : fallback;
}

const port = Number(arg("port", "47925"));
const tokenFile = arg("token-file", "");
const keep = arg("keep", "");
if (tokenFile === "") throw new Error("--token-file is required");
const expected = `Bearer ${readFileSync(tokenFile, "utf8").trim()}`;
if (keep !== "") mkdirSync(keep, { recursive: true, mode: 0o700 });

/** One mail as Central handed it over. */
export type Sent = {
  to: string;
  subject: string;
  text: string;
  html: string;
  ttl_secs?: number;
  limit_key?: string;
  inline?: string[];
  at: number;
};

const FIELDS = new Set([
  "to",
  "subject",
  "text",
  "html",
  "idempotency_key",
  "ttl_secs",
  "limit_key",
  "inline",
]);
/** The images the real mailer is configured with on the live host. */
const IMAGES = new Set(["baylee-logo"]);

/**
 * The real mailer's rule for `inline`: only with `html`, 1 to 4 configured
 * names, each once, each referenced as `cid:<name>` and not as the start of
 * a longer name.
 */
function inlineOk(inline: unknown, html: unknown): boolean {
  if (inline === undefined) return true;
  if (typeof html !== "string" || !Array.isArray(inline)) return false;
  if (inline.length < 1 || inline.length > 4) return false;
  if (new Set(inline).size !== inline.length) return false;
  return inline.every(
    (name) =>
      typeof name === "string" &&
      IMAGES.has(name) &&
      new RegExp(`cid:${name.replace(/[.-]/g, "\\$&")}(?![A-Za-z0-9._-])`).test(
        html,
      ),
  );
}
const LIMIT = 600_000;
const mails: Sent[] = [];
let count = 0;

function body(req: IncomingMessage): Promise<string | null> {
  return new Promise((done) => {
    const chunks: Buffer[] = [];
    let size = 0;
    req.on("data", (chunk: Buffer) => {
      size += chunk.length;
      if (size > LIMIT) {
        req.destroy();
        done(null);
      } else chunks.push(chunk);
    });
    req.on("end", () => done(Buffer.concat(chunks).toString("utf8")));
    req.on("error", () => done(null));
  });
}

function answer(
  res: import("node:http").ServerResponse,
  status: number,
  value: unknown,
) {
  res.writeHead(status, {
    "content-type": "application/json",
    "cache-control": "no-store",
  });
  res.end(JSON.stringify(value));
}

createServer(async (req, res) => {
  const url = new URL(req.url ?? "/", "http://127.0.0.1");
  if (req.method === "GET" && url.pathname === "/mails") {
    const to = url.searchParams.get("to") ?? "";
    answer(
      res,
      200,
      mails.filter((mail) => mail.to === to),
    );
    return;
  }
  if (req.method !== "POST" || url.pathname !== "/v1/send") {
    answer(res, 404, { error: "not_found" });
    return;
  }
  if (req.headers.authorization !== expected) {
    answer(res, 401, { error: "unauthorized" });
    return;
  }
  if (!(req.headers["content-type"] ?? "").startsWith("application/json")) {
    answer(res, 415, { error: "unsupported_media_type" });
    return;
  }
  const text = await body(req);
  let sent: Record<string, unknown>;
  try {
    sent = JSON.parse(text ?? "") as Record<string, unknown>;
  } catch {
    answer(res, 400, { error: "invalid_json" });
    return;
  }
  if (
    Object.keys(sent).some((key) => !FIELDS.has(key)) ||
    [sent["to"], sent["subject"], sent["text"]].some(
      (value) => typeof value !== "string",
    )
  ) {
    answer(res, 400, { error: "invalid_json" });
    return;
  }
  if (!inlineOk(sent["inline"], sent["html"])) {
    answer(res, 400, { error: "invalid_inline", field: "inline" });
    return;
  }
  const mail: Sent = {
    to: sent["to"] as string,
    subject: sent["subject"] as string,
    text: sent["text"] as string,
    html: typeof sent["html"] === "string" ? sent["html"] : "",
    at: Date.now(),
    ...(typeof sent["ttl_secs"] === "number"
      ? { ttl_secs: sent["ttl_secs"] }
      : {}),
    ...(typeof sent["limit_key"] === "string"
      ? { limit_key: sent["limit_key"] }
      : {}),
    ...(Array.isArray(sent["inline"])
      ? { inline: sent["inline"] as string[] }
      : {}),
  };
  mails.push(mail);
  if (mails.length > 200) mails.shift();
  count += 1;
  if (keep !== "") {
    const name = `${String(count).padStart(3, "0")}-${mail.to.replace(/[^a-z0-9.@-]/gi, "_")}`;
    writeFileSync(
      join(keep, `${name}.txt`),
      `To: ${mail.to}\nSubject: ${mail.subject}\n\n${mail.text}`,
      { mode: 0o600 },
    );
    writeFileSync(join(keep, `${name}.html`), mail.html, { mode: 0o600 });
  }
  answer(res, 202, { id: randomUUID(), status: "queued" });
}).listen(port, "127.0.0.1", () => {
  process.stdout.write(`mailer stand-in listening on 127.0.0.1:${port}\n`);
});
