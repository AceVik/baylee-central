// `npm run content -- <baylee checkout>`: copies what this repository does
// not keep from a checkout of Baylee: the card status (web/site/data/, which
// Baylee's `baylee-tools site card-status` writes) into public/data/, and the
// client's screenshots (web/site/img/shot-*) into public/media/. Both carry
// card names and card text, Wizards' material, which stays out of this
// repository (README.md §"Content from Baylee"); both directories are
// gitignored. `npm run build` then copies them into dist/. Plain JavaScript,
// not TypeScript: a deployment runs it on the server's Node, and a
// distribution's build (Ubuntu's `+dfsg`) has no type stripping.

import { cpSync, existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";

const from = process.argv[2];
if (from === undefined) {
  process.stderr.write("usage: npm run content -- <path to a Baylee checkout>\n");
  process.exit(2);
}
const site = join(resolve(from), "web", "site");
const data = join(site, "data");
const img = join(site, "img");
if (!existsSync(join(data, "sets.json")) || !existsSync(img)) {
  process.stderr.write(`no web/site/data/sets.json or web/site/img under ${resolve(from)}\n`);
  process.exit(1);
}

const out = resolve("public");
rmSync(join(out, "data"), { recursive: true, force: true });
cpSync(data, join(out, "data"), { recursive: true });
rmSync(join(out, "media"), { recursive: true, force: true });
mkdirSync(join(out, "media"), { recursive: true });
let shots = 0;
for (const name of readdirSync(img)) {
  if (!/^shot-[a-z-]+(-640)?\.(avif|webp)$/.test(name)) continue;
  cpSync(join(img, name), join(out, "media", name));
  shots++;
}
const sets = readdirSync(join(out, "data", "sets")).length;
process.stdout.write(`copied the card status (${sets} sets) and ${shots} screenshots into public/\n`);
