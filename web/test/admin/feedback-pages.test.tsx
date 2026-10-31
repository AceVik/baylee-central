/// <reference types="node" />
// The feedback section with stand-ins (test/support/reports.ts): the views
// with their counts, the search in the address, a report triaged on its page
// with its revision named and read again on a conflict, notes removed by
// their author only, the unread count in the navigation, and the overview's
// tile.

import { gzipSync } from "node:zlib";

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { idText } from "../../src/account/owner-api";
import { AccountProvider, SESSION_KEY } from "../../src/account/session";
import type { AdminApi } from "../../src/admin/admin-api";
import FeedbackPage from "../../src/admin/feedback/FeedbackPage";
import ReportPage from "../../src/admin/feedback/ReportPage";
import OverviewPage from "../../src/admin/OverviewPage";
import { LangProvider } from "../../src/i18n";
import { ACCOUNT, fakeCentral, fakeCrypto } from "../support/central";
import { standInDialogs } from "../support/dialog";
import { fakeReports, id } from "../support/reports";
import { idOf } from "../../src/account/owner-api";

standInDialogs();

function show(node: ReactNode) {
  const api = fakeCentral();
  api.accounts.set("synthetic_admin", { identity: { account: ACCOUNT, epoch: 1 }, wrapped: new Uint8Array(73), recovery: new Uint8Array(73), displayName: "Synthetic_Admin", tag: 0x1000 });
  api.session = { name: "synthetic_admin", expiresAt: Math.floor(Date.now() / 1000) + 600 };
  const summary = api.accountSummary.bind(api);
  api.accountSummary = async () => ({ ...(await summary()), admin: true });
  window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: api.session.expiresAt }));
  return render(
    <LangProvider initial="en">
      <AccountProvider api={api} crypto={fakeCrypto("unused").factory}>
        {node}
      </AccountProvider>
    </LangProvider>,
  );
}

/** A request these pages never make. */
const no = async () => Promise.reject(new Error("not asked here"));

function admin(): AdminApi {
  return {
    accounts: vi.fn<AdminApi["accounts"]>(no),
    account: vi.fn<AdminApi["account"]>(no),
    audit: vi.fn<AdminApi["audit"]>(no),
    gateways: vi.fn<AdminApi["gateways"]>(no),
    gateway: vi.fn<AdminApi["gateway"]>(no),
    setAdmin: vi.fn<AdminApi["setAdmin"]>(no),
    resetLogin: vi.fn<AdminApi["resetLogin"]>(no),
    setTag: vi.fn<AdminApi["setTag"]>(no),
    setGatewayApproval: vi.fn<AdminApi["setGatewayApproval"]>(no),
    revokeGatewayToken: vi.fn<AdminApi["revokeGatewayToken"]>(no),
    legacyAccounts: vi.fn<AdminApi["legacyAccounts"]>(no),
    overview: vi.fn<AdminApi["overview"]>(async () => ({ accounts: 12, accountsWeek: 3, moved: 4, admins: 2, pending: 0, approved: 5, disabled: 1, online: 4, liveTokens: 7 })),
    reportsUnread: vi.fn<AdminApi["reportsUnread"]>(async () => 1),
    reportsAtGlance: vi.fn<AdminApi["reportsAtGlance"]>(async () => ({ open: 3, newCrashes: 2 })),
  };
}

beforeEach(() => {
  window.localStorage.clear();
  window.history.replaceState(null, "", "/");
});

describe("the feedback section", () => {
  it("lists a view's reports, the unread marked, every view counted, and the address follows a view", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    window.history.replaceState(null, "", "/admin/feedback?status=new");
    show(<FeedbackPage client={admin()} reports={reports} />);
    expect(await screen.findByRole("link", { name: /Synthetic report 1/ })).toBeTruthy();
    expect(screen.getByRole("link", { name: /Synthetic report 2/ })).toBeTruthy();
    expect(screen.queryByRole("link", { name: /Synthetic report 3/ })).toBeNull();
    // report 1 is new and unopened: marked in words too
    expect(screen.getAllByText("Unread:", { exact: false }).length).toBeGreaterThan(0);
    expect(await screen.findByRole("button", { name: "Inbox 2" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Inbox 2" }).getAttribute("aria-pressed")).toBe("true");
    expect(await screen.findByRole("button", { name: "Mine 1" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Open 3" }));
    expect(window.location.search).toBe("?status=new%2Ctriaged%2Cin_progress");
    expect(await screen.findByRole("link", { name: /Synthetic report 3/ })).toBeTruthy();
    // the navigation counts the unread ones
    expect(screen.getByRole("link", { name: /^Feedback 1 unread/ })).toBeTruthy();
  });

  it("follows the search once typing pauses and keeps it in the address", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    window.history.replaceState(null, "", "/admin/feedback");
    show(<FeedbackPage client={admin()} reports={reports} />);
    await screen.findByRole("link", { name: /Synthetic report 4/ });
    await user.type(screen.getByRole("searchbox", { name: "Search reports" }), "report 2");
    await waitFor(() => expect(reports.reports).toHaveBeenLastCalledWith(expect.objectContaining({ filter: { q: "report 2" } })));
    expect(window.location.search).toBe("?q=report+2");
    expect(await screen.findByRole("button", { name: /“report 2”/ })).toBeTruthy();
    await waitFor(() => expect(screen.queryByRole("link", { name: /Synthetic report 4/ })).toBeNull());
  });

  it("opens a report beside the list and steps through it with j and k", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    window.history.replaceState(null, "", "/admin/feedback?status=new,triaged,in_progress");
    show(<FeedbackPage client={admin()} reports={reports} />);
    await user.click(await screen.findByRole("link", { name: /Synthetic report 3/ }));
    const drawer = await screen.findByRole("complementary", { name: "The report beside the list" });
    expect(await within(drawer).findByRole("heading", { name: /Synthetic report 3/ })).toBeTruthy();
    await user.keyboard("j");
    expect(await within(drawer).findByRole("heading", { name: /Synthetic report 2/ })).toBeTruthy();
    // report 2 was opened before: no mark is sent for it
    await waitFor(() => expect(reports.setSeen).not.toHaveBeenCalledWith([id(2)], true));
    await user.keyboard("j");
    expect(await within(drawer).findByRole("heading", { name: /Synthetic report 1/ })).toBeTruthy();
    await waitFor(() => expect(reports.setSeen).toHaveBeenCalledWith([id(1)], true));
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("complementary", { name: "The report beside the list" })).toBeNull());
  });

  it("triages a report on its page, names the revision, and reads it again after a conflict", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    window.history.replaceState(null, "", `/admin/feedback/${idText(id(1))}?status=new`);
    show(<ReportPage client={admin()} reports={reports} />);
    expect(await screen.findByRole("heading", { name: /Synthetic report 1/ })).toBeTruthy();
    await waitFor(() => expect(reports.setSeen).toHaveBeenCalledWith([id(1)], true));
    await user.click(screen.getByRole("button", { name: "Triaged" }));
    await waitFor(() => expect(reports.setReport).toHaveBeenCalledWith(id(1), { status: "triaged" }, 1));
    expect((await screen.findAllByText("Status set to Triaged.")).length).toBeGreaterThan(0);
    // another administrator moved it on meanwhile
    reports.staleOnce();
    await user.click(screen.getByRole("button", { name: "In progress" }));
    expect(await screen.findByText(/Someone else changed this report meanwhile/)).toBeTruthy();
    await waitFor(() => expect(reports.report).toHaveBeenCalledTimes(2));
    // the list it came from: its position, and the way back
    expect(await screen.findByText("2 of 2")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Back to the list" }).getAttribute("href")).toBe("/admin/feedback?status=new");
  });

  it("keeps notes: everyone's shown, only one's own removable", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    window.history.replaceState(null, "", `/admin/feedback/${idText(id(3))}`);
    show(<ReportPage client={admin()} reports={reports} />);
    expect(await screen.findByText("Seen on Windows too.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Remove your note/ })).toBeNull();
    await user.type(screen.getByLabelText("A note for the administrators"), "Mine, synthetic.");
    await user.click(screen.getByRole("button", { name: "Save note" }));
    expect(await screen.findByText("Mine, synthetic.")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /Remove your note/ }));
    await waitFor(() => expect(screen.queryByText("Mine, synthetic.")).toBeNull());
    expect(vi.mocked(reports.removeNote).mock.calls.map(([r, n]) => [idText(r), idText(n)])).toEqual([[idText(id(3)), idText(id(0x61))]]);
  });

  it("shows the open reports and the new crashes on the overview", async () => {
    show(<OverviewPage client={admin()} />);
    expect(await screen.findByRole("heading", { name: "Open reports" })).toBeTruthy();
    expect(screen.getByText("2 new crashes wait for you.")).toBeTruthy();
    expect(screen.getByRole("link", { name: "New crashes" }).getAttribute("href")).toBe("/admin/feedback?status=new&kind=crash");
  });

  it("changes a selection in one request, and takes a move to the trash back", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    window.history.replaceState(null, "", "/admin/feedback?status=new");
    show(<FeedbackPage client={admin()} reports={reports} />);
    await user.click(await screen.findByRole("checkbox", { name: /Select: Synthetic report 1/ }));
    await user.click(screen.getByRole("checkbox", { name: /Select: Synthetic report 2/ }));
    const bar = screen.getByRole("region", { name: "The selected reports" });
    expect(within(bar).getByText("2 reports selected")).toBeTruthy();
    await user.selectOptions(within(bar).getByLabelText("Set the priority"), "high");
    await waitFor(() => expect(reports.bulk).toHaveBeenCalledWith([id(2), id(1)], { do: "change", change: { priority: "high" } }));
    expect((await screen.findAllByText("2 reports set to High.")).length).toBeGreaterThan(0);
    await user.click(within(screen.getByRole("region", { name: "The selected reports" })).getByRole("button", { name: "Move to the trash" }));
    await waitFor(() => expect(reports.bulk).toHaveBeenCalledWith([id(2), id(1)], { do: "trash" }));
    await waitFor(() => expect(screen.queryByRole("link", { name: /Synthetic report 1/ })).toBeNull());
    await user.click(await screen.findByRole("button", { name: "Undo" }));
    await waitFor(() => expect(reports.bulk).toHaveBeenCalledWith([id(2), id(1)], { do: "restore" }));
    expect(await screen.findByRole("link", { name: /Synthetic report 1/ })).toBeTruthy();
  });

  it("deletes for good from the trash only after a second step", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    await reports.trash(id(4));
    window.history.replaceState(null, "", "/admin/feedback?trash=1");
    show(<FeedbackPage client={admin()} reports={reports} />);
    await user.click(await screen.findByRole("checkbox", { name: /Select: Synthetic report 4/ }));
    await user.click(screen.getByRole("button", { name: "Delete for good" }));
    const dialog = await screen.findByRole("dialog");
    expect(reports.bulk).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Delete for good" }));
    await waitFor(() => expect(reports.bulk).toHaveBeenCalledWith([id(4)], { do: "purge" }));
    expect(await screen.findByText("No report matches.")).toBeTruthy();
  });

  it("groups the crashes and shows a group's reports", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    window.history.replaceState(null, "", "/admin/feedback");
    show(<FeedbackPage client={admin()} reports={reports} />);
    await user.click(await screen.findByRole("button", { name: "Crash groups" }));
    expect(await screen.findByText("index 7 out of range for 3")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Show its reports" }));
    expect(window.location.search).toBe(`?signature=${"a".repeat(32)}`);
  });

  it("draws the last 30 days when unfolded, and saves a list under a name", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    window.history.replaceState(null, "", "/admin/feedback?status=new&kind=crash");
    show(<FeedbackPage client={admin()} reports={reports} />);
    await screen.findByRole("link", { name: /Synthetic report 2/ });
    expect(reports.stats).not.toHaveBeenCalled();
    await user.click(screen.getByText("The last 30 days"));
    expect(await screen.findByText(/^Crashes:/)).toBeTruthy();
    await user.click(screen.getByText("Saved lists"));
    await user.type(await screen.findByLabelText("A name for this list"), "New crashes");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(reports.saveView).toHaveBeenCalledWith("New crashes", "status=new&kind=crash"));
    expect(await screen.findByRole("link", { name: "New crashes" })).toBeTruthy();
  });

  it("shows the keys on ?, and a single key does nothing once they are off", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    window.history.replaceState(null, "", "/admin/feedback?status=new");
    show(<FeedbackPage client={admin()} reports={reports} />);
    await screen.findByRole("link", { name: /Synthetic report 1/ });
    await user.keyboard("?");
    const dialog = await screen.findByRole("dialog", { name: "Keys" });
    await user.click(within(dialog).getByRole("checkbox", { name: "Single keys on this page" }));
    await user.click(within(dialog).getByRole("button", { name: "Close" }));
    await user.keyboard("x");
    expect(screen.queryByRole("region", { name: "The selected reports" })).toBeNull();
  });

  it("reads the list again quietly and announces new reports instead of moving the rows", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    try {
      const reports = fakeReports();
      window.history.replaceState(null, "", "/admin/feedback?status=new");
      show(<FeedbackPage client={admin()} reports={reports} />);
      await screen.findByRole("link", { name: /Synthetic report 1/ });
      const fresh = structuredClone(reports.rows[0]);
      if (fresh === undefined) throw new Error("no row");
      fresh.summary = { ...fresh.summary, id: idOf("0192d0c57b3e7a1f8c2d3e4f5a6b7c09") ?? id(9), excerpt: "Synthetic report 9: brand new", createdAt: 1_900_000_000 };
      reports.rows.push(fresh);
      vi.advanceTimersByTime(30_000);
      expect(await screen.findByText("A new report came in.")).toBeTruthy();
      expect(screen.queryByRole("link", { name: /Synthetic report 9/ })).toBeNull();
      await userEvent.setup().click(screen.getByRole("button", { name: "Show them" }));
      expect(await screen.findByRole("link", { name: /Synthetic report 9/ })).toBeTruthy();
    } finally {
      vi.useRealTimers();
    }
  });

  it("shows what the client sent: the screenshot through a blob address, the cards as chips; and hands over the record as gzip", async () => {
    const user = userEvent.setup();
    const reports = fakeReports();
    const row = reports.rows[0];
    if (row === undefined) throw new Error("no row");
    row.text = "It went wrong after [Lightning Bolt] resolved.";
    row.client = { bytes: 300, received: true };
    row.record = { bytes: 40, received: true };
    row.summary = { ...row.summary, recordOrigin: "gateway", recordBytes: 40, hasRecord: true };
    const client = {
      screenshot: { width: 1, height: 1, png_base64: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGNQM3ABAAEaAJsR09CMAAAAAElFTkSuQmCC" },
      refs: { cards: [{ text: "Lightning Bolt", at: [20, 36], zone: "stack", owner: 1, print: { scryfall_id: "77c6fa74-5543-42ac-9ead-0e890b188e99" } }], players: [] },
      system: { platform: "Synthetic OS", cpus: 8 },
    };
    vi.mocked(reports.part).mockImplementation(async (_r, part) =>
      part === "client" ? { gzip: new Uint8Array(gzipSync(JSON.stringify(client))), origin: null } : { gzip: new Uint8Array(gzipSync("{}\n")), origin: "gateway" },
    );
    const made: Blob[] = [];
    const created = vi.fn<(b: Blob) => string>((b) => {
      made.push(b);
      return `blob:test/${made.length}`;
    });
    const revoked = vi.fn<(url: string) => void>();
    Object.assign(URL, { createObjectURL: created, revokeObjectURL: revoked });
    window.history.replaceState(null, "", `/admin/feedback/${idText(id(1))}`);
    const view = show(<ReportPage client={admin()} reports={reports} />);
    const shot = await screen.findByRole("img", { name: /The client's window when the report was written, 1 × 1 pixels/ });
    expect(shot.getAttribute("src")).toBe("blob:test/1");
    expect(made[0]?.type).toBe("image/png");
    expect(reports.part).toHaveBeenCalledWith(id(1), "client", 2 * 1024 * 1024);
    expect(screen.getByRole("link", { name: /Lightning Bolt/ }).getAttribute("href")).toBe("https://scryfall.com/card/77c6fa74-5543-42ac-9ead-0e890b188e99");
    expect(screen.getByText("Synthetic OS")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Download the record (.gz)" }));
    await waitFor(() => expect(reports.part).toHaveBeenCalledWith(id(1), "record", 64 * 1024 * 1024));
    await waitFor(() => expect(made.some((b) => b.type === "application/gzip")).toBe(true));
    view.unmount();
    expect(revoked).toHaveBeenCalledWith("blob:test/1");
  });
});
