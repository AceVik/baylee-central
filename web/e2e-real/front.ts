// A stand-in for baylee.app's Caddy site block (README.md §"Serving it",
// ../docs/operations.md §"The browser listener"), for a local run against a
// real Central only (e2e-real/README.md): TLS for baylee.app with a
// throwaway certificate, dist/ with the production CSP, and /api/* passed to
// Central's browser listener with the Host unchanged and X-Forwarded-For set
// to the client, as Caddy does without trusted_proxies (it replaces whatever
// the client sent). Loopback only; nothing here is for production.
//
//   node e2e-real/front.ts --tls <dir with cert.pem, key.pem> [--port 8443] [--listener 47921] [--dir dist]

import { createReadStream, existsSync, readFileSync, statSync } from "node:fs";
import { request } from "node:http";
import { createServer } from "node:https";
import { extname, join, resolve, sep } from "node:path";

import { csp } from "../scripts/csp.ts";

function arg(name: string, fallback: string): string {
  const at = process.argv.indexOf(`--${name}`);
  return at > 0 && process.argv[at + 1] !== undefined ? (process.argv[at + 1] as string) : fallback;
}

const port = Number(arg("port", "8443"));
const listener = Number(arg("listener", "47921"));
const root = resolve(arg("dir", "dist"));
const tls = arg("tls", "");
const policy = csp();
const LIMIT = 65_536;

const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".woff2": "font/woff2",
  ".png": "image/png",
  ".webp": "image/webp",
  ".avif": "image/avif",
  ".ico": "image/x-icon",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".txt": "text/plain; charset=utf-8",
};

const server = createServer({ key: readFileSync(join(tls, "key.pem")), cert: readFileSync(join(tls, "cert.pem")) }, (req, res) => {
  const url = new URL(req.url ?? "/", "https://baylee.app");
  if (url.pathname.startsWith("/api/")) {
    // request_body max_size 64KiB, then reverse_proxy 127.0.0.1:<listener>
    const chunks: Buffer[] = [];
    let size = 0;
    req.on("data", (chunk: Buffer) => {
      size += chunk.length;
      if (size <= LIMIT) chunks.push(chunk);
    });
    req.on("end", () => {
      if (size > LIMIT) {
        res.writeHead(413).end();
        return;
      }
      const body = Buffer.concat(chunks);
      const headers = {
        ...req.headers,
        "content-length": String(body.length),
        "x-forwarded-for": req.socket.remoteAddress ?? "",
        "x-forwarded-proto": "https",
        "x-forwarded-host": req.headers.host ?? "",
      };
      const upstream = request({ host: "127.0.0.1", port: listener, method: req.method, path: req.url, headers }, (answer) => {
        // the operation and the status only: never a body, a cookie or a key
        let operation = "";
        try {
          operation = String((JSON.parse(body.toString("utf8")) as { operation?: unknown }).operation);
        } catch {
          /* not JSON: Central refuses it */
        }
        process.stdout.write(`${req.method} ${url.pathname} ${operation} -> ${answer.statusCode}${answer.headers["set-cookie"] ? " (sets the cookie)" : ""}\n`);
        res.writeHead(answer.statusCode ?? 502, answer.headers);
        answer.pipe(res);
      });
      upstream.on("error", () => res.writeHead(502).end());
      upstream.end(body);
    });
    return;
  }
  if (req.method !== "GET" && req.method !== "HEAD") {
    res.writeHead(405).end();
    return;
  }
  let file = resolve(join(root, decodeURIComponent(url.pathname)));
  if (file !== root && !file.startsWith(root + sep)) {
    res.writeHead(403).end();
    return;
  }
  if (!(existsSync(file) && statSync(file).isFile())) file = join(root, "index.html");
  res.writeHead(200, {
    "Content-Security-Policy": policy,
    "X-Content-Type-Options": "nosniff",
    "Referrer-Policy": "no-referrer",
    "Content-Type": TYPES[extname(file)] ?? "application/octet-stream",
    "Cache-Control": "no-cache",
  });
  createReadStream(file).pipe(res);
});

server.listen(port, "127.0.0.1", () => {
  process.stdout.write(`https://baylee.app at 127.0.0.1:${port}, /api/* to 127.0.0.1:${listener}\n`);
});
const stop = () => server.close(() => process.exit(0));
process.on("SIGINT", stop);
process.on("SIGTERM", stop);
