// The messenger's interface (docs/proposals/frontend-rewrite.md §6.3)
// against a stand-in for Central and a canned crypto Worker: what the page
// does with what the Worker and Central say, not the cryptography, which
// web/e2e-real/site-chat.spec.ts runs for real. The stand-ins speak
// central-api's exact JSON, which the page parses strictly.
//
// The canned Worker "opens" a delivery by reading its text from the
// ciphertext field (base64url), and "seals" by writing it there; the
// stand-in Central delivers what was sent back to its sender, as Central
// does for the sender's own devices.
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { expect, type Page, type Route, test } from "@playwright/test";

import { axe, targets } from "./check.ts";

const ACCOUNT = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
const OWN = 0x1a2b;
const BEA = 0x2c3d;
const CID = 0x3e4f;
const CONV = "dm:1a2b:2c3d";
const HOSTILE = '<img src=x onerror="document.title=1">';

const b64url = (text: string) => Buffer.from(text, "utf8").toString("base64url");
const device = (id: string) => ({ id, ik: "a".repeat(43), dk: "b".repeat(43), dsig: "c".repeat(86), label: "Linux", added_at: 1 });
const person = (tag: number, display_name: string) => ({ tag, display_name });
const hex = (tag: number) => tag.toString(16).padStart(4, "0");
// Bea's picture moves; Erika and Cid have none (docs/profile-pictures.md).
const BEA_KEY = Array(16).fill(0x6b);
const BEA_FILE = `/api/v1/pictures/${"6b".repeat(16)}`;
const AVIF = readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), "pictures", "imageio.avif"));

interface Delivery {
  seq: number;
  sender: { tag: number; display_name: string };
  device: ReturnType<typeof device>;
  sealed: { hdr: string; n: string; ct: string; epk: string; sig: string; wraps: { d: string; k: string }[] };
  at: number;
}

function delivery(seq: number, sender: number, name: string, text: string, at: number): Delivery {
  const header = { v: 1, id: `0190f3a0-0000-7000-8000-${seq.toString(16).padStart(12, "0")}`, conv: CONV, from: hex(sender), dev: "0123456789abcdef", ts: at, kind: "text" };
  return {
    seq,
    sender: person(sender, name),
    device: device("fedcba9876543210"),
    sealed: { hdr: b64url(JSON.stringify(header)), n: "n".repeat(32), ct: b64url(text), epk: "e".repeat(43), sig: "s".repeat(86), wraps: [{ d: "0123456789abcdef", k: "k".repeat(64) }] },
    at,
  };
}

/** The canned Worker: `changed` names tags whose keys changed. */
function worker(changed: string[]): string {
  return `
const changed = new Set(${JSON.stringify(changed)});
const text = (b64) => new TextDecoder().decode(Uint8Array.from(atob(b64.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0)));
const b64url = (s) => btoa(String.fromCharCode(...new TextEncoder().encode(s))).replace(/\\+/g, "-").replace(/\\//g, "_").replace(/=+$/, "");
let serial = 0;
self.addEventListener("message", (event) => {
  const m = JSON.parse(event.data);
  const ok = (value) => self.postMessage(JSON.stringify({ id: m.id, ok: value }));
  switch (m.op) {
    case "resume": return ok({ device: "0123456789abcdef", registration: { ik: "a".repeat(43), dk: "b".repeat(43), dsig: "c".repeat(86), label: "Linux" } });
    case "restore": return ok({});
    case "observe": return ok({ reviews: m.keys.map((k) => ({ tag: k.tag, number: k.tag === "2c3d" ? "1234567890".repeat(6) : "0987654321".repeat(6), changed: changed.has(k.tag), snapshot: k })), trust: [1, 2, 3] });
    case "confirm": changed.delete(m.snapshot.tag); return ok({ trust: [4, 5, 6] });
    case "open": return ok({ messages: m.messages.map((d) => {
      const header = JSON.parse(text(d.sealed.hdr));
      return { seq: d.seq, message: { id: header.id, conv: header.conv, sender: d.sender.tag, text: text(d.sealed.ct), emote: false, at: d.at } };
    }) });
    case "seal": {
      serial += 1;
      const header = { v: 1, id: "0190f3a1-0000-7000-8000-" + serial.toString(16).padStart(12, "0"), conv: m.conv, from: "1a2b", dev: "0123456789abcdef", ts: m.now, kind: "text" };
      return ok({ sealed: { hdr: b64url(JSON.stringify(header)), n: "n".repeat(32), ct: b64url(m.text), epk: "e".repeat(43), sig: "s".repeat(86), wraps: m.members.map((t, i) => ({ d: (i + 1).toString(16).padStart(16, "0"), k: "k".repeat(64) })) } });
    }
    default: return self.postMessage(JSON.stringify({ id: m.id, error: "failed" }));
  }
});`;
}

/** A stand-in for Central's browser surface with one friend's history. */
function central(inbox: Delivery[]) {
  const sent: string[] = [];
  const pictures: number[][] = [];
  const handle = async (route: Route) => {
    const body = JSON.parse(route.request().postData() ?? "{}");
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    const social = (response: unknown) => json({ result: "social", response });
    if (body.operation === "account_summary") {
      return json({
        result: "account_summary", identity: { account: ACCOUNT, epoch: 1 }, username: "erika", display_name: "Erika", tag: OWN,
        source: "registered", recovery: true, admin: false, signed_in_at: 1_794_000_000, expires_at: 4_102_444_800,
      });
    }
    if (body.operation !== "social") return route.fulfill({ status: 401, body: "rejected\n" });
    const r = body.request;
    if (r.action === "contacts") {
      return social({ kind: "contacts", contacts: [
        { person: person(BEA, "Bea"), relation: "friend" },
        { person: person(CID, "Cid"), relation: "friend" },
        { person: person(0x4a5b, "Dora"), relation: "incoming" },
      ] });
    }
    if (r.action === "group") return social({ kind: "group", response: { kind: "groups", snapshot: { groups: [], invites: [] } } });
    // one batch for every tag the page shows, within the lookup budget's 300
    if (r.action === "pictures") {
      pictures.push(r.tags);
      return social({ kind: "pictures", pictures: r.tags.includes(BEA) ? [{ tag: BEA, key: BEA_KEY, moving: true }] : [] });
    }
    if (r.action !== "chat") return route.fulfill({ status: 401, body: "rejected\n" });
    const c = r.request;
    switch (c.command) {
      case "register": return social({ kind: "chat", response: { kind: "registered", id: "0123456789abcdef" } });
      case "keys": return social({ kind: "chat", response: { kind: "keys", keys: c.tags.map((tag: number) => ({ tag: hex(tag), handle: `${tag === OWN ? "Erika" : tag === BEA ? "Bea" : "Cid"}#${hex(tag)}`, devices: [device(tag === OWN ? "0123456789abcdef" : "fedcba9876543210")] })) } });
      case "inbox": return social({ kind: "chat", response: { kind: "inbox", messages: inbox.filter((d) => d.seq > c.after), more: false } });
      case "send": {
        const header = JSON.parse(Buffer.from(c.sealed.hdr, "base64url").toString("utf8"));
        sent.push(Buffer.from(c.sealed.ct, "base64url").toString("utf8"));
        const at = Date.now();
        inbox.push({ ...delivery(inbox.length + 1, OWN, "Erika", "", at), sealed: { ...c.sealed, wraps: [c.sealed.wraps[0]] } });
        return social({ kind: "chat", response: { kind: "sent", id: header.id, at } });
      }
      default: return route.fulfill({ status: 401, body: "rejected\n" });
    }
  };
  return { handle, sent, pictures };
}

async function open(page: Page, changed: string[] = []) {
  const now = Date.now();
  const stand = central([
    delivery(1, BEA, "Bea", "Hello Erika", now - 3_600_000),
    delivery(2, BEA, "Bea", HOSTILE, now - 3_500_000),
    delivery(3, BEA, "Bea", "Line one\nline two", now - 3_400_000),
  ]);
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") problems.push(message.text());
  });
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  await page.route("**/api/v1", stand.handle);
  await page.route("**/api/v1/pictures/*", (route: Route) => route.fulfill({ status: 200, contentType: "image/avif", body: AVIF }));
  await page.route("**/assets/crypto-worker.*.js", (route) => route.fulfill({ contentType: "text/javascript", body: worker(changed) }));
  return { stand, problems };
}

test.use({ serviceWorkers: "block" });

test("a conversation reads as text, passes axe, and sends from the keyboard", async ({ page }) => {
  const { stand, problems } = await open(page);
  await page.goto(`/en/account/chat?c=${CONV}`);
  const log = page.locator(".conv-msgs");
  await expect(log.getByText("Hello Erika")).toBeVisible({ timeout: 15_000 });
  // the server's lead gives way to the open messenger
  await expect(page.locator(".msgr-lead")).toBeHidden();
  // markup in a message is text: shown as written, never an element
  await expect(log.getByText(HOSTILE, { exact: true })).toBeVisible();
  expect(await page.locator(".conv-msgs img").count()).toBe(0);
  expect(await page.title()).not.toBe("1");
  await expect(page.getByRole("heading", { level: 2, name: "Bea" })).toBeVisible();
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);

  // Enter sends; the text is gone from the composer and back as a message
  const composer = page.getByLabel("Write a message");
  await composer.fill("Sent from the keyboard");
  // on a touch keyboard Enter breaks the line; Ctrl+Enter always sends
  const touch = await page.evaluate(() => matchMedia("(pointer: coarse)").matches);
  await composer.press(touch ? "Control+Enter" : "Enter");
  await expect(log.getByText("Sent from the keyboard")).toBeVisible({ timeout: 15_000 });
  await expect(composer).toHaveValue("");
  expect(stand.sent).toEqual(["Sent from the keyboard"]);
  // Shift+Enter breaks the line instead (and plain Enter on touch)
  await composer.fill("first");
  await composer.press(touch ? "Enter" : "Shift+Enter");
  await expect(composer).toHaveValue("first\n");

  // the details: the safety number in groups of five, the Fan Content line
  await page.getByRole("button", { name: "Details" }).click();
  await expect(page.locator(".safety-number")).toHaveText("12345 67890 12345 67890 12345 67890 12345 67890 12345 67890 12345 67890");
  await expect(page.getByRole("link", { name: "What that means" })).toHaveAttribute("href", "/en/fan-content");
  expect(await axe(page)).toEqual([]);
  await page.keyboard.press("Escape");
  await expect(page.locator("#conv-details")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Details" })).toBeFocused();

  // Ctrl+K finds a conversation (on a phone: from the list)
  if ((page.viewportSize()?.width ?? 0) < 768) {
    await page.getByRole("button", { name: "Back to the conversations" }).click();
    // back in the list, on the conversation it came from
    await expect(page.locator("#conv-dm-1a2b-2c3d")).toBeFocused();
  }
  await page.keyboard.press("Control+k");
  await expect(page.getByLabel("Find a conversation")).toBeFocused();
  expect(problems).toEqual([]);
});

test("the list is one tab stop with arrows, and opens by Enter", async ({ page }) => {
  const { problems } = await open(page);
  await page.goto("/en/account/chat");
  const rows = page.locator(".conv-row");
  await expect(rows).toHaveCount(2, { timeout: 15_000 });
  await expect(rows.nth(0)).toHaveAttribute("tabindex", "0");
  await expect(rows.nth(1)).toHaveAttribute("tabindex", "-1");
  await rows.nth(0).focus();
  await page.keyboard.press("ArrowDown");
  await expect(rows.nth(1)).toBeFocused();
  await expect(rows.nth(1)).toHaveAttribute("tabindex", "0");
  await page.keyboard.press("Home");
  await expect(rows.nth(0)).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { level: 2, name: "Bea" })).toBeFocused();
  await expect(page).toHaveURL(new RegExp(`\\?c=${CONV}$`));
  // the request count beside Contacts (on a phone, from the list)
  if ((page.viewportSize()?.width ?? 0) < 768) await page.getByRole("button", { name: "Back to the conversations" }).click();
  await expect(page.getByRole("link", { name: /Contacts\s*1 requests/ })).toBeVisible();
  await page.goto("/en/account/contacts");
  await expect(page.getByRole("heading", { name: "Requests for you" })).toBeVisible({ timeout: 15_000 });
  expect(await axe(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(problems).toEqual([]);
});

test("changed keys stop writing until the safety number is confirmed", async ({ page }) => {
  const { problems } = await open(page, ["2c3d"]);
  await page.goto(`/en/account/chat?c=${CONV}`);
  await expect(page.getByRole("alert").filter({ hasText: "Someone's keys in this conversation changed." })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByLabel("Write a message")).toHaveCount(0);
  await page.getByRole("button", { name: "Compare the safety number" }).click();
  await expect(page.locator(".safety-item[data-warn]")).toBeVisible();
  // the person is pictured there too, beside the name, at one size
  await expect(page.locator(".safety-item[data-warn] .avatar")).toHaveAttribute("aria-hidden", "true");
  await page.getByRole("button", { name: "It matches: trust the new keys" }).click();
  await expect(page.getByLabel("Write a message")).toBeVisible({ timeout: 15_000 });
  expect(problems).toEqual([]);
});

test("pictures stand beside every name: one size, initials without one, the still where motion is reduced", async ({ page }, info) => {
  // a phone shows the list or the conversation; the desktop both at once
  test.skip(info.project.name === "phone", "the list and the conversation side by side");
  const { stand, problems } = await open(page);
  await page.goto(`/en/account/chat?c=${CONV}`);
  await expect(page.locator(".conv-msgs").getByText("Hello Erika")).toBeVisible({ timeout: 15_000 });
  const pictured = page.locator(`.avatar:has(source[srcset='${BEA_FILE}.sequence.avif'])`);
  await expect(pictured.first()).toBeVisible();
  // the list's row and the conversation's head; one request for all tags
  expect(await pictured.count()).toBeGreaterThanOrEqual(2);
  expect(stand.pictures.length).toBe(1);
  expect(stand.pictures[0]).toEqual(expect.arrayContaining([OWN, BEA, CID, 0x4a5b]));
  // without a picture, the initials: never an empty mark
  const bare = page.locator(".avatar:visible:not(:has(picture))");
  expect(await bare.count()).toBeGreaterThan(0);
  for (const text of await bare.allTextContents()) expect(text.trim()).not.toBe("");
  // one fixed size, and no name read twice: the name stands beside each
  for (const avatar of await page.locator(".avatar:visible").all()) {
    const box = await avatar.boundingBox();
    expect(box && Math.round(box.width) === Math.round(box.height) && box.width >= 24).toBe(true);
    await expect(avatar).toHaveAttribute("aria-hidden", "true");
  }
  for (const image of await page.locator(".avatar img").all()) await expect(image).toHaveAttribute("alt", "");
  const shown = () => pictured.first().locator("img").evaluate((img: HTMLImageElement) => img.currentSrc);
  await expect.poll(shown).toMatch(/\.sequence\.avif$/);
  // asked for less motion, the browser takes the still
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.reload();
  await expect(pictured.first()).toBeVisible({ timeout: 15_000 });
  await expect.poll(shown).toMatch(/[0-9a-f]{32}\.avif$/);
  expect(problems).toEqual([]);
});

test("the messenger fits from 320 px without sideways scrolling", async ({ page }) => {
  await open(page);
  for (const width of [320, 375, 768, 1024, 1440]) {
    await page.setViewportSize({ width, height: 800 });
    await page.goto(`/en/account/chat?c=${CONV}`);
    await expect(page.locator(".conv-msgs").getByText("Hello Erika")).toBeVisible({ timeout: 15_000 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), `${width}px`).toBe(true);
  }
});
