// The Impressum's details behind their puzzle (crates/central-web/src/
// islands/impressum.rs; Baylee's docs/feedback.md, "The Impressum"). The
// service that hands them out is Baylee's and does not run here, so this
// suite answers for it with made-up details: a puzzle the page must really
// solve and a blob as Baylee's tool writes one. What is held here is what
// keeps address collectors out and people in: the bait for robots, the three
// yes-or-no answers about the browser, no mail or telephone address anywhere
// in the page's markup, and the details exact for whoever sees, hears,
// selects or copies them.
import { createHash } from "node:crypto";

import { expect, type Page, test } from "@playwright/test";

import { axe, targets, watch } from "./check.ts";

const DETAILS = {
  name: "Erika Mustermann",
  street: "Musterweg 1",
  postcode: "12345",
  city: "Musterstadt",
  country_de: "Deutschland",
  country_en: "Germany",
  email: "erika@baylee-e2e.invalid",
  phone: "+49 30 1234567",
};
const SCRIPT_KEY = Buffer.from("6b1f0c9e52d84a37b0e1f6a2c9d35e48", "hex");
const CHALLENGE = { nonce: "e2e-nonce_1", parts: 3, difficulty: 6, expires: 4102444800, sig: "made-up" };

/** A blob as `baylee-tools site impressum-blob` writes one. */
function blob(details: Record<string, string>): string {
  const fields: Record<string, number[][]> = {};
  for (const [name, text] of Object.entries(details)) {
    fields[name] = [...text].map((char, position) => [position, char.codePointAt(0) ?? 0]).reverse();
  }
  const nonce = Buffer.alloc(16, 7);
  const body = Buffer.from(JSON.stringify({ v: 2, fields }), "utf8");
  for (let block = 0; block * 32 < body.length; block++) {
    const counter = Buffer.alloc(4);
    counter.writeUInt32BE(block);
    const key = createHash("sha256").update(SCRIPT_KEY).update(nonce).update(counter).digest();
    for (let i = 0; i < 32 && block * 32 + i < body.length; i++) body[block * 32 + i] ^= key[i];
  }
  return Buffer.concat([Buffer.from("BYIM"), Buffer.from([2]), nonce, body]).toString("base64url");
}

/** Whether `counter` solves `part`, as the service checks it. */
function solves(part: number, counter: number): boolean {
  const hash = createHash("sha256").update(`${CHALLENGE.nonce}:${part}:${counter}`).digest();
  return hash.readUInt32BE(0) >>> (32 - CHALLENGE.difficulty) === 0;
}

/** Answers as the service would; what the page asked and sent. */
async function service(page: Page): Promise<{ asked: string[]; solved: boolean[] }> {
  const seen = { asked: [] as string[], solved: [] as boolean[] };
  await page.route("**/site/impressum/challenge**", (route) => {
    seen.asked.push(new URL(route.request().url()).search);
    return route.fulfill({ json: CHALLENGE });
  });
  await page.route("**/site/impressum", (route) => {
    const answer = route.request().postDataJSON() as typeof CHALLENGE & { counters: number[] };
    const solved =
      route.request().method() === "POST" &&
      answer.nonce === CHALLENGE.nonce &&
      answer.sig === CHALLENGE.sig &&
      answer.expires === CHALLENGE.expires &&
      answer.counters.length === CHALLENGE.parts &&
      answer.counters.every((counter, part) => solves(part, counter));
    seen.solved.push(solved);
    return solved
      ? route.fulfill({ contentType: "text/plain; charset=us-ascii", body: blob(DETAILS) })
      : route.fulfill({ status: 400, json: { error: "not solved" } });
  });
  return seen;
}

test("the bait for robots is in the page, and no person sees, hears or reaches it", async ({ page }) => {
  await page.goto("/de/impressum");
  const bait = page.locator("a.bait");
  await expect(bait).toHaveCount(1);
  await expect(bait).toHaveAttribute("href", "/site/impressum/kontakt");
  await expect(bait).toHaveAttribute("rel", "nofollow");
  await expect(bait).toBeHidden();
  expect(await bait.evaluate((link) => getComputedStyle(link).display)).toBe("none");
  // not a link for a screen reader, not a stop for the Tab key
  await expect(page.getByRole("link", { name: "Kontakt", exact: true, includeHidden: false }).and(bait)).toHaveCount(0);
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  const stops: string[] = [];
  for (let i = 0; i < 60; i++) {
    await page.keyboard.press("Tab");
    stops.push(await page.evaluate(() => document.activeElement?.getAttribute("href") ?? ""));
  }
  expect(stops).not.toContain("/site/impressum/kontakt");
  expect((await targets(page)).filter((target) => target.includes("Kontakt\" 0"))).toEqual([]);
});

for (const lang of ["de", "en"] as const) {
  test(`${lang}: the solved puzzle shows the details as a person reads them, and no address a collector could take`, async ({
    page,
    baseURL,
  }) => {
    const problems = watch(page, baseURL ?? "");
    const seen = await service(page);
    await page.goto(`/${lang}/impressum`);
    await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
    await page.getByRole("button", { name: lang === "de" ? "Angaben anzeigen" : "Show the details" }).click();
    const details = page.locator("dl.impressum-details");
    await expect(details).toBeFocused();

    // the page solved the puzzle it was given and said what it knows of its browser
    expect(seen.solved).toEqual([true]);
    expect(seen.asked).toHaveLength(1);
    const bits = Number(/^\?s=(\d+)$/.exec(seen.asked[0])?.[1] ?? "-1");
    expect(bits).toBeGreaterThanOrEqual(0);
    expect(bits).toBeLessThanOrEqual(7);
    // a browser a program drives says so (this one is)
    expect(bits & 1).toBe(1);

    // as a person sees them, and as a selection copies them
    const values = details.locator("dd");
    await expect(values).toHaveCount(4);
    expect(await values.nth(0).innerText()).toBe(DETAILS.name);
    expect((await values.nth(1).innerText()).split("\n")).toEqual([
      DETAILS.street,
      `${DETAILS.postcode} ${DETAILS.city}`,
      lang === "de" ? DETAILS.country_de : DETAILS.country_en,
    ]);
    const mail = values.nth(2).locator(".impressum-value");
    const phone = values.nth(3).locator(".impressum-value");
    expect(await mail.innerText()).toBe(DETAILS.email);
    expect(await phone.innerText()).toBe(DETAILS.phone);
    const selected = await details.evaluate((list) => {
      const selection = getSelection();
      selection?.selectAllChildren(list);
      return selection?.toString() ?? "";
    });
    expect(selected).toContain(DETAILS.email);
    expect(selected).toContain(DETAILS.phone);
    // as a screen reader is told them: nothing of what is hidden
    const heard = await details.ariaSnapshot();
    expect(heard).toContain("erika@");
    expect(heard).toContain("baylee-e2e.invalid");
    expect(heard).not.toContain("invalid.example");

    // as a collector reads a page: its markup, and its text with what is hidden
    const markup = await page.content();
    const text = (await page.locator("body").evaluate((body) => body.textContent)) ?? "";
    for (const read of [markup, text]) {
      expect(read).not.toContain(DETAILS.email);
      expect(read).not.toContain(DETAILS.phone);
      expect(read).not.toContain("mailto:");
      expect(read).not.toContain("tel:");
    }
    expect(text).toContain("erika@invalid.example");
    await expect(page.locator('a[href^="mailto:"], a[href^="tel:"]')).toHaveCount(0);

    // the shown state is as accessible as the rest
    expect(await axe(page)).toEqual([]);
    expect(await targets(page)).toEqual([]);
    for (const name of lang === "de" ? ["E-Mail schreiben", "Anrufen"] : ["Write an e-mail", "Call"]) {
      await expect(details.getByRole("button", { name, exact: true })).toBeVisible();
    }
    expect(problems).toEqual([]);
  });
}

test("the buttons build the mail and telephone addresses only when pressed", async ({ page, browserName }) => {
  test.skip(browserName !== "chromium", "the navigation is watched through Chromium's protocol");
  await service(page);
  await page.goto("/en/impressum");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByRole("button", { name: "Show the details" }).click();
  const details = page.locator("dl.impressum-details");
  await expect(details).toBeFocused();
  const asked: string[] = [];
  const protocol = await page.context().newCDPSession(page);
  await protocol.send("Page.enable");
  protocol.on("Page.frameRequestedNavigation", (event) => asked.push(event.url));
  await details.getByRole("button", { name: "Write an e-mail" }).click();
  await expect.poll(() => asked).toContain(`mailto:${DETAILS.email}`);
  await details.getByRole("button", { name: "Call" }).click();
  await expect.poll(() => asked).toContain("tel:+49301234567");
  // the page stayed, with the details
  await expect(details).toBeVisible();
});

test("a puzzle that expired on the way is asked for once more", async ({ page }) => {
  const seen = await service(page);
  let first = true;
  await page.route("**/site/impressum", (route) => {
    if (!first) return route.fallback();
    first = false;
    return route.fulfill({ status: 410, json: { error: "the challenge expired" } });
  });
  await page.goto("/en/impressum");
  await expect(page.locator("html")).toHaveAttribute("data-islands", "ready");
  await page.getByRole("button", { name: "Show the details" }).click();
  await expect(page.locator("dl.impressum-details")).toBeFocused();
  expect(seen.asked).toHaveLength(2);
  expect(seen.solved).toEqual([true]);
});
