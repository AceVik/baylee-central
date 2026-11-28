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
//
// Which spec runs at which width: the desktop project (1440 px) runs every
// spec; the phone project (Pixel 7) runs only the specs whose subject
// depends on the layout (PHONE_SPECS below: the pages and their bar, the
// languages, the strip, the dialogs, the theme, the messenger, the
// registration's steps, the installable app, the security page, the
// leaderboards). The other
// specs test behaviour no width changes (the administration's and the
// account's flows, mail, feedback, pictures, passwords, ...), so a second
// run at a phone's width would only repeat them. A test that sets its own
// window sizes in a loop is likewise independent of the project's size and
// runs once, in the desktop project (`onlyDesktop` in check.ts).
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
  process.platform === "win32"
    ? PORT === DEV_PORT
      ? "target\\debug\\central-web.exe --dev target/site crates/central-web-server/tests/data"
      : `node -e "const{spawn}=require('node:child_process');const p=spawn('target/debug/central-web.exe',['--config-stdin'],{stdio:['pipe','inherit','inherit']});p.stdin.end(process.env.CENTRAL_WEB_E2E_CFG);p.on('exit',c=>process.exit(c??1))"`
    : PORT === DEV_PORT
      ? "target/debug/central-web --dev target/site crates/central-web-server/tests/data"
      : `printf '%s' '${settings}' | target/debug/central-web --config-stdin`;

const PHONE_SPECS = /\/(site|strip|dialogs|theme|chat|recovery|pwa|security|ranked)\.spec\.ts$/;

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
    env: { CENTRAL_WEB_E2E_CFG: settings },
    url: `http://127.0.0.1:${PORT}/robots.txt`,
    reuseExistingServer: false,
    timeout: 30_000,
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } } },
    { name: "phone", use: { ...devices["Pixel 7"] }, testMatch: PHONE_SPECS },
  ],
});
