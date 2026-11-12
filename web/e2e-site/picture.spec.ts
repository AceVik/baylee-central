import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { crc32, deflateSync } from "node:zlib";

import { expect, type Page, type Route, test } from "@playwright/test";

import { axe, narrow, schemes } from "./check.ts";

// Profile pictures on the profile page (crates/central-web/src/islands/picture.rs,
// docs/profile-pictures.md) in a real browser, against a stand-in for
// Central's browser surface. Every picture is synthetic: drawn here, or the
// files in pictures/ (README there). The page opens the file itself, places
// a square and sends a lossless square PNG; Central's own decoding of
// originals is the codec's tests (crates/central-picture-codec/tests).

const ACCOUNT = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
const KEY = Array(16).fill(0x5a);
const HERE = path.dirname(fileURLToPath(import.meta.url));
const fixture = (name: string) => readFileSync(path.join(HERE, "pictures", name));

type Pixel = [number, number, number, number];

/** A PNG drawn by `pixel`, RGBA, 8 bits. */
function png(width: number, height: number, pixel: (x: number, y: number) => Pixel): Buffer {
  const rows: number[] = [];
  for (let y = 0; y < height; y++) {
    rows.push(0);
    for (let x = 0; x < width; x++) rows.push(...pixel(x, y));
  }
  const chunk = (kind: string, data: Buffer) => {
    const head = Buffer.alloc(4);
    head.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(kind, "latin1"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body) >>> 0);
    return Buffer.concat([head, body, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr.set([8, 6, 0, 0, 0], 8);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(Buffer.from(rows))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const RED: Pixel = [255, 0, 0, 255];
const BLUE: Pixel = [0, 0, 255, 255];
/** Top-left quarter red, the rest blue. */
const quadrant = (w: number, h: number) => (x: number, y: number): Pixel => (x < w / 2 && y < h / 2 ? RED : BLUE);

/** A JPEG with an EXIF orientation segment put in front of its own. */
function oriented(jpeg: Buffer, orientation: number): Buffer {
  const tiff = Buffer.from([0x4d, 0x4d, 0, 0x2a, 0, 0, 0, 8, 0, 1, 0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, orientation, 0, 0, 0, 0, 0, 0]);
  const payload = Buffer.concat([Buffer.from("Exif\0\0", "latin1"), tiff]);
  const length = Buffer.alloc(2);
  length.writeUInt16BE(payload.length + 2);
  return Buffer.concat([jpeg.subarray(0, 2), Buffer.from([0xff, 0xe1]), length, payload, jpeg.subarray(2)]);
}

type Upload = { kind: string; body: Buffer };
type World = { picture: boolean; uploads: Upload[]; asked: string[]; answer?: { status: number; body: string } };

const mine = (w: World) => ({
  result: "my_picture",
  picture: w.picture ? { key: KEY, side: 256, avif_bytes: 409, png_bytes: 200, sha256: Array(32).fill(1), set_at: 1_794_000_000 } : null,
  notice: null,
  block: null,
});

async function open(page: Page, w: World, options: { admin?: boolean; path?: string } = {}) {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("dialog", (dialog) => {
    problems.push(`a dialog: ${dialog.message()}`);
    void dialog.dismiss();
  });
  page.on("console", (message) => {
    const text = message.text();
    // the stand-in's refusals are statuses the page reads, not faults
    if (message.type() === "error" && !/status of (401|403|422)/.test(text)) problems.push(text);
  });
  await page.addInitScript(() => {
    localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 }));
    document.addEventListener("securitypolicyviolation", (event) => {
      console.error(`csp: ${event.violatedDirective} ${event.blockedURI}`);
    });
  });
  await page.route("**/api/v1", (route: Route) => {
    const body = JSON.parse(route.request().postData() ?? "{}");
    w.asked.push(body.request?.action ? `${body.operation}:${body.request.action}` : body.operation);
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    switch (body.operation) {
      case "account_summary":
        return json({
          result: "account_summary", identity: { account: ACCOUNT, epoch: 1 }, username: "erika", display_name: "Erika", tag: 0x1a2b,
          source: "registered", recovery: true, admin: options.admin ?? false, signed_in_at: 1_794_000_000, expires_at: 4_102_444_800,
        });
      case "picture":
        if (body.request.action === "remove") {
          w.picture = false;
          return json({ result: "done" });
        }
        return json(mine(w));
      case "admin":
        return json({
          result: "admin",
          answer: {
            answer: "picture_reports",
            more: false,
            reports: [{
              id: ACCOUNT, account: { id: ACCOUNT, display_name: "Mallory", tag: 0x0f0f }, sha256: Array(32).fill(3), current: KEY,
              reporter: null, reason: "hate", note: "Synthetic note <b>not bold</b>", created_at: 1_794_000_000, state: "open", decided_at: null,
            }],
          },
        });
      default:
        return route.fulfill({ status: 401, body: "account request rejected\n" });
    }
  });
  await page.route("**/api/v1/picture", async (route: Route) => {
    const request = route.request();
    expect(request.method()).toBe("PUT");
    expect(request.headers()["x-baylee-central"]).toBe("1");
    w.uploads.push({ kind: request.headers()["content-type"], body: request.postDataBuffer() ?? Buffer.alloc(0) });
    if (w.answer) return route.fulfill({ status: w.answer.status, contentType: "application/json", body: w.answer.body });
    w.picture = true;
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(mine(w)) });
  });
  await page.route("**/api/v1/pictures/*", (route: Route) => {
    const avif = route.request().url().endsWith(".avif");
    return route.fulfill({
      status: 200,
      contentType: avif ? "image/avif" : "image/png",
      body: avif ? fixture("imageio.avif") : png(8, 8, () => RED),
    });
  });
  await page.goto(options.path ?? "/en/account/profile");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  return problems;
}

const world = (picture = false): World => ({ picture, uploads: [], asked: [] });

/** Chooses `file`, waits for the square, saves it. */
async function choose(page: Page, name: string, mimeType: string, buffer: Buffer) {
  await page.locator("#picture-file").setInputFiles({ name, mimeType, buffer });
  await expect(page.locator("#picture-preview")).toBeVisible({ timeout: 15_000 });
}

/** What the browser itself reads of a PNG the page sent. */
async function decoded(page: Page, body: Buffer) {
  return page.evaluate(async (bytes) => {
    const bitmap = await createImageBitmap(new Blob([new Uint8Array(bytes)], { type: "image/png" }));
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const context = canvas.getContext("2d");
    if (!context) throw new Error("no 2d context");
    context.drawImage(bitmap, 0, 0);
    const at = (x: number, y: number) => Array.from(context.getImageData(x, y, 1, 1).data);
    const side = bitmap.width;
    return { width: bitmap.width, height: bitmap.height, tl: at(8, 8), tr: at(side - 8, 8), bl: at(8, side - 8), br: at(side - 8, side - 8) };
  }, Array.from(body));
}

const near = (a: number[], b: Pixel, tolerance = 48) => a.every((v, i) => Math.abs(v - b[i]) <= tolerance);

test("every kind of picture the browser opens becomes a square PNG without its details", async ({ page }) => {
  const w = world();
  const problems = await open(page, w);
  const cases: [string, string, Buffer, "quadrant" | "red"][] = [
    ["wide.png", "image/png", png(96, 64, quadrant(96, 64)), "quadrant"],
    ["photo.jpg", "image/jpeg", fixture("quadrant.jpg"), "quadrant"],
    ["moving.gif", "image/gif", fixture("animated.gif"), "red"],
    ["moving.webp", "image/webp", fixture("animated.webp"), "red"],
    ["picture.avif", "image/avif", fixture("imageio.avif"), "quadrant"],
    ["paint.bmp", "image/bmp", fixture("quadrant.bmp"), "quadrant"],
    ["icon.ico", "image/x-icon", fixture("quadrant.ico"), "quadrant"],
  ];
  for (const [name, mimeType, buffer, drawn] of cases) {
    await choose(page, name, mimeType, buffer);
    await page.getByRole("button", { name: "Save picture" }).click();
    await expect(page.getByRole("status").filter({ hasText: "Your new picture is saved." })).toBeVisible();
    const sent = w.uploads.at(-1)!;
    expect(sent.kind, name).toBe("image/png");
    expect(sent.body.subarray(1, 4).toString("latin1"), name).toBe("PNG");
    // nothing of the file's own metadata travels: only IHDR, IDAT and IEND
    expect(sent.body.includes(Buffer.from("Exif")), name).toBe(false);
    const seen = await decoded(page, sent.body);
    expect(seen.width, name).toBe(seen.height);
    expect(seen.width >= 256 && seen.width <= 512, `${name}: ${seen.width}`).toBe(true);
    if (drawn === "red") {
      expect(near(seen.br, RED), `${name}: ${seen.br}`).toBe(true);
    } else {
      expect(near(seen.tl, RED), `${name} top left: ${seen.tl}`).toBe(true);
      expect(near(seen.br, BLUE), `${name} bottom right: ${seen.br}`).toBe(true);
    }
  }
  // the stored picture is shown, AVIF first
  await expect(page.locator(".picture-current source[type='image/avif']")).toHaveAttribute("srcset", `/api/v1/pictures/${"5a".repeat(16)}.avif`);
  expect(problems).toEqual([]);
});

test("a photo's own orientation turns it upright before the square is cut", async ({ page }) => {
  const w = world();
  await open(page, w);
  await choose(page, "turned.jpg", "image/jpeg", oriented(fixture("quadrant.jpg"), 6));
  await page.getByRole("button", { name: "Save picture" }).click();
  await expect.poll(() => w.uploads.length).toBe(1);
  const seen = await decoded(page, w.uploads[0].body);
  // 6 turns it clockwise: the red quarter goes to the top right
  expect(near(seen.tr, RED), `${seen.tr}`).toBe(true);
  expect(near(seen.tl, BLUE), `${seen.tl}`).toBe(true);
});

test("an SVG is drawn through an image, without its script and without loading anything", async ({ page }) => {
  const w = world();
  const problems = await open(page, w);
  const outside: string[] = [];
  page.on("request", (request) => {
    if (!request.url().startsWith("http://127.0.0.1:28792")) outside.push(request.url());
  });
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 100" width="100" height="100" onload="alert('onload')">
    <script>alert('script')</script>
    <rect width="100" height="100" fill="#f00"/>
    <image href="https://example.invalid/x.png" width="10" height="10"/>
  </svg>`;
  await choose(page, "drawing.svg", "image/svg+xml", Buffer.from(svg));
  await page.getByRole("button", { name: "Save picture" }).click();
  await expect.poll(() => w.uploads.length).toBe(1);
  expect(w.uploads[0].kind).toBe("image/png");
  const seen = await decoded(page, w.uploads[0].body);
  // the rectangle stays red away from the corner where the browser drew the refused remote image its own way
  expect(near(seen.br, RED) && near(seen.tr, RED) && near(seen.bl, RED), `${seen.br} ${seen.tr} ${seen.bl}`).toBe(true);
  expect(problems).toEqual([]);
  expect(outside).toEqual([]);
});

test("a picture that claims too many pixels is refused before the browser decodes it", async ({ page }) => {
  const w = world();
  await open(page, w);
  const head = png(1, 1, () => RED);
  head.writeUInt32BE(30_000, 16);
  head.writeUInt32BE(30_000, 20);
  await page.locator("#picture-file").setInputFiles({ name: "huge.png", mimeType: "image/png", buffer: head });
  await expect(page.getByRole("alert").filter({ hasText: "This picture has too many pixels." })).toBeVisible();
  await page.locator("#picture-file").setInputFiles({ name: "big.png", mimeType: "image/png", buffer: Buffer.alloc(26 * 1024 * 1024) });
  await expect(page.getByRole("alert").filter({ hasText: "This file is larger than 25 MB." })).toBeVisible();
  await page.locator("#picture-file").setInputFiles({ name: "empty.txt", mimeType: "text/plain", buffer: Buffer.alloc(0) });
  await expect(page.getByRole("alert").filter({ hasText: "This browser cannot open this file as a picture." })).toBeVisible();
  expect(w.uploads).toEqual([]);
});

test("a file the browser cannot open goes to Central as it is, and Central's refusal is named", async ({ page }) => {
  const w = world();
  w.answer = { status: 422, body: JSON.stringify({ refusal: "unsupported" }) };
  await open(page, w);
  const html = Buffer.from("<html><body><script>alert(1)</script></body></html>");
  await choose(page, "renamed.png", "image/png", html);
  await expect(page.locator("#picture-original")).toBeVisible();
  await page.getByRole("button", { name: "Save picture" }).click();
  await expect(page.getByRole("alert").filter({ hasText: "Baylee Central does not take this kind of file." })).toBeVisible();
  expect(w.uploads[0].kind).toBe("image/png");
  expect(w.uploads[0].body.equals(html)).toBe(true);
});

test("a browser that will not let the page read its canvas sends the file itself", async ({ page }) => {
  await page.addInitScript(() => {
    // as a hardened browser does: every read gives the same blank picture
    HTMLCanvasElement.prototype.toDataURL = () => "data:,";
  });
  const w = world();
  await open(page, w);
  await choose(page, "photo.jpg", "image/jpeg", fixture("quadrant.jpg"));
  await expect(page.locator("#picture-original")).toBeVisible();
  await page.getByRole("button", { name: "Save picture" }).click();
  await expect.poll(() => w.uploads.length).toBe(1);
  expect(w.uploads[0].kind).toBe("image/jpeg");
  expect(w.uploads[0].body.equals(fixture("quadrant.jpg"))).toBe(true);
});

test("the keyboard and the slider place the square, and a live region says where", async ({ page }) => {
  const w = world();
  await open(page, w);
  await choose(page, "wide.png", "image/png", png(300, 100, quadrant(300, 100)));
  const where = page.locator("#picture-position");
  await expect(where).toHaveText("Square at 50 % across and 50 % down, zoomed to 100 %.");
  await page.locator("#picture-preview").focus();
  await page.keyboard.press("ArrowLeft");
  await expect(where).toHaveText("Square at 48 % across and 50 % down, zoomed to 100 %.");
  await page.keyboard.press("Shift+ArrowLeft");
  await page.keyboard.press("Shift+ArrowLeft");
  await page.keyboard.press("Shift+ArrowLeft");
  await page.keyboard.press("Shift+ArrowLeft");
  await expect(where).toHaveText("Square at 17 % across and 50 % down, zoomed to 100 %.");
  await page.locator("#picture-zoom").fill("200");
  await expect(where).toContainText("zoomed to 200 %");
  await page.getByRole("button", { name: "Save picture" }).click();
  await expect.poll(() => w.uploads.length).toBe(1);
  const seen = await decoded(page, w.uploads[0].body);
  // the left edge's quarter, zoomed: red at the top left
  expect(near(seen.tl, RED), `${seen.tl}`).toBe(true);
});

test("refusals are named, a block is said, and removing asks first", async ({ page }) => {
  const w = world(true);
  await open(page, w);
  for (const [status, body, text] of [
    [422, { refusal: "too_small" }, "This picture is too small. Choose one at least 16 pixels on a side."],
    [422, { refusal: "malformed" }, "Baylee Central could not read this picture. It may be damaged."],
    [403, { refusal: "blocked" }, "An administrator has blocked new pictures for your account."],
    [429, {}, "You changed your picture often just now. Try again later."],
  ] as const) {
    w.answer = { status, body: JSON.stringify(body) };
    await choose(page, "dot.png", "image/png", png(64, 64, () => BLUE));
    await page.getByRole("button", { name: "Save picture" }).click();
    await expect(page.getByRole("alert").filter({ hasText: text })).toBeVisible();
    await page.getByRole("button", { name: "Cancel" }).click();
  }
  await page.getByRole("button", { name: "Remove picture" }).click();
  await expect(page.getByText("Remove your profile picture? Everyone sees your initials again.")).toBeVisible();
  await page.getByRole("button", { name: "Remove", exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: "Your picture is removed." })).toBeVisible();
  expect(w.asked).toContain("picture:remove");
  await expect(page.locator("#picture-none")).toBeVisible();
});

test("the card at 320 px and in both schemes", async ({ page }) => {
  const w = world(true);
  const problems = await open(page, w);
  await choose(page, "wide.png", "image/png", png(96, 64, quadrant(96, 64)));
  expect(await narrow(page)).toBe(0);
  expect(await schemes(page)).toEqual([]);
  expect(problems).toEqual([]);
});

test("only the profile page allows blob: pictures", async ({ page }) => {
  const profile = await page.request.get("/en/account/profile");
  expect(profile.headers()["content-security-policy"]).toContain("img-src 'self' blob:;");
  for (const other of ["/en/account", "/en/account/privacy", "/en/chat", "/en/"]) {
    const answer = await page.request.get(other);
    expect(answer.headers()["content-security-policy"], other).toContain("img-src 'self';");
  }
});

test("the administrators see reported pictures with their reasons and notes as text", async ({ page }) => {
  const w = world();
  const problems = await open(page, w, { admin: true, path: "/en/account/admin" });
  const section = page.locator("section", { has: page.getByRole("heading", { name: "Reported profile pictures" }) });
  await expect(section.getByText("Mallory#0f0f")).toBeVisible();
  await expect(section.getByText("Synthetic note <b>not bold</b>")).toBeVisible();
  await expect(section.locator("b")).toHaveCount(0);
  await expect(section.getByText("a deleted account")).toBeVisible();
  await expect(section.getByRole("button", { name: "Remove the picture" })).toBeVisible();
  expect(await axe(page)).toEqual([]);
  expect(problems).toEqual([]);
});
