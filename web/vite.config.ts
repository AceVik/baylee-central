import { defineConfig } from "vitest/config";

// No plugin: Vite's own transform (oxc) compiles TSX with React's automatic
// runtime. The build writes hashed files under `assets/`, which Caddy may
// serve as immutable, and an `index.html` without inline script or style, so
// the site's CSP (`script-src 'self'; style-src 'self'`) holds. `public/`
// (fonts, our own pictures, boot.js, robots.txt) is copied as it is.
export default defineConfig({
  oxc: {
    jsx: { runtime: "automatic" },
  },
  build: {
    outDir: "dist",
    assetsDir: "assets",
    sourcemap: false,
    // The modulepreload polyfill is injected inline in some setups; this app
    // targets current browsers only, and the CSP forbids inline script.
    modulePreload: { polyfill: false },
    target: "es2023",
  },
  worker: {
    format: "es",
  },
  // The legal texts live beside the app (`../legal`), where the service
  // compiles them in too; the page reads the same files at build time.
  server: {
    fs: { allow: [".", "../legal"] },
  },
  test: {
    environment: "jsdom",
    include: ["test/**/*.test.{ts,tsx}"],
    setupFiles: ["test/setup.ts"],
    restoreMocks: true,
    unstubGlobals: true,
  },
});
