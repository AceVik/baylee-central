// The administration with stand-ins: nobody but an administrator sees it;
// an account's tag changes after a second step that warns about whispers;
// a pending gateway is approved, an approved one disabled.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { idText } from "../../src/account/owner-api";
import { AccountProvider, SESSION_KEY } from "../../src/account/session";
import { ApiError } from "../../src/account/api";
import type { AdminApi, AdminGateway, LegacyStore } from "../../src/admin/admin-api";
import AuditPage from "../../src/admin/AuditPage";
import GatewaysPage from "../../src/admin/GatewaysPage";
import LegacyPage from "../../src/admin/LegacyPage";
import OverviewPage from "../../src/admin/OverviewPage";
import UserPage from "../../src/admin/UserPage";
import UsersPage from "../../src/admin/UsersPage";
import { LangProvider } from "../../src/i18n";
import { ACCOUNT, fakeCentral, fakeCrypto } from "../support/central";
import { standInDialogs } from "../support/dialog";

standInDialogs();

const OTHER = Uint8Array.from([0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x99]);
const GW = Uint8Array.from([0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0xaa]);

function show(node: ReactNode, isAdmin = true) {
  const api = fakeCentral();
  api.accounts.set("synthetic_admin", { identity: { account: ACCOUNT, epoch: 1 }, wrapped: new Uint8Array(73), recovery: new Uint8Array(73), displayName: "Synthetic_Admin", tag: 0x1000 });
  api.session = { name: "synthetic_admin", expiresAt: Math.floor(Date.now() / 1000) + 600 };
  const summary = api.accountSummary.bind(api);
  api.accountSummary = async () => ({ ...(await summary()), admin: isAdmin });
  window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: api.session.expiresAt }));
  return render(
    <LangProvider initial="en">
      <AccountProvider api={api} crypto={fakeCrypto("unused").factory}>
        {node}
      </AccountProvider>
    </LangProvider>,
  );
}

const account = {
  id: OTHER,
  username: "synthetic_player",
  displayName: "Synthetic_Player",
  tag: 0x3fa2,
  source: "registered" as const,
  state: "restored" as const,
  createdAt: 1_800_000_000,
  lastSignInAt: 1_800_000_100,
  admin: false,
  resetUntil: null,
};

function gateway(state: AdminGateway["state"]): AdminGateway {
  return {
    id: GW,
    origin: "https://gateway.invalid",
    metadata: { name: "Synthetic gateway", version: "1.0", website: "https://gateway.invalid/" },
    official: false,
    owner: OTHER,
    ownerName: "Synthetic_Player",
    ownerTag: 0x3fa2,
    state,
    online: false,
    lastSeen: null,
    createdAt: 1_800_000_000,
    liveTokens: 1,
    credentialPair: false,
  };
}

const LEGACY: LegacyStore = {
  deadline: 1_800_864_000,
  accounts: [
    {
      legacyId: "0192d0c5-7b3e-7a1f-8c2d-3e4f5a6b7c01",
      username: "synthetic_moving",
      displayName: "Moving",
      tag: 0x2b1c,
      progress: "reserved",
      importedAt: 1_800_000_000,
      failures: 3,
      lockedUntil: 4_000_000_000,
      mustChangePassword: false,
      banned: true,
      snapshotBytes: 2048,
    },
  ],
};

const OVERVIEW = { accounts: 12, accountsWeek: 3, moved: 4, admins: 2, pending: 2, approved: 5, disabled: 1, online: 4, liveTokens: 7 };

function admin(overrides: Partial<AdminApi> = {}): AdminApi {
  return {
    accounts: vi.fn<AdminApi["accounts"]>(async () => ({ items: [account], more: false })),
    account: vi.fn<AdminApi["account"]>(async () => ({ account, recovery: true, gateways: [], audit: [] })),
    audit: vi.fn<AdminApi["audit"]>(async () => ({ items: [], more: false, names: { accounts: new Map(), gateways: new Map() } })),
    gateways: vi.fn<AdminApi["gateways"]>(async (state) => ({ items: state === null ? [] : [gateway(state)], more: false })),
    gateway: vi.fn<AdminApi["gateway"]>(async () => ({ gateway: gateway("approved"), tokens: [] })),
    setAdmin: vi.fn<AdminApi["setAdmin"]>(async () => {}),
    resetLogin: vi.fn<AdminApi["resetLogin"]>(async () => ({ code: "RESET-AAAA-AAAA-AAAA-AAAA-AAAA-AAAA-AAAA-AAAA", expiresAt: 1_800_086_400 })),
    setTag: vi.fn<AdminApi["setTag"]>(async () => {}),
    setGatewayApproval: vi.fn<AdminApi["setGatewayApproval"]>(async () => {}),
    revokeGatewayToken: vi.fn<AdminApi["revokeGatewayToken"]>(async () => {}),
    legacyAccounts: vi.fn<AdminApi["legacyAccounts"]>(async () => LEGACY),
    overview: vi.fn<AdminApi["overview"]>(async () => OVERVIEW),
    ...overrides,
  };
}

beforeEach(() => {
  window.localStorage.clear();
  window.history.replaceState(null, "", "/");
});

describe("the administration", () => {
  it("is for administrators only", async () => {
    const client = admin();
    show(<UsersPage client={client} />, false);
    expect(await screen.findByText("This part is for Baylee Central's administrators.")).toBeTruthy();
    expect(client.accounts).not.toHaveBeenCalled();
    expect(screen.queryByRole("link", { name: "Accounts" })).toBeNull();
  });

  it("lists the accounts and links each to its page", async () => {
    show(<UsersPage client={admin()} />);
    const link = await screen.findByRole("link", { name: "Synthetic_Player#3fa2" });
    expect(link.getAttribute("href")).toBe(`/admin/users/${idText(OTHER)}`);
  });

  it("gives an account another tag after a step that warns about whispers", async () => {
    const user = userEvent.setup();
    const client = admin();
    window.history.replaceState(null, "", `/admin/users/${idText(OTHER)}`);
    show(<UserPage client={client} />);
    const field = await screen.findByLabelText("Tag", { selector: "input" });
    await user.type(field, "#zz");
    await user.click(screen.getByRole("button", { name: "Set the tag" }));
    expect(await screen.findByText("A tag is one to eight hex digits, from #0001.")).toBeTruthy();
    await user.clear(field);
    await user.type(field, "#1A2B");
    await user.click(screen.getByRole("button", { name: "Set the tag" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/earlier whisper conversations/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Set the tag" }));
    await waitFor(() => expect(client.setTag).toHaveBeenCalledWith(OTHER, 0x1a2b));
    expect(await screen.findByText("The tag is set.")).toBeTruthy();
  });

  it("lists the legacy accounts left, how far each got and what holds it back", async () => {
    show(<LegacyPage client={admin()} />);
    expect(await screen.findByText("synthetic_moving")).toBeTruthy();
    expect(screen.getByText("Moving#2b1c")).toBeTruthy();
    expect(screen.getByText("Name reserved")).toBeTruthy();
    expect(screen.getByText("Banned")).toBeTruthy();
    expect(screen.getByText(/^Locked until/)).toBeTruthy();
    expect(screen.getByText(/^Left: 1\. After /)).toBeTruthy();
    expect(screen.getByRole("link", { name: "Legacy accounts" }).getAttribute("aria-current")).toBe("page");
  });

  it("says when this Central keeps no legacy accounts", async () => {
    show(<LegacyPage client={admin({ legacyAccounts: vi.fn<AdminApi["legacyAccounts"]>(async () => Promise.reject(new ApiError("rejected"))) })} />);
    expect(await screen.findByText("This Central keeps no legacy accounts.")).toBeTruthy();
  });

  it("offers actions on another account and none on one's own", async () => {
    window.history.replaceState(null, "", `/admin/users/${idText(OTHER)}`);
    const other = show(<UserPage client={admin()} />);
    expect(await screen.findByRole("button", { name: "Make administrator" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Actions" })).toBeTruthy();
    other.unmount();
    const own = { ...account, id: ACCOUNT, admin: true };
    window.history.replaceState(null, "", `/admin/users/${idText(ACCOUNT)}`);
    show(<UserPage client={admin({ account: vi.fn<AdminApi["account"]>(async () => ({ account: own, recovery: true, gateways: [], audit: [] })) })} />);
    expect(await screen.findByLabelText("Tag", { selector: "input" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Actions" })).toBeNull();
  });

  it("approves a pending gateway and disables an approved one", async () => {
    const user = userEvent.setup();
    const client = admin();
    show(<GatewaysPage client={client} />);
    await user.click(await screen.findByRole("button", { name: "Approve" }));
    let dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/https:\/\/gateway\.invalid/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Approve" }));
    await waitFor(() => expect(client.setGatewayApproval).toHaveBeenCalledWith(GW, true));
    // the change is counted again: the navigation and the filters follow
    await waitFor(() => expect(client.overview).toHaveBeenCalledTimes(2));
    expect(screen.getByRole("link", { name: /^Gateways 2 awaiting approval/ })).toBeTruthy();
    // each filter says how many it holds, and the address keeps the choice
    await user.click(screen.getByRole("button", { name: "Approved 5" }));
    expect(window.location.search).toBe("?state=approved");
    await user.click(await screen.findByRole("button", { name: "Disable" }));
    dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Disable" }));
    await waitFor(() => expect(client.setGatewayApproval).toHaveBeenCalledWith(GW, false));
  });

  it("opens on the counts, each leading where the work is, and never for anyone else", async () => {
    const client = admin();
    show(<OverviewPage client={client} />);
    expect(await screen.findByRole("heading", { name: "Awaiting approval" })).toBeTruthy();
    expect(screen.getByText("3 new this week, 4 moved from the official gateway")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Review gateways" }).getAttribute("href")).toBe("/admin/gateways");
    expect(screen.getByRole("link", { name: "Approved gateways" }).getAttribute("href")).toBe("/admin/gateways?state=approved");
    // the legacy store's tile, from its own read
    expect(await screen.findByRole("heading", { name: "Left to move" })).toBeTruthy();
    const refused = admin();
    show(<OverviewPage client={refused} />, false);
    await screen.findAllByText("This part is for Baylee Central's administrators.");
    expect(refused.overview).not.toHaveBeenCalled();
  });

  it("follows the search as it is typed, keeps it in the address, and Escape empties it", async () => {
    const user = userEvent.setup();
    const client = admin();
    show(<UsersPage client={client} />);
    await screen.findByRole("link", { name: "Synthetic_Player#3fa2" });
    const field = screen.getByRole("searchbox", { name: "Search" });
    await user.type(field, "synth");
    await waitFor(() => expect(client.accounts).toHaveBeenLastCalledWith("synth", null));
    expect(window.location.search).toBe("?q=synth");
    await user.type(field, "{Escape}");
    expect((field as HTMLInputElement).value).toBe("");
    await waitFor(() => expect(client.accounts).toHaveBeenLastCalledWith(null, null));
    expect(window.location.search).toBe("");
  });

  it("names the accounts and gateways of the audit log as they are called today", async () => {
    const event = { id: GW, at: 1_800_000_000, actor: ACCOUNT, action: "gateway.approved", account: null, gateway: GW, betaKey: null };
    const names = {
      accounts: new Map([[idText(ACCOUNT), { displayName: "Synthetic_Admin", tag: 0x1000 }]]),
      gateways: new Map([[idText(GW), { name: "Synthetic gateway", origin: "https://gateway.invalid" }]]),
    };
    show(<AuditPage client={admin({ audit: vi.fn<AdminApi["audit"]>(async () => ({ items: [event], more: false, names })) })} />);
    expect(await screen.findByRole("link", { name: "Synthetic_Admin#1000" })).toBeTruthy();
    expect(screen.getByText("Synthetic gateway")).toBeTruthy();
  });
});
