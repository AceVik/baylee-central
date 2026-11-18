import { defineConfig, devices } from "@playwright/test";

// The server-rendered site (crates/central-web, docs/proposals/frontend-rewrite.md)
// in a real browser: central-web in its development mode, serving the assets
// `cargo xtask site` built and the synthetic data of
// crates/central-web-server/tests/data. No outside service exists here: a
// request to another origin fails the test that made it. Build first:
//
//   cargo build -p central-web-server && cargo xtask site
//   npx playwright test -c e2e-site/playwright.config.ts
//
// The development mode listens on 28792, which a local stack's website uses
// too. `CENTRAL_WEB_E2E_PORT=<port>` runs the same server with the same
// settings on another loopback port, so the suite can run beside one.
const DEV_PORT = 28792;
const asked = Number(process.env.CENTRAL_WEB_E2E_PORT ?? DEV_PORT);
const PORT = Number.isInteger(asked) && asked > 1023 && asked < 65536 ? asked : DEV_PORT;
const settings = JSON.stringify({
  listen: `127.0.0.1:${PORT}`,
  origin: `http://127.0.0.1:${PORT}`,
  site: "target/site",
  github: false,
  fallback: "crates/central-web-server/tests/data",
});
const command =
  PORT === DEV_PORT
    ? "target/debug/central-web --dev target/site crates/central-web-server/tests/data"
    : `printf '%s' '${settings}' | target/debug/central-web --config-stdin`;

export default defineConfig({
  testDir: ".",
  outputDir: "../test-results/site",
  fullyParallel: true,
  retries: 0,
  reporter: [["list"]],
  use: { baseURL: `http://127.0.0.1:${PORT}`, trace: "retain-on-failure" },
  webServer: {
    command,
    cwd: "../../../..",
    url: `http://127.0.0.1:${PORT}/robots.txt`,
    reuseExistingServer: false,
    timeout: 30_000,
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } } },
    { name: "phone", use: { ...devices["Pixel 7"] } },
  ],
});
