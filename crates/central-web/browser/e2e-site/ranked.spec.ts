import { readFileSync } from "node:fs";

import { expect, type Page, type Route, test } from "@playwright/test";

import { narrow, schemes, targets } from "./check.ts";

// The leaderboards (crates/central-web/src/islands/ranked/, docs/website-design.md
// §"The leaderboards") in a real browser, against a stand-in for Central's
// public ranked reads. Every row is synthetic, built from ranked.json, whose
// shapes the island's Rust tests read with central-api's own types.

interface Rank {
  league: string;
  division: number | null;
  champion_place: number | null;
}
interface Row {
  account: number[];
  name: string | null;
  tag: number | null;
  standing: { format: string; track: string; deck: number[] | null; elo: number; rank: Rank };
  ai?: { kind: string; label: string };
}
interface Season {
  id: number[];
  name: string;
  starts: number;
  ends: number | null;
}
interface Fixture {
  season: Season;
  older: Season;
  person: Row;
  ai: Row;
  deck: Row;
  placing: Row;
}
interface BoardAsk {
  season: number[] | null;
  format: string;
  track: string;
  league: string;
  division: number | null;
  after: { elo: number; account: number[]; deck: number[] | null } | null;
  players?: string;
}
interface Body {
  operation: string;
  request: { board?: BoardAsk; seasons?: { before: number[] | null } };
}

const FIXTURE: Fixture = JSON.parse(readFileSync(new URL("./ranked.json", import.meta.url), "utf8"));

/** The boards the stand-in holds: league → rows, best first. */
type Boards = Record<string, Row[]>;

/** `count` rows of the template `kind` in `league`, Elo falling. */
function rows(kind: "person" | "ai" | "deck", league: string, count: number, from = 0): Row[] {
  const template = FIXTURE[kind];
  return Array.from({ length: count }, (_, at) => {
    const n = from + at;
    const row = structuredClone(template);
    row.account = [...template.account.slice(0, 14), 0x10 + (n >> 8), n & 0xff];
    row.name = kind === "ai" ? `KI-Haus-${n}` : `Synth${n}`;
    row.tag = 0x0100 + n;
    // people and AI players interleave by Elo, as on a real board
    row.standing.elo = kind === "ai" ? 2597 - at * 14 : 2600 - at * 7;
    row.standing.rank = { league, division: null, champion_place: null };
    if (kind === "ai") row.ai = { kind: n % 3 === 0 ? "model" : "house", label: n % 3 === 0 ? "Sprachmodell" : "Haus-KI · Expert" };
    return row;
  });
}

/** People and AI players mixed, as Central lists them by Elo. */
function mixed(league: string, count: number): Row[] {
  const people = rows("person", league, count - Math.floor(count / 3));
  const ais = rows("ai", league, Math.floor(count / 3), 1000);
  return [...people, ...ais].sort((a, b) => b.standing.elo - a.standing.elo);
}

interface Stand {
  /** Every board question, as `league[:division]`. */
  asked: string[];
  bodies: Body[];
  boards: Boards;
  /** The status the next answers get instead of a board, while it is set. */
  refuse?: number;
  /** No season runs: a board of the running season is refused. */
  seasonless?: boolean;
}

async function central(page: Page, boards: Boards): Promise<Stand> {
  const stand: Stand = { asked: [], bodies: [], boards };
  await page.route("**/api/v1", (route: Route) => {
    const request = route.request();
    expect(request.headers()["x-baylee-central"]).toBe("1");
    const body: Body = JSON.parse(request.postData() ?? "{}");
    stand.bodies.push(body);
    const json = (value: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(value) });
    if (body.operation !== "ranked") return route.fulfill({ status: 401, body: "account request rejected\n" });
    if (body.request.seasons) {
      return json({ result: "ranked", response: { seasons: { items: [FIXTURE.season, FIXTURE.older], next: null } } });
    }
    const board = body.request.board;
    if (!board) return route.fulfill({ status: 401, body: "account request rejected\n" });
    stand.asked.push(board.division ? `${board.league}:${board.division}` : board.league);
    if (stand.refuse) return route.fulfill({ status: stand.refuse, body: "busy\n" });
    if (stand.seasonless && board.season === null) return route.fulfill({ status: 401, body: "account request rejected\n" });
    const wanted = (row: Row) =>
      row.standing.format === board.format &&
      row.standing.track === board.track &&
      (board.players === undefined || (board.players === "ai") === Boolean(row.ai));
    const all = (stand.boards[board.league] ?? []).filter(wanted);
    // the keyset cursor: everything after the row the cursor names
    const cursor = board.after;
    const start = cursor ? all.findIndex((row) => row.account.join() === cursor.account.join()) + 1 : 0;
    const entries = all.slice(start, start + 40);
    const last = entries.at(-1);
    const next = last && start + 40 < all.length ? { elo: last.standing.elo, account: last.account, deck: last.standing.deck } : null;
    const season = board.season === null ? FIXTURE.season : FIXTURE.older;
    return json({ result: "ranked", response: { board: { season, entries, next } } });
  });
  return stand;
}

/** Console errors, except the stand-in's refusals, which are answers the page reads. */
function errors(page: Page): string[] {
  const found: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error" && !/status of (401|429)/.test(message.text())) found.push(message.text());
  });
  page.on("pageerror", (error) => found.push(error.message));
  return found;
}

const board = (page: Page) => page.locator("section.ladder");
const title = (page: Page) => page.locator("#ladder-title");
const listed = (page: Page) => page.locator("ol.ladder-rows > li.ladder-row:not(.is-skeleton)");

/** Where the bar stands on the page (not in the window, which may have scrolled). */
const barPlace = (page: Page) =>
  page.locator(".ladder-tools").evaluate((bar) => {
    const box = bar.getBoundingClientRect();
    return { top: box.top + window.scrollY, left: box.left, width: box.width, height: box.height };
  });

test("the page shows the highest league anybody stands in, AI players marked", async ({ page, baseURL }) => {
  const problems = errors(page);
  page.on("request", (request) => {
    if (!request.url().startsWith(baseURL ?? "")) problems.push(`elsewhere: ${request.url()}`);
  });
  const master = mixed("master", 45);
  const stand = await central(page, { master, gold: rows("person", "gold", 3) });
  await page.goto("/en/ranked");
  await expect(title(page)).toHaveText("Master");
  await expect(listed(page)).toHaveCount(40);
  // asked from the top down, and no further than the first league with anybody
  expect(stand.asked).toEqual(["champion", "grandmaster", "master"]);
  await expect(board(page)).toHaveAttribute("aria-busy", "false");
  const ai = listed(page).filter({ has: page.locator(".ai-badge") });
  await expect(ai).toHaveCount(master.slice(0, 40).filter((row) => row.ai).length);
  await expect(ai.first().locator(".ai-badge")).toContainText(/Haus-KI · Expert|Sprachmodell/);
  // a person's row carries no AI badge, and every name leads to its profile
  const person = listed(page).filter({ hasNot: page.locator(".ai-badge") }).first();
  await expect(person.locator("a.handle-link")).toHaveAttribute("href", /^\/en\/player\/Synth\d+-0[0-9a-f]{3}$/);
  await expect(page.locator(".ladder-place.is-podium")).toHaveCount(3);
  await expect(page.locator(".ladder-count")).toHaveText("Page 1 · 40 places");
  await expect(page.locator("#ladder-league")).toHaveValue("");
  await expect(page.locator("#ladder-season option")).toHaveCount(3);
  expect(problems).toEqual([]);
});

test("the pages turn without moving the bar, and the title takes the focus", async ({ page }) => {
  await central(page, { master: mixed("master", 45) });
  await page.goto("/en/ranked");
  await expect(listed(page)).toHaveCount(40);
  const before = page.getByRole("button", { name: "Previous page" });
  const after = page.getByRole("button", { name: "Next page" });
  await expect(before).toHaveAttribute("aria-disabled", "true");
  const bar = await barPlace(page);
  await after.click();
  await expect(listed(page)).toHaveCount(5);
  await expect(page.locator(".ladder-count")).toHaveText("Page 2 · 5 places");
  await expect(listed(page).first().locator(".ladder-place")).toHaveAttribute("data-place", "41");
  await expect(title(page)).toBeFocused();
  await expect(after).toHaveAttribute("aria-disabled", "true");
  await expect(before).toHaveAttribute("aria-disabled", "false");
  expect(await barPlace(page)).toEqual(bar);
  await before.click();
  await expect(listed(page)).toHaveCount(40);
  await expect(page.locator(".ladder-count")).toHaveText("Page 1 · 40 places");
});

test("a choice stands in the address and the bar keeps its size for every choice", async ({ page }) => {
  const stand = await central(page, { master: mixed("master", 12), gold: [...rows("person", "gold", 2), ...rows("ai", "gold", 2, 50)] });
  await page.goto("/en/ranked");
  await expect(listed(page)).toHaveCount(12);
  const size = await barPlace(page);
  await page.getByRole("button", { name: "Highlander" }).click();
  await expect(page).toHaveURL(/\/en\/ranked\?format=highlander$/);
  await page.getByRole("button", { name: "AI players" }).click();
  await page.locator("#ladder-league").selectOption("gold-2");
  await expect(page).toHaveURL(/\?format=highlander&who=ai&league=gold-2$/);
  expect(await barPlace(page)).toEqual(size);
  const last = stand.bodies.at(-1)?.request.board;
  expect(last).toMatchObject({ format: "highlander", track: "player", league: "gold", division: 2, players: "ai" });
  // the address brings the same board back
  stand.asked.length = 0;
  await page.reload();
  await expect(page.getByRole("button", { name: "Highlander" })).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByRole("button", { name: "AI players" })).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#ladder-league")).toHaveValue("gold-2");
  expect(stand.asked).toEqual(["gold:2"]);
});

test("an empty group says so and leads to the highest league with standings", async ({ page }) => {
  await central(page, { master: mixed("master", 6) });
  await page.goto("/en/ranked?league=diamond-4");
  const quiet = page.locator(".ladder-quiet");
  await expect(quiet).toContainText("No standings in this group yet.");
  const height = (await page.locator(".ladder-body").boundingBox())?.height ?? 0;
  await quiet.getByRole("button", { name: "Show the highest league with standings" }).click();
  await expect(title(page)).toHaveText("Master");
  await expect(page).toHaveURL(/\/en\/ranked$/);
  // the rows take the place the empty board kept
  expect((await page.locator(".ladder-body").boundingBox())?.height).toBe(height);
  expect(await schemes(page)).toEqual([]);
});

test("nobody anywhere, no season, too many requests and a failure each say their own words", async ({ page }) => {
  const stand = await central(page, {});
  await page.goto("/en/ranked");
  await expect(page.locator(".ladder-quiet")).toContainText("Nobody has a standing in Standard this season yet.");
  await expect(page.locator(".ladder-quiet a[href='/en/download']")).toBeVisible();
  expect(stand.asked).toHaveLength(12);
  stand.seasonless = true;
  await page.getByRole("button", { name: "Refresh" }).click();
  await expect(page.locator(".ladder-quiet")).toContainText("No season is running right now.");
  await expect(page.locator(".ladder-quiet")).toContainText("Earlier seasons keep their leaderboards");
  stand.seasonless = false;
  stand.refuse = 429;
  await page.getByRole("button", { name: "Refresh" }).click();
  await expect(page.locator(".ladder-quiet[data-state='busy']")).toContainText("Too many requests just now.");
  stand.refuse = 503;
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(page.locator(".ladder-quiet[data-state='failed']")).toContainText("Central could not load this leaderboard.");
  stand.refuse = undefined;
  stand.boards.master = mixed("master", 3);
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(listed(page)).toHaveCount(3);
});

test("an earlier season and the deck ladder read as Central lists them", async ({ page }) => {
  const stand = await central(page, { gold: rows("deck", "gold", 4) });
  await page.goto("/en/ranked?track=deck");
  await expect(title(page)).toHaveText("Gold");
  await expect(listed(page).first()).toContainText("Deck AB12CD34");
  const older = FIXTURE.older.id.map((b: number) => b.toString(16).padStart(2, "0")).join("");
  await page.locator("#ladder-season").selectOption(older);
  await expect(page).toHaveURL(new RegExp(`track=deck&season=${older}$`));
  expect(stand.bodies.at(-1)?.request.board?.season).toEqual(FIXTURE.older.id);
  await expect(page.locator(".ladder-meta")).toContainText("Prolog");
});

test("the board passes WCAG 2.2 AA in both schemes, with 44 px targets and no sideways scroll", async ({ page }) => {
  await central(page, { master: mixed("master", 45) });
  await page.goto("/en/ranked");
  await expect(listed(page)).toHaveCount(40);
  expect(await schemes(page)).toEqual([]);
  expect(await targets(page)).toEqual([]);
  expect(await narrow(page)).toBe(0);
});
