import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import type { Plugin } from "vite";

/** A reproducible public-shell service worker, without a runtime dependency. */
export function pwa(): Plugin {
  return {
    name: "baylee-pwa",
    apply: "build",
    generateBundle: {
      order: "post",
      handler(_, bundle) {
        const worker = readFileSync(new URL("./service-worker.js", import.meta.url), "utf8");
        const files = Object.keys(bundle).filter((name) => name === "index.html" || name.startsWith("assets/")).sort();
        if (!files.includes("index.html")) this.error("PWA public shell missing from build");
        const digest = createHash("sha256").update(worker);
        for (const name of files) {
          const item = bundle[name];
          if (item) digest.update(name).update(item.type === "chunk" ? item.code : item.source);
        }
        for (const name of ["boot.js", "apple-touch-icon.png", "favicon-32.png", "manifest.webmanifest", "pwa-icon.svg", "img/baylee-sunlight-icon.webp", "fonts/Faustina.woff2", "fonts/Faustina-Italic.woff2", "fonts/AlegreyaSans-Regular.woff2", "fonts/AlegreyaSans-Medium.woff2", "fonts/AlegreyaSans-Bold.woff2"]) {
          digest.update(readFileSync(new URL(`../public/${name}`, import.meta.url)));
          files.push(name);
        }
        const version = digest.digest("hex").slice(0, 24);
        this.emitFile({ type: "asset", fileName: "sw.js", source: `const CACHE = "baylee-public-${version}";\nconst ASSETS = ${JSON.stringify(files.map((name) => `/${name}`))};\n${worker}` });
      },
    },
  };
}
