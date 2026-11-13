import { readFileSync } from "node:fs";
import { join } from "node:path";
import { crc32, deflateSync } from "node:zlib";
import { expect, test, type Browser, type Page } from "@playwright/test";

import { appoint, open, problems, register } from "./site.ts";

// A profile picture's whole path against a real Central (docs/profile-pictures.md):
// the browser prepares and sends it through the front (front.ts, which lets
// a picture's upload through to its own bound as Caddy does), Central hands
// it to the worker behind its socket (picture-socket.ts, standing in for
// systemd), stores what comes back and serves it; a contact reports it from
// the chat, an administrator the operator appointed removes it with a
// reason, the person is told and the files are gone. With the worker in a
// container whose ffmpeg has libaom (README.md), an animated GIF also
// becomes an AVIF sequence; without one, REAL_PICTURE_SEQUENCE=0 expects it
// kept as its still.

const password = "violet lantern harbour quietly";
const stamp = String(Date.now() % 1_000_000);
const SEQUENCE = process.env["REAL_PICTURE_SEQUENCE"] !== "0";
const GIF = readFileSync(join(import.meta.dirname, "..", "e2e-site", "pictures", "animated.gif"));

/** One PNG chunk: its length, its kind and data, and their CRC. */
function chunk(kind: string, data: Buffer): Buffer {
  const body = Buffer.concat([Buffer.from(kind, "latin1"), data]);
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const sum = Buffer.alloc(4);
  sum.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, sum]);
}

/** A PNG of `side` × `side` noisy pixels: large after any re-encoding (over 64 KiB). */
function noise(side: number): Buffer {
  const head = Buffer.alloc(13);
  head.writeUInt32BE(side, 0);
  head.writeUInt32BE(side, 4);
  head[8] = 8; // bits
  head[9] = 2; // RGB
  const rows: Buffer[] = [];
  let seed = 12345;
  for (let y = 0; y < side; y += 1) {
    const row = Buffer.alloc(1 + side * 3);
    for (let i = 1; i < row.length; i += 1) {
      seed = (seed * 1103515245 + 12345) >>> 0;
      row[i] = seed >>> 24;
    }
    rows.push(row);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", head),
    chunk("IDAT", deflateSync(Buffer.concat(rows))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** A second browser of its own, as the chat spec makes them. */
async function another(browser: Browser, phone: boolean): Promise<Page> {
  const context = await browser.newContext({
    baseURL: "https://baylee.app",
    ignoreHTTPSErrors: true,
    viewport: phone ? { width: 412, height: 915 } : { width: 1366, height: 900 },
  });
  await context.route("**/*", (route) =>
    new URL(route.request().url()).hostname === "baylee.app" ? route.continue() : route.fulfill({ status: 503, body: "synthetic test: external service disabled" }),
  );
  return context.newPage();
}

/** Chooses `buffer` on the profile card and saves it; the key of what Central stored. */
async function upload(page: Page, name: string, mimeType: string, buffer: Buffer): Promise<string> {
  await open(page, "/en/account/profile");
  await page.locator("#picture-file").setInputFiles({ name, mimeType, buffer });
  await page.getByRole("button", { name: "Save picture" }).click();
  await expect(page.getByRole("status").filter({ hasText: "Your new picture is saved." })).toBeVisible({ timeout: 60_000 });
  const still = page.locator(".picture-current source[type='image/avif']").last();
  await expect(still).toHaveAttribute("srcset", /^\/api\/v1\/pictures\/[0-9a-f]{32}\.avif$/);
  return /([0-9a-f]{32})/.exec((await still.getAttribute("srcset")) ?? "")?.[1] ?? "";
}

/** What a read of a picture's file gave: its status, its headers and its first bytes. */
type Got = { status: number; headers: Record<string, string>; head: number[] };

/**
 * One of a picture's files, read by the page itself as the site reads it.
 * Never through Playwright's own request client (`page.request`): that one
 * goes past the browser's host mapping to whatever `baylee.app` resolves to,
 * which is production, not this run's front. No cache: a removed picture
 * must be asked of Central again.
 */
function file(page: Page, key: string, kind: "avif" | "png" | "sequence.avif"): Promise<Got> {
  return page.evaluate(async (url) => {
    const got = await fetch(url, { cache: "no-store" });
    const bytes = new Uint8Array(await got.arrayBuffer());
    const headers: Record<string, string> = {};
    got.headers.forEach((value, name) => {
      headers[name] = value;
    });
    return { status: got.status, headers, head: Array.from(bytes.subarray(0, 32)) };
  }, `/api/v1/pictures/${key}.${kind}`);
}

const text = (got: Got, from: number, to: number) => Buffer.from(got.head.slice(from, to)).toString("latin1");

/** The headers every served picture carries (docs/profile-pictures.md §"What the service stores and serves"). */
function served(got: Got, type: string) {
  expect(got.status).toBe(200);
  expect(got.headers["content-type"]).toBe(type);
  expect(got.headers["content-security-policy"]).toContain("sandbox");
  expect(got.headers["content-security-policy"]).toContain("default-src 'none'");
  expect(got.headers["x-content-type-options"]).toBe("nosniff");
  expect(got.headers["cross-origin-resource-policy"]).toBe("same-origin");
  expect(got.headers["cache-control"]).toBe("private, max-age=3600");
}

test("a picture goes from the browser through Central and its worker, is reported, removed and gone", async ({ browser, page }, info) => {
  test.setTimeout(480_000);
  const phone = info.project.name === "phone";
  const seenProblems = problems(page);
  await page.context().route("**/*", (route) =>
    new URL(route.request().url()).hostname === "baylee.app" ? route.continue() : route.fulfill({ status: 503, body: "synthetic test: external service disabled" }),
  );
  const bob = await another(browser, phone);
  const carol = await another(browser, phone);
  const [alice_, bob_, carol_] = ["pa", "pb", "pc"].map((p) => `${p}_${info.project.name}_${stamp}`);

  await register(page, alice_ ?? "", "Pic_Alice", password);
  const aliceHandle = (await page.locator(".account-who .handle").textContent()) ?? "";

  // A still over 64 KiB: the browser's square PNG, Central's AVIF and PNG at 256 px
  const still = await upload(page, "noise.png", "image/png", noise(600));
  expect(still).toMatch(/^[0-9a-f]{32}$/);
  const avif = await file(page, still, "avif");
  served(avif, "image/avif");
  expect(text(avif, 4, 12)).toBe("ftypavif");
  const png = await file(page, still, "png");
  served(png, "image/png");
  expect(text(png, 12, 16)).toBe("IHDR");
  const side = Buffer.from(png.head.slice(16, 24));
  expect([side.readUInt32BE(0), side.readUInt32BE(4)]).toEqual([256, 256]);
  expect((await file(page, still, "sequence.avif")).status).toBe(404);

  // An animated GIF, sent whole: a sequence where the worker's ffmpeg has libaom
  const moving = await upload(page, "moving.gif", "image/gif", GIF);
  expect(moving).not.toBe(still);
  // the replaced picture is gone at once
  for (const kind of ["avif", "png"] as const) expect((await file(page, still, kind)).status).toBe(404);
  served(await file(page, moving, "png"), "image/png");
  if (SEQUENCE) {
    await expect(page.locator("#picture-kept-still")).toBeHidden();
    const sequence = await file(page, moving, "sequence.avif");
    served(sequence, "image/avif");
    expect(text(sequence, 4, 12)).toBe("ftypavis");
    await expect(page.locator(`.picture-current source[srcset='/api/v1/pictures/${moving}.sequence.avif']`)).toHaveAttribute(
      "media",
      "(prefers-reduced-motion: no-preference)",
    );
  } else {
    await expect(page.locator("#picture-kept-still")).toBeVisible();
    expect((await file(page, moving, "sequence.avif")).status).toBe(404);
  }
  // the header shows it, hidden from screen readers (on a page with islands)
  await open(page, "/en/account");
  if (!(await page.locator("#nav").isVisible())) await page.locator(".menu-btn").click();
  await expect(page.locator(".nav-account .nav-face img").first()).toHaveAttribute("src", `/api/v1/pictures/${moving}.png`);

  // A contact sees it beside the name and reports it from the chat
  await register(bob, bob_ ?? "", "Pic_Bob", password);
  await open(bob, "/en/account/contacts");
  await bob.getByLabel("Add a contact by their handle").fill(aliceHandle);
  await bob.getByRole("button", { name: "Find", exact: true }).click();
  await bob.getByRole("button", { name: "Send a contact request" }).click();
  await expect(bob.getByText("Request sent.")).toBeVisible();
  await open(page, "/en/account/contacts");
  await page.getByRole("button", { name: "Accept", exact: true }).click();
  await open(bob, "/en/account/chat");
  await expect(bob.getByRole("status").filter({ hasText: "Opening your chats" })).toHaveCount(0, { timeout: 60_000 });
  const row = bob.locator(".conv-row", { hasText: "Pic_Alice" });
  await expect(row.locator(`img[src='/api/v1/pictures/${moving}.png']`)).toHaveCount(1, { timeout: 30_000 });
  await row.click();
  await bob.locator("#conv-details-toggle").click();
  await bob.getByRole("button", { name: "Report picture" }).click();
  await bob.getByLabel("Reason").selectOption({ label: "Harassment" });
  const note = `Synthetic report ${stamp} <b>as text</b>`;
  await bob.getByLabel("Note (optional)").fill(note);
  await bob.getByRole("button", { name: "Send report" }).click();
  await expect(bob.getByText("Thank you. The administrators will look at it.")).toBeVisible({ timeout: 30_000 });

  // The operator appoints an administrator, who removes it with a reason
  await register(carol, carol_ ?? "", "Pic_Carol", password);
  appoint(carol_ ?? "");
  await open(carol, "/en/account/admin/pictures");
  const report = carol.locator(".picture-report-item", { hasText: note });
  await expect(report).toBeVisible({ timeout: 30_000 });
  // the note is text, never markup
  expect(await carol.locator(".picture-report-item b", { hasText: "as text" }).count()).toBe(0);
  await carol.locator("#admin-pictures-password").fill(password);
  await report.getByLabel("Reason").selectOption({ label: "Harassment" });
  await report.getByLabel("A sentence for the person (optional)").fill("Synthetic: please choose another picture.");
  await report.getByRole("button", { name: "Remove the picture" }).click();
  await expect(carol.locator(".picture-report-item", { hasText: note })).toHaveCount(0, { timeout: 60_000 });

  // The person is told, and every file of the picture is gone
  await open(page, "/en/account/profile");
  await expect(page.getByText(/An administrator removed your profile picture on .+\. Reason: Harassment\./)).toBeVisible({ timeout: 30_000 });
  for (const kind of ["avif", "png", "sequence.avif"] as const) {
    const gone = await file(page, moving, kind);
    expect(gone.status, kind).toBe(404);
    expect(gone.headers["cache-control"], kind).toBe("no-store");
  }
  expect(seenProblems).toEqual([]);
  await bob.context().close();
  await carol.context().close();
});
