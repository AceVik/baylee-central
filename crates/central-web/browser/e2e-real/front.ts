// A stand-in for baylee.app's Caddy site block (../../../../docs/operations.md
// §"The browser listener"), for a local run against a real Central only
// (e2e-real/README.md): TLS for baylee.app with a throwaway certificate,
// /api/* passed to Central's browser listener with the Host unchanged and
// X-Forwarded-For set to the client, as Caddy does without trusted_proxies (it
// replaces whatever the client sent), and every other request passed to the
// server-rendered site (crates/central-web) on 127.0.0.1:<port> without the
// cookie, as the same Caddy block does. Loopback only; nothing here is for
// production.
//
//   node e2e-real/front.ts --tls <dir with cert.pem, key.pem> --site <port> [--port 8443] [--listener 47921]
// Omit --tls only for an isolated numeric-loopback development demo. Its
// browser listener and separately built crypto module must use the matching
// development origin/issuer; production assets and cookie checks stay intact.

import { readFileSync } from "node:fs";
import { request, createServer as createHttpServer, type IncomingMessage, type ServerResponse } from "node:http";
import { createServer as createHttpsServer } from "node:https";
import { join } from "node:path";

function arg(name: string, fallback: string): string {
  const at = process.argv.indexOf(`--${name}`);
  return at > 0 && process.argv[at + 1] !== undefined ? (process.argv[at + 1] as string) : fallback;
}

const port = Number(arg("port", "8443"));
const listener = Number(arg("listener", "47921"));
const tls = arg("tls", "");
const site = Number(arg("site", "0"));
const LIMIT = 65_536;
/** A profile picture's upload, as the Caddy block's own handle lets it through (docs/operations.md). */
const PICTURE_LIMIT = 9 * 1024 * 1024;

if (!(site > 0)) {
  process.stderr.write("front.ts needs --site <port>: the server-rendered site on 127.0.0.1\n");
  process.exit(2);
}

const origin = tls ? "https://baylee.app" : `http://127.0.0.1:${port}`;
const handle = (req: IncomingMessage, res: ServerResponse) => {
  const url = new URL(req.url ?? "/", origin);
  if (url.pathname.startsWith("/api/")) {
    // request_body max_size 64KiB (9MiB for a picture's upload), then
    // reverse_proxy 127.0.0.1:<listener>
    const limit = req.method === "PUT" && url.pathname === "/api/v1/picture" ? PICTURE_LIMIT : LIMIT;
    const chunks: Buffer[] = [];
    let size = 0;
    req.on("data", (chunk: Buffer) => {
      size += chunk.length;
      if (size <= limit) chunks.push(chunk);
    });
    req.on("end", () => {
      if (size > limit) {
        res.writeHead(413).end();
        return;
      }
      const body = Buffer.concat(chunks);
      const headers = {
        ...req.headers,
        "content-length": String(body.length),
        "x-forwarded-for": req.socket.remoteAddress ?? "",
        "x-forwarded-proto": tls ? "https" : "http",
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
  // header_up -Cookie: the site never sees the session
  const headers = { ...req.headers, "x-forwarded-for": req.socket.remoteAddress ?? "" };
  delete headers.cookie;
  const upstream = request({ host: "127.0.0.1", port: site, method: req.method, path: req.url, headers }, (answer) => {
    res.writeHead(answer.statusCode ?? 502, answer.headers);
    answer.pipe(res);
  });
  upstream.on("error", () => {
    if (!res.headersSent) res.writeHead(502);
    res.end();
  });
  req.pipe(upstream);
};

// A numeric loopback HTTP origin is also supported by Central's development
// listener. Its cookie and origin checks stay enabled; no authentication mock.
const server = tls
  ? createHttpsServer({ key: readFileSync(join(tls, "key.pem")), cert: readFileSync(join(tls, "cert.pem")) }, handle)
  : createHttpServer(handle);

server.listen(port, "127.0.0.1", () => {
  process.stdout.write(`${origin} at 127.0.0.1:${port}, /api/* to 127.0.0.1:${listener}\n`);
});
const stop = () => server.close(() => process.exit(0));
process.on("SIGINT", stop);
process.on("SIGTERM", stop);
