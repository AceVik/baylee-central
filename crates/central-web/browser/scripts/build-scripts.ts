import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_CONFIGS: readonly string[] = [
  "tsconfig.scripts.dom.json",
  "tsconfig.scripts.classic.json",
  "tsconfig.scripts.sw.json",
  "tsconfig.scripts.workers.json",
];

const EXPECTED_JS: readonly string[] = [
  "badges.js",
  "boot.js",
  "clouds.js",
  "crypto-worker.js",
  "directory.js",
  "hint.js",
  "home.js",
  "islands.js",
  "nav.js",
  "pow-worker.js",
  "pwa.js",
  "security.js",
  "sw.js",
];

const scriptPath = fileURLToPath(import.meta.url);
const browserDir = path.resolve(path.dirname(scriptPath), "..");
const centralWebDir = path.resolve(browserDir, "..");
const jsDir = path.join(centralWebDir, "js");
const tscBin = path.join(browserDir, "node_modules", "typescript", "bin", "tsc");

function fail(message: string): never {
  process.stderr.write(`build-scripts: ${message}\n`);
  process.exit(1);
}

function normalizeLf(text: string): string {
  return `${text.replace(/\r\n?/g, "\n").replace(/\n+$/, "")}\n`;
}

function assertContracts(files: ReadonlyMap<string, string>): void {
  const actualNames = [...files.keys()].sort();
  const expectedNames = [...EXPECTED_JS].sort();
  if (
    actualNames.length !== expectedNames.length ||
    actualNames.some((name, idx) => name !== expectedNames[idx])
  ) {
    fail(
      `unexpected emitted script set: expected [${expectedNames.join(", ")}], got [${actualNames.join(", ")}]`,
    );
  }

  for (const classic of ["boot.js", "sw.js"]) {
    const body = files.get(classic) ?? "";
    if (/^\s*(?:export|import)\b/m.test(body)) {
      fail(`${classic} must be a classic script without top-level import/export statements`);
    }
  }

  const swJs = files.get("sw.js") ?? "";
  for (const needle of ['"__BUILD__"', "/* the offline page's files */ []", '/* the site\'s languages */ ["en"]']) {
    if (!swJs.includes(needle)) {
      fail(`sw.js is missing required site.rs placeholder: ${needle}`);
    }
  }

  const islandsJs = files.get("islands.js") ?? "";
  for (const needle of ['import("./islands-glue.js")', '"./islands.wasm"']) {
    if (!islandsJs.includes(needle)) {
      fail(`islands.js is missing required site.rs placeholder: ${needle}`);
    }
  }

  const cryptoWorkerJs = files.get("crypto-worker.js") ?? "";
  for (const needle of ['"./central_web_crypto.js"', '"./central_web_crypto_bg.wasm"']) {
    if (!cryptoWorkerJs.includes(needle)) {
      fail(`crypto-worker.js is missing required site.rs placeholder: ${needle}`);
    }
  }
}

function compileToTemp(): Map<string, string> {
  if (!fs.existsSync(tscBin)) {
    fail(`TypeScript compiler entry not found at ${tscBin}; run npm ci in crates/central-web/browser`);
  }
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "central-scripts-"));
  try {
    for (const cfg of SCRIPT_CONFIGS) {
      execFileSync(
        process.execPath,
        [tscBin, "-p", path.join(browserDir, cfg), "--outDir", tmpDir],
        { cwd: browserDir, stdio: "inherit" },
      );
    }

    const entries = fs.readdirSync(tmpDir, { withFileTypes: true });
    const out = new Map<string, string>();
    for (const entry of entries) {
      if (!entry.isFile() || !entry.name.endsWith(".js")) {
        fail(`unexpected non-js output from tsc: ${entry.name}`);
      }
      const raw = fs.readFileSync(path.join(tmpDir, entry.name), "utf8");
      const tsName = entry.name.replace(/\.js$/, ".ts");
      const banner = `// GENERATED from crates/central-web/ts/${tsName} via TypeScript 7.1 — do not edit directly.\n`;
      out.set(entry.name, banner + normalizeLf(raw));
    }
    assertContracts(out);
    return out;
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

function main(): void {
  const args = new Set(process.argv.slice(2));
  const checkOnly = args.has("--check");
  const generated = compileToTemp();

  fs.mkdirSync(jsDir, { recursive: true });
  const existing = fs.readdirSync(jsDir).sort();

  if (checkOnly) {
    const expectedNames = [...generated.keys()].sort();
    if (
      existing.length !== expectedNames.length ||
      existing.some((name, idx) => name !== expectedNames[idx])
    ) {
      fail(
        `crates/central-web/js file list differs: on disk [${existing.join(", ")}], built [${expectedNames.join(", ")}]; run npm run build:scripts`,
      );
    }
    for (const [name, content] of generated) {
      const diskPath = path.join(jsDir, name);
      const onDisk = fs.readFileSync(diskPath, "utf8");
      if (onDisk !== content) {
        fail(`crates/central-web/js/${name} is out of date with crates/central-web/ts/${name.replace(/\.js$/, ".ts")}; run npm run build:scripts`);
      }
    }
    process.stdout.write(`Verified ${generated.size} generated browser scripts match crates/central-web/ts/*.ts.\n`);
    return;
  }

  for (const name of existing) {
    if (!generated.has(name)) {
      fs.unlinkSync(path.join(jsDir, name));
    }
  }
  for (const [name, content] of generated) {
    fs.writeFileSync(path.join(jsDir, name), content, { encoding: "utf8" });
  }
  process.stdout.write(`Built ${generated.size} browser scripts into crates/central-web/js/.\n`);
}

main();
