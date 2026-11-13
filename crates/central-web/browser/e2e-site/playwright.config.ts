import { defineConfig, devices } from "@playwright/test";

// The server-rendered site (crates/central-web, docs/proposals/frontend-rewrite.md)
// in a real browser: central-web in its development mode, serving the assets
// `cargo xtask site` built and the synthetic data of
// crates/central-web-server/tests/data. No outside service exists here: a
// request to another origin fails the test that made it. Build first:
//
//   cargo build -p central-web-server && cargo xtask site
//   npx playwright test -c e2e-site/playwright.config.ts
const PORT = 28792;

export default defineConfig({
  testDir: ".",
  outputDir: "../test-results/site",
  fullyParallel: true,
  retries: 0,
  reporter: [["list"]],
  use: { baseURL: `http://127.0.0.1:${PORT}`, trace: "retain-on-failure" },
  webServer: {
    command: "target/debug/central-web --dev target/site crates/central-web-server/tests/data",
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
