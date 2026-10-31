// The feedback section with stand-ins (test/support/reports.ts): the views
// with their counts, the search in the address, a report triaged on its page
// with its revision named and read again on a conflict, notes removed by
// their author only, the unread count in the navigation, and the overview's
// tile.

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
});
