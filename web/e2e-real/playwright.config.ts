import { defineConfig, devices } from "@playwright/test";

// The account pages against a real Central (e2e-real/account.spec.ts): the
// browser opens https://baylee.app, which this machine maps to a local
// stand-in for the site's Caddy (REAL_FRONT, default 127.0.0.1:8443).
const FRONT = process.env["REAL_FRONT"] ?? "127.0.0.1:8443";

export default defineConfig({
  testDir: ".",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 120_000,
  reporter: [["list"]],
  use: {
    baseURL: "https://baylee.app",
    ignoreHTTPSErrors: true,
    trace: "retain-on-failure",
    launchOptions: { args: [`--host-resolver-rules=MAP baylee.app:443 ${FRONT}`] },
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"], viewport: { width: 1366, height: 900 } } },
    { name: "phone", use: { ...devices["Pixel 7"] } },
  ],
});
