import { defineConfig, devices } from "@playwright/test";

// Smoke tests against the built site, served as Caddy serves it (the same
// CSP and headers: scripts/serve.ts). Every outside service is answered by
// e2e/mocks.ts: GitHub, Scryfall's image host, Central, the feedback service
// and the card status; no test reaches a real one. `npm run e2e` builds
// first. Two widths: a phone and a desktop.
const PORT = 4319;

export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  retries: 0,
  reporter: [["list"]],
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: "retain-on-failure",
  },
  webServer: {
    command: `npm run build && node scripts/serve.ts --port ${PORT}`,
    url: `http://127.0.0.1:${PORT}/`,
    reuseExistingServer: false,
    timeout: 60_000,
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"], viewport: { width: 1366, height: 900 } } },
    { name: "phone", use: { ...devices["Pixel 7"] } },
  ],
});
