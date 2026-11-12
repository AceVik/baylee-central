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

import { expect, type Locator, type Page, type Route, test } from "@playwright/test";

import { axe, narrow, schemes, targets } from "./check.ts";

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

function delivery(seq: number, sender: number, name: string, text: string, at: number, conv = CONV): Delivery {
  const header = { v: 1, id: `0190f3a0-0000-7000-8000-${seq.toString(16).padStart(12, "0")}`, conv, from: hex(sender), dev: "0123456789abcdef", ts: at, kind: "text" };
  return {
    seq,
    sender: person(sender, name),
    device: device("fedcba9876543210"),
    sealed: { hdr: b64url(JSON.stringify(header)), n: "n".repeat(32), ct: b64url(text), epk: "e".repeat(43), sig: "s".repeat(86), wraps: [{ d: "0123456789abcdef", k: "k".repeat(64) }] },
    at,
  };
}

/**
 * The canned Worker: `changed` names tags whose keys changed; without
 * `opens` it resumes no identity the chat can use (no device), so the chat
 * cannot be opened.
 */
function worker(changed: string[], opens = true): string {
  return `
const changed = new Set(${JSON.stringify(changed)});
const text = (b64) => new TextDecoder().decode(Uint8Array.from(atob(b64.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0)));
const b64url = (s) => btoa(String.fromCharCode(...new TextEncoder().encode(s))).replace(/\\+/g, "-").replace(/\\//g, "_").replace(/=+$/, "");
let serial = 0;
self.addEventListener("message", (event) => {
  const m = JSON.parse(event.data);
  const ok = (value) => self.postMessage(JSON.stringify({ id: m.id, ok: value }));
  switch (m.op) {
    case "resume": return ok(${opens} ? { device: "0123456789abcdef", registration: { ik: "a".repeat(43), dk: "b".repeat(43), dsig: "c".repeat(86), label: "Linux" } } : {});
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

interface Named {
  tag: number;
  display_name: string;
}

interface Contact {
  person: Named;
  relation: "friend" | "incoming" | "outgoing" | "blocked";
}

interface Snapshot {
  groups: { id: string; name: string; members: { person: Named; owner: boolean }[]; invited: Named[] }[];
  invites: { group: string; name: string; inviter: Named; at: number }[];
}

/**
 * What the stand-in holds: the contacts, the groups, the inbox, whether
 * sending fails, how long the contacts take, and how many reads of them
 * fail first.
 */
interface World {
  contacts: Contact[];
  groups: Snapshot;
  inbox: Delivery[];
  failSend?: boolean;
  contactsWait?: number;
  contactsFail?: number;
  people?: Named[];
}

/** The names the stand-in gives each tag's keys. */
const NAMES = new Map<number, string>([
  [OWN, "Erika"],
  [BEA, "Bea"],
  [CID, "Cid"],
]);

/** Two friends and one request, no group, and the history given. */
function world(inbox: Delivery[]): World {
  return {
    contacts: [
      { person: person(BEA, "Bea"), relation: "friend" },
      { person: person(CID, "Cid"), relation: "friend" },
      { person: person(0x4a5b, "Dora"), relation: "incoming" },
    ],
    groups: { groups: [], invites: [] },
    inbox,
  };
}

/** A stand-in for Central's browser surface. */
function central(held: World) {
  const inbox = held.inbox;
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
      if (held.contactsWait) await new Promise((done) => setTimeout(done, held.contactsWait));
      if (held.contactsFail) {
        held.contactsFail -= 1;
        return route.fulfill({ status: 503, body: "unavailable\n" });
      }
      return social({ kind: "contacts", contacts: held.contacts });
    }
    if (r.action === "group") return social({ kind: "group", response: { kind: "groups", snapshot: held.groups } });
    if (r.action === "lookup") return social({ kind: "person", person: held.people?.find((p) => p.tag === r.tag) ?? null });
    // one batch for every tag the page shows, within the lookup budget's 300
    if (r.action === "pictures") {
      pictures.push(r.tags);
      return social({ kind: "pictures", pictures: r.tags.includes(BEA) ? [{ tag: BEA, key: BEA_KEY, moving: true }] : [] });
    }
    if (r.action !== "chat") return route.fulfill({ status: 401, body: "rejected\n" });
    const c = r.request;
    switch (c.command) {
      case "register": return social({ kind: "chat", response: { kind: "registered", id: "0123456789abcdef" } });
      case "keys": return social({ kind: "chat", response: { kind: "keys", keys: c.tags.map((tag: number) => ({ tag: hex(tag), handle: `${NAMES.get(tag) ?? "Someone"}#${hex(tag)}`, devices: [device(tag === OWN ? "0123456789abcdef" : "fedcba9876543210")] })) } });
      case "inbox": return social({ kind: "chat", response: { kind: "inbox", messages: inbox.filter((d) => d.seq > c.after), more: false } });
      case "send": {
        if (held.failSend) return route.fulfill({ status: 503, body: "unavailable\n" });
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

/** Bea's three lines in the direct conversation, an hour ago. */
function history(): Delivery[] {
  const now = Date.now();
  return [
    delivery(1, BEA, "Bea", "Hello Erika", now - 3_600_000),
    delivery(2, BEA, "Bea", HOSTILE, now - 3_500_000),
    delivery(3, BEA, "Bea", "Line one\nline two", now - 3_400_000),
  ];
}

async function open(page: Page, changed: string[] = [], held: World = world(history()), opens = true) {
  const stand = central(held);
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") problems.push(message.text());
  });
  await page.addInitScript(() => localStorage.setItem("baylee-central-session", JSON.stringify({ expiresAt: 4_102_444_800 })));
  await page.route("**/api/v1", stand.handle);
  await page.route("**/api/v1/pictures/*", (route: Route) => route.fulfill({ status: 200, contentType: "image/avif", body: AVIF }));
  await page.route("**/assets/crypto-worker.*.js", (route) => route.fulfill({ contentType: "text/javascript", body: worker(changed, opens) }));
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
  // the system line and the blocked composer, in both schemes
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
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

// ---- A fuller world: a group, own lines, every kind of contact ------------

const GROUP = "0190f3a0-1111-7000-8000-000000000001";
const GROUP_CONV = `grp:${GROUP}`;
const INVITE = "0190f3a0-2222-7000-8000-000000000002";
const CID_CONV = "dm:1a2b:3e4f";
const MINUTE = 60_000;

/** Yesterday at noon, local time: a run of lines that never crosses midnight. */
function noon(): number {
  const day = new Date();
  day.setDate(day.getDate() - 1);
  day.setHours(12, 0, 0, 0);
  return day.getTime();
}

/**
 * Bea's history and Erika's answers, a group of three where Bea writes two
 * lines in a row, Cid one, Erika two, and Bea again after a pause, an
 * invitation, a sent request and a blocked account. Cid's direct
 * conversation has no message.
 */
function rich(): World {
  const now = Date.now();
  const at = noon();
  return {
    contacts: [
      { person: person(BEA, "Bea"), relation: "friend" },
      { person: person(CID, "Cid"), relation: "friend" },
      { person: person(0x4a5b, "Dora"), relation: "incoming" },
      { person: person(0x5c6d, "Finnegan-Whitley"), relation: "outgoing" },
      { person: person(0x6e7f, "Gus"), relation: "blocked" },
    ],
    groups: {
      groups: [
        {
          id: GROUP,
          name: "Tuesday table",
          members: [
            { person: person(OWN, "Erika"), owner: true },
            { person: person(BEA, "Bea"), owner: false },
            { person: person(CID, "Cid"), owner: false },
          ],
          invited: [],
        },
      ],
      invites: [{ group: INVITE, name: "Friday drafts", inviter: person(BEA, "Bea"), at: now - 10 * MINUTE }],
    },
    inbox: [
      ...history(),
      delivery(4, OWN, "Erika", "Hi Bea! Good to hear from you.", now - 3_300_000),
      delivery(5, OWN, "Erika", "Are you coming on Tuesday?", now - 3_280_000),
      delivery(6, BEA, "Bea", "Who brings the cards on Tuesday?", at, GROUP_CONV),
      delivery(7, BEA, "Bea", "I can bring my old deck.", at + MINUTE, GROUP_CONV),
      delivery(8, CID, "Cid", "I'll bring snacks.", at + 2 * MINUTE, GROUP_CONV),
      delivery(9, OWN, "Erika", "See you at seven.", at + 3 * MINUTE, GROUP_CONV),
      delivery(10, OWN, "Erika", "I'll bring the dice.", at + 4 * MINUTE, GROUP_CONV),
      delivery(11, BEA, "Bea", "Perfect.", at + 20 * MINUTE, GROUP_CONV),
    ],
  };
}

/** A new account: nobody, no group, no message. */
function empty(): World {
  return { contacts: [], groups: { groups: [], invites: [] }, inbox: [] };
}

/** Bea writes to the group now, after the page read its inbox: unread. */
function arrive(held: World, text: string) {
  held.inbox.push(delivery(held.inbox.length + 1, BEA, "Bea", text, Date.now(), GROUP_CONV));
}

// ---- Screens for a person to look at (CHAT_SHOTS=<folder>) ----------------

const SHOTS = process.env["CHAT_SHOTS"];

test("screens of the messenger, both schemes", async ({ page }, info) => {
  test.skip(!SHOTS, "only when CHAT_SHOTS names a folder");
  test.setTimeout(180_000);
  const held = rich();
  await open(page, [], held);
  const shot = (name: string, scheme: string) =>
    page.screenshot({ path: path.join(SHOTS ?? "", `${name}-${info.project.name}-${scheme}.png`) });
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`/en/account/chat?c=${CONV}`);
    await expect(page.locator(".conv-msgs").getByText("Hello Erika")).toBeVisible({ timeout: 15_000 });
    await page.waitForTimeout(600);
    await shot("dm", scheme);
    await page.goto(`/en/account/chat?c=${GROUP_CONV}`);
    await expect(page.locator(".conv-msgs").getByText("Perfect.")).toBeVisible({ timeout: 15_000 });
    await page.waitForTimeout(600);
    await shot("group", scheme);
    await page.goto(`/en/account/chat?c=${CID_CONV}`);
    await expect(page.getByRole("heading", { level: 2, name: "Cid" })).toBeVisible({ timeout: 15_000 });
    await page.waitForTimeout(1_200);
    await shot("dm-empty", scheme);
    await page.goto("/en/account/contacts");
    await expect(page.getByRole("heading", { name: "Requests for you" })).toBeVisible({ timeout: 15_000 });
    await page.waitForTimeout(600);
    await shot("contacts", scheme);
    await page.goto("/en/account/chat");
    // the inbox's first read is through once the list shows its last line: what comes now is new
    await expect(page.locator("#conv-dm-1a2b-2c3d")).toContainText("Are you coming on Tuesday?", { timeout: 15_000 });
    arrive(held, `On my way (${scheme}).`);
    await expect(page.locator(".conv-row[data-unread]")).toBeVisible({ timeout: 15_000 });
    await page.waitForTimeout(300);
    await shot("list", scheme);
    held.failSend = true;
    await page.goto(`/en/account/chat?c=${CONV}`);
    await expect(page.locator(".conv-msgs").getByText("Hello Erika")).toBeVisible({ timeout: 15_000 });
    await page.getByLabel("Write a message").fill("This one stays here");
    await page.getByRole("button", { name: "Send" }).click();
    await expect(page.locator(".msg.pending[data-failed]")).toBeVisible({ timeout: 15_000 });
    await page.waitForTimeout(300);
    await shot("dm-failed", scheme);
    await page.getByRole("button", { name: "Discard" }).click();
    held.failSend = false;
  }
});

test("screens of a new account's messenger, both schemes", async ({ page }, info) => {
  test.skip(!SHOTS, "only when CHAT_SHOTS names a folder");
  test.setTimeout(120_000);
  await open(page, [], empty());
  const shot = (name: string, scheme: string) =>
    page.screenshot({ path: path.join(SHOTS ?? "", `${name}-${info.project.name}-${scheme}.png`) });
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto("/en/account/chat");
    await expect(page.locator(".msgr-tabs")).toBeVisible({ timeout: 15_000 });
    await page.waitForTimeout(2_000);
    await shot("empty-list", scheme);
    await page.goto("/en/account/contacts");
    await expect(page.locator(".msgr-tabs")).toBeVisible({ timeout: 15_000 });
    await page.waitForTimeout(2_000);
    await shot("empty-contacts", scheme);
  }
});

// ---- The look: runs, sides, system lines, empty states ---------------------

/** The colour `token` resolves to on the page, as the browser writes colours. */
function colour(page: Page, token: string): Promise<string> {
  return page.evaluate((name) => {
    const probe = document.createElement("div");
    probe.style.background = `var(${name})`;
    document.body.append(probe);
    const value = getComputedStyle(probe).backgroundColor;
    probe.remove();
    return value;
  }, token);
}

test("a group's lines run together: one face and name per run, one's own on the end side in the gold", async ({ page }, info) => {
  const { problems } = await open(page, [], rich());
  await page.goto(`/en/account/chat?c=${GROUP_CONV}`);
  await expect(page.locator(".conv-msgs").getByText("Perfect.")).toBeVisible({ timeout: 15_000 });
  // yesterday's lines, all on one day: Bea twice, Cid, Erika twice, Bea after a pause
  const day = page.locator(".conv-day").filter({ hasText: "Who brings the cards on Tuesday?" });
  const lines = await day.locator(".msg").evaluateAll((items) =>
    items.map((item) => ({
      text: item.querySelector(".msg-text")?.textContent,
      own: item.hasAttribute("data-own"),
      first: item.hasAttribute("data-first"),
      last: item.hasAttribute("data-last"),
      face: item.querySelectorAll(".msg-face .avatar").length,
      sender: item.querySelector(".msg-sender")?.textContent ?? null,
      heard: item.querySelector(".visually-hidden")?.textContent?.split(",")[0],
      time: item.querySelector("time")?.getAttribute("aria-hidden"),
    })),
  );
  expect(lines).toEqual([
    { text: "Who brings the cards on Tuesday?", own: false, first: true, last: false, face: 1, sender: "Bea", heard: "Bea", time: "true" },
    { text: "I can bring my old deck.", own: false, first: false, last: true, face: 0, sender: null, heard: "Bea", time: "true" },
    { text: "I'll bring snacks.", own: false, first: true, last: true, face: 1, sender: "Cid", heard: "Cid", time: "true" },
    { text: "See you at seven.", own: true, first: true, last: false, face: 0, sender: null, heard: "You", time: "true" },
    { text: "I'll bring the dice.", own: true, first: false, last: true, face: 0, sender: null, heard: "You", time: "true" },
    { text: "Perfect.", own: false, first: true, last: true, face: 1, sender: "Bea", heard: "Bea", time: "true" },
  ]);
  // the name beside the face is for the eye: a screen reader hears it once, before the line
  for (const sender of await day.locator(".msg-sender").all()) await expect(sender).toHaveAttribute("aria-hidden", "true");
  // others' lines start at the start, one's own end at the end, in the gold
  const scroll = await page.locator(".conv-scroll").boundingBox();
  const bubble = (text: string) => day.locator(".msg", { hasText: text }).locator(".msg-bubble");
  const theirs = await bubble("Who brings the cards on Tuesday?").boundingBox();
  const mine = await bubble("See you at seven.").boundingBox();
  expect(scroll && theirs && mine).toBeTruthy();
  if (scroll && theirs && mine) {
    // theirs beside the face's column at the start, mine against the end
    expect(theirs.x - scroll.x).toBeLessThan(80);
    expect(scroll.x + scroll.width - (theirs.x + theirs.width)).toBeGreaterThan(48);
    expect(scroll.x + scroll.width - (mine.x + mine.width)).toBeLessThan(48);
    expect(mine.x - scroll.x).toBeGreaterThan(80);
  }
  const gold = await colour(page, "--gold");
  expect(await bubble("See you at seven.").evaluate((b) => getComputedStyle(b).backgroundColor)).toBe(gold);
  expect(await bubble("Perfect.").evaluate((b) => getComputedStyle(b).backgroundColor)).not.toBe(gold);
  // the list says who wrote a group's last line (the list stands beside it on a desktop)
  if (info.project.name === "desktop") await expect(page.locator(`#conv-grp-${GROUP}`)).toContainText("Bea: Perfect.");
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  expect(problems).toEqual([]);
});

test("a line that could not be sent says so under its bubble, with its two ways on", async ({ page }) => {
  const held = rich();
  held.failSend = true;
  const { problems } = await open(page, [], held);
  await page.goto(`/en/account/chat?c=${CONV}`);
  await expect(page.locator(".conv-msgs").getByText("Hello Erika")).toBeVisible({ timeout: 15_000 });
  const composer = page.getByLabel("Write a message");
  await composer.fill("This one stays here");
  await page.getByRole("button", { name: "Send" }).click();
  const pending = page.locator(".msg.pending[data-failed]");
  await expect(pending).toBeVisible({ timeout: 15_000 });
  await expect(pending.getByRole("alert")).toHaveText("Not sent.");
  await expect(pending.getByText("This one stays here")).toBeVisible();
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  await pending.getByRole("button", { name: "Discard" }).click();
  await expect(page.locator(".msg.pending")).toHaveCount(0);
  // the page's own error lines are expected here: Central answered 503
  expect(problems.filter((p) => !p.includes("503"))).toEqual([]);
});

test("a new account: each empty place says so in a line and shows the way on", async ({ page, context }, info) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  const { problems } = await open(page, [], empty());
  await page.goto("/en/account/chat");
  const none = page.locator(".msgr-empty");
  await expect(none.getByText("It’s quiet here, for now.")).toBeVisible({ timeout: 15_000 });
  await expect(none.getByText("No conversations yet. Add a contact, then write to them.")).toBeVisible();
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  // the way on: the contacts, at the field that adds one
  await none.getByRole("link", { name: "Add a contact" }).click();
  await expect(page).toHaveURL(/\/en\/account\/contacts$/);
  await expect(page.getByLabel("Add a contact by their handle")).toBeFocused();
  await expect(none.getByText("No contacts yet. Find someone by their handle, or give them yours.")).toBeVisible();
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  // or give one's own handle away
  await none.getByRole("button", { name: "Copy your handle" }).click();
  await expect(none.getByRole("status")).toHaveText("Copied.");
  if (info.project.name === "desktop") expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("Erika#1a2b");
  // in German, written by hand, and still whole at 320 px
  await page.goto("/de/account/chat");
  await expect(none.getByText("Noch ist es still hier.")).toBeVisible({ timeout: 15_000 });
  await expect(none.getByRole("link", { name: "Kontakt hinzufügen" })).toBeVisible();
  expect(await narrow(page)).toBe(0);
  expect(problems).toEqual([]);
});

test("a conversation without a message shows who it is with and leads to the composer", async ({ page }) => {
  const { problems } = await open(page, [], rich());
  await page.goto(`/en/account/chat?c=${CID_CONV}`);
  const start = page.locator(".conv-start");
  await expect(start.getByText("Say hello.")).toBeVisible({ timeout: 15_000 });
  await expect(start.locator(".avatar")).toHaveAttribute("aria-hidden", "true");
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  await start.getByRole("button", { name: "Write the first message" }).click();
  await expect(page.getByLabel("Write a message")).toBeFocused();
  expect(problems).toEqual([]);
});

test("the list says it loads, or that it failed, before it ever says it is empty", async ({ page }) => {
  const held = empty();
  held.contactsWait = 1_500;
  held.contactsFail = 1;
  const { problems } = await open(page, [], held);
  await page.goto("/en/account/chat");
  await expect(page.locator(".msgr-panel").getByRole("status").filter({ hasText: "Loading" })).toBeVisible({ timeout: 15_000 });
  await expect(page.locator(".msgr-empty")).toHaveCount(0);
  // the contacts could not be read: said aloud, with a way to try again, and no empty list
  const failed = page.locator(".msgr-panel").getByRole("alert").filter({ hasText: "The chat could not reach Central just now." });
  await expect(failed).toBeVisible({ timeout: 15_000 });
  await expect(page.locator(".msgr-empty")).toHaveCount(0);
  expect(await schemes(page)).toEqual([]);
  await failed.getByRole("button", { name: "Try again" }).click();
  await expect(page.locator(".msgr-empty").getByText("It’s quiet here, for now.")).toBeVisible({ timeout: 15_000 });
  await expect(failed).toHaveCount(0);
  expect(problems.filter((p) => !p.includes("503"))).toEqual([]);
});

test("contacts: requests and invitations as cards, one gold answer each, names with their tags as typed", async ({ page }) => {
  const { problems } = await open(page, [], rich());
  await page.goto("/en/account/contacts");
  const requests = page.locator(".people[data-relation='incoming']");
  await expect(requests.getByText("Dora#4a5b")).toBeVisible({ timeout: 15_000 });
  // the tag as it is typed, never upper case
  const tag = requests.locator(".tag").first();
  await expect(tag).toHaveText("#4a5b");
  expect(await tag.evaluate((t) => getComputedStyle(t).textTransform)).toBe("none");
  // in the lists, the gold is only for the answer a request waits for
  const gold = page.locator(".people .button:not(.quiet):not(.danger)");
  expect(await gold.allTextContents()).toEqual(["Accept"]);
  await expect(page.locator(".people[data-relation='friend']").getByText("Bea#2c3d")).toBeVisible();
  await expect(page.locator(".people[data-relation='outgoing']").getByText("Finnegan-Whitley#5c6d")).toBeVisible();
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  // the invitation in the Chats half: a card with its answer in the gold
  await page.goto("/en/account/chat");
  const invited = page.locator(".people[data-relation='invited']");
  await expect(invited.getByText("Friday drafts")).toBeVisible({ timeout: 15_000 });
  await expect(invited.getByText("from Bea#2c3d")).toBeVisible();
  expect(await invited.locator(".button:not(.quiet):not(.danger)").allTextContents()).toEqual(["Accept"]);
  expect(await schemes(page)).toEqual([]);
  expect(problems).toEqual([]);
});

test("a line arriving marks its row and leaves the rest of the list, and its focus, as they were", async ({ page }) => {
  const held = rich();
  const { problems } = await open(page, [], held);
  await page.goto("/en/account/chat");
  const bea = page.locator("#conv-dm-1a2b-2c3d");
  // the inbox's first read is through once the row shows its last line: what comes now is new
  await expect(bea).toContainText("Are you coming on Tuesday?", { timeout: 15_000 });
  await bea.focus();
  await bea.evaluate((row) => row.setAttribute("data-probe", "kept"));
  arrive(held, "Running late.");
  const group = page.locator(`#conv-grp-${GROUP}`);
  await expect(group).toHaveAttribute("data-unread", "true", { timeout: 15_000 });
  await expect(group).toContainText("Bea: Running late.");
  // the same element, still focused: the list was not drawn anew
  await expect(bea).toBeFocused();
  await expect(bea).toHaveAttribute("data-probe", "kept");
  // the marked row, measured
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(problems).toEqual([]);
});

// ---- The keyboard: the same stops in the same order -----------------------

/**
 * The Tab stops inside the messenger from where the focus is: role (or
 * tag), id, and name; a conversation's row by its id alone, as its words
 * carry the time.
 */
async function stops(page: Page): Promise<string[]> {
  const found: string[] = [];
  for (let i = 0; i < 40; i++) {
    const stop = await page.evaluate(() => {
      const e = document.activeElement as HTMLElement | null;
      if (!e || !e.closest(".messenger")) return null;
      const role = e.getAttribute("role") ?? e.tagName.toLowerCase();
      const id = e.id ? `#${e.id}` : "";
      const name = (e.getAttribute("aria-label") ?? e.textContent ?? "").trim().replace(/\s+/g, " ");
      return e.classList.contains("conv-row") || !name ? `${role}${id}` : `${role}${id} "${name}"`;
    });
    if (!stop) break;
    found.push(stop);
    await page.keyboard.press("Tab");
  }
  return found;
}

const TOP = ['a "Chats"', 'a "Contacts1 requests"'];
const LIST = [...TOP, "input#chat-search", 'button "New group"', 'button "Accept"', 'button "Decline"', "a#conv-dm-1a2b-2c3d"];
const OPEN = ['button#conv-details-toggle "Details"', 'region "Messages"', "textarea#composer-text", 'button "Send"'];
const CONTACTS = [
  ...TOP, "input#contact-find", 'button "Find"',
  'button "Accept"', 'button "Decline"', 'button "Block"',
  'a "Write"', 'button "Remove"', 'button "Block"',
  'a "Write"', 'button "Remove"', 'button "Block"',
  'button "Withdraw"', 'button "Unblock"',
];

test("the keyboard meets the same stops in the same order", async ({ page }, info) => {
  const { problems } = await open(page, [], rich());
  const phone = info.project.name === "phone";
  await page.goto("/en/account/chat");
  await expect(page.locator(".conv-row").first()).toBeVisible({ timeout: 15_000 });
  await page.locator(".msgr-tabs a").first().focus();
  expect(await stops(page)).toEqual(LIST);
  await page.goto(`/en/account/chat?c=${CONV}`);
  await expect(page.locator(".conv-msgs").getByText("Hello Erika")).toBeVisible({ timeout: 15_000 });
  if (phone) {
    // one pane at a time: the conversation from its heading, the way back before it
    await expect(page.locator("#conv-title")).toBeFocused();
    expect(await stops(page)).toEqual(['h2#conv-title "Bea#2c3d"', ...OPEN]);
    await page.locator("#conv-title").focus();
    await page.keyboard.press("Shift+Tab");
    await expect(page.getByRole("button", { name: "Back to the conversations" })).toBeFocused();
  } else {
    // both panes: the list, then the conversation; no way back to show
    await page.locator(".msgr-tabs a").first().focus();
    expect(await stops(page)).toEqual([...LIST, ...OPEN]);
    await expect(page.getByRole("button", { name: "Back to the conversations" })).toBeHidden();
  }
  await page.goto("/en/account/contacts");
  await expect(page.locator(".person").first()).toBeVisible({ timeout: 15_000 });
  await page.locator(".msgr-tabs a").first().focus();
  expect(await stops(page)).toEqual(CONTACTS);
  expect(problems).toEqual([]);
});

// ---- What the other tests do not show ------------------------------------

/** A button's own padding at its start, in px. */
function padding(button: Locator): Promise<number> {
  return button.evaluate((b) => parseFloat(getComputedStyle(b).paddingInlineStart));
}

/** With CHAT_SHOTS, `name` in the light and the dark scheme; the page is left light. */
async function shoot(page: Page, project: string, name: string) {
  if (!SHOTS) return;
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
    await page.screenshot({ path: path.join(SHOTS, `${name}-${project}-${scheme}.png`) });
  }
  await page.emulateMedia({ colorScheme: "light", reducedMotion: null });
}

test("a person found by handle, and a group's details as its owner", async ({ page }, info) => {
  const held = rich();
  held.people = [person(0x7a8b, "Ella")];
  // a friend outside the group, so its owner has someone to invite
  held.contacts.push({ person: person(0x8c9d, "Hana"), relation: "friend" });
  held.groups.groups[0]?.invited.push(person(0x5c6d, "Finnegan-Whitley"));
  const { problems } = await open(page, [], held);

  await page.goto("/en/account/contacts");
  await page.getByLabel("Add a contact by their handle").fill("Ella#7a8b");
  await page.getByRole("button", { name: "Find", exact: true }).click();
  const found = page.locator(".person.found");
  await expect(found.getByText("Ella#7a8b")).toBeVisible({ timeout: 15_000 });
  const request = found.getByRole("button", { name: "Send a contact request" });
  // under the name, with the button's own padding
  expect(await padding(request)).toBeGreaterThan(8);
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  await shoot(page, info.project.name, "found");

  await page.goto(`/en/account/chat?c=${GROUP_CONV}`);
  await expect(page.locator(".conv-msgs").getByText("Perfect.")).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { name: "Details" }).click();
  const details = page.locator("#conv-details");
  await expect(details.getByLabel("Group name")).toHaveValue("Tuesday table");
  await expect(details.getByRole("button", { name: "Make owner" })).toHaveCount(2);
  await expect(details.getByLabel("Invite a contact")).toBeVisible();
  const withdraw = details.getByRole("button", { name: "Withdraw" });
  await expect(withdraw).toBeVisible();
  expect(await padding(withdraw)).toBeGreaterThan(8);
  await expect(details.getByRole("button", { name: "Delete group" })).toBeVisible();
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
  await shoot(page, info.project.name, "group-details");
  expect(problems).toEqual([]);
});

test("a chat that could not be opened lets the empty list say so, not that it loads", async ({ page }) => {
  const { problems } = await open(page, [], empty(), false);
  await page.goto("/en/account/chat");
  await expect(page.locator(".msgr-empty").getByText("It’s quiet here, for now.")).toBeVisible({ timeout: 15_000 });
  await expect(page.locator(".msgr-panel").getByRole("status").filter({ hasText: "Loading" })).toHaveCount(0);
  await expect(page.locator(".msgr-welcome .error")).toHaveText("The chat could not be opened. Reload the page to try again.");
  expect(problems).toEqual([]);
});
