// `npm run serve`: the built site (dist/) on this machine, as Caddy serves
// it: every page path answers index.html, the production CSP (plus the local
// feedback service, http://127.0.0.1:28780, which a page on loopback talks
// to) and the same headers. Loopback only; nothing here is for production.
//
//   node scripts/serve.ts [--port 4173] [--dir dist]

import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, resolve, sep } from "node:path";

import { csp } from "./csp.ts";

function arg(name: string, fallback: string): string {
  const at = process.argv.indexOf(`--${name}`);
  return at > 0 && process.argv[at + 1] !== undefined ? (process.argv[at + 1] as string) : fallback;
}

const port = Number(arg("port", process.env["PORT"] ?? "4173"));
const root = resolve(arg("dir", "dist"));
if (!existsSync(join(root, "index.html"))) {
  process.stderr.write(`no ${join(root, "index.html")}: run \`npm run build\` first\n`);
  process.exit(1);
}

const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".txt": "text/plain; charset=utf-8",
  ".woff2": "font/woff2",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".webp": "image/webp",
  ".avif": "image/avif",
  ".ico": "image/x-icon",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
};

const policy = csp(["http://127.0.0.1:28780"]);

const server = createServer((req, res) => {
  // a static site answers reading only; anything else (the directory's POST
  // to /api, say) is not this server's
  if (req.method !== "GET" && req.method !== "HEAD") {
    res.writeHead(405, { Allow: "GET, HEAD" }).end();
    return;
  }
  const url = new URL(req.url ?? "/", "http://localhost");
  let path: string;
  try {
    path = decodeURIComponent(url.pathname);
  } catch {
    res.writeHead(400).end();
    return;
  }
  let file = resolve(join(root, path));
  if (file !== root && !file.startsWith(root + sep)) {
    res.writeHead(403).end();
    return;
  }
  let isFile = existsSync(file) && statSync(file).isFile();
  // a page path (no extension) is the app's: index.html
  if (!isFile && extname(path) === "") {
    file = join(root, "index.html");
    isFile = true;
  }
  const headers: Record<string, string> = {
    "Content-Security-Policy": policy,
    "X-Content-Type-Options": "nosniff",
    "Referrer-Policy": "no-referrer",
  };
  if (/^\/impressum\/?$/.test(path)) headers["X-Robots-Tag"] = "noindex, noarchive, nosnippet";
  if (!isFile) {
    res.writeHead(404, { ...headers, "Content-Type": "text/plain; charset=utf-8" }).end("not found\n");
    return;
  }
  const ext = extname(file);
  headers["Content-Type"] = TYPES[ext] ?? "application/octet-stream";
  headers["Cache-Control"] = ext === ".html" ? "no-cache" : "public, max-age=60";
  res.writeHead(200, headers);
  createReadStream(file).pipe(res);
});

server.listen(port, "127.0.0.1", () => {
  process.stdout.write(`serving ${root} at http://127.0.0.1:${port}/\n`);
});
const stop = () => server.close(() => process.exit(0));
process.on("SIGINT", stop);
process.on("SIGTERM", stop);
