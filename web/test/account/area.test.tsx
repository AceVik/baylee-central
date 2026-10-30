// The account area with stand-ins: the sessions Central knows and ending the
// client's; one's own gateways, a token made and shown once in its text
// form, and one revoked after a second step.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import GatewaysPage from "../../src/account/GatewaysPage";
import type { MyGateway, OwnerApi } from "../../src/account/owner-api";
import SecurityPage from "../../src/account/SecurityPage";
import { AccountProvider, SESSION_KEY } from "../../src/account/session";
import { LangProvider } from "../../src/i18n";
import { ACCOUNT, fakeCentral, fakeCrypto } from "../support/central";
import { standInDialogs } from "../support/dialog";

standInDialogs();

const ID = Uint8Array.from([0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d]);
const TOKEN = Uint8Array.from([0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8e]);
const ORIGIN = "https://gateway.invalid";

function signedIn() {
  const api = fakeCentral();
  api.accounts.set("synthetic_name", { identity: { account: ACCOUNT, epoch: 1 }, wrapped: new Uint8Array(73), recovery: new Uint8Array(73), displayName: "Synthetic_Name", tag: 0x3fa2 });
  api.session = { name: "synthetic_name", expiresAt: Math.floor(Date.now() / 1000) + 600 };
  window.localStorage.setItem(SESSION_KEY, JSON.stringify({ expiresAt: api.session.expiresAt }));
  return api;
}

function show(node: ReactNode) {
  const api = signedIn();
  return render(
    <LangProvider initial="en">
      <AccountProvider api={api} crypto={fakeCrypto("unused").factory}>
        {node}
      </AccountProvider>
    </LangProvider>,
  );
}

function gateway(): MyGateway {
  return {
    id: ID,
    origin: ORIGIN,
    metadata: { name: "Synthetic gateway", version: "0", website: `${ORIGIN}/` },
    state: "approved",
    online: true,
    lastSeen: 1_800_000_000,
    createdAt: 1_800_000_000,
    tokens: [{ id: TOKEN, name: "Old machine", scopes: { sso: true, listing: false }, createdAt: 1_800_000_000, expiresAt: 4_000_000_000, revokedAt: null, lastUsedAt: null }],
  };
}

function owner(overrides: Partial<OwnerApi> = {}): OwnerApi {
  return {
    sessions: vi.fn<OwnerApi["sessions"]>(async () => [
      { kind: "browser" as const, current: true, signedInAt: 1_800_000_000, expiresAt: 1_800_001_800, gateways: [] },
      { kind: "native" as const, current: false, signedInAt: 1_800_000_000, expiresAt: 1_800_001_800, gateways: [ORIGIN] },
    ]),
    endClientSession: vi.fn<OwnerApi["endClientSession"]>(async () => {}),
    gateways: vi.fn<OwnerApi["gateways"]>(async () => [gateway()]),
    addGateway: vi.fn<OwnerApi["addGateway"]>(async () => ID),
    createToken: vi.fn<OwnerApi["createToken"]>(async () => ({ id: TOKEN, text: `bgt_${"ab".repeat(32)}`, expiresAt: 1_807_776_000 })),
    revokeToken: vi.fn<OwnerApi["revokeToken"]>(async () => {}),
    ...overrides,
  };
}

beforeEach(() => window.localStorage.clear());

describe("security and sessions", () => {
  it("lists the website's and the client's session, with the client's gateways, and ends the client's after a second step", async () => {
    const user = userEvent.setup();
    const client = owner();
    show(<SecurityPage client={client} />);
    expect(await screen.findByText("The Baylee client")).toBeTruthy();
    expect(screen.getByText("this session")).toBeTruthy();
    expect(screen.getByText(ORIGIN)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Sign the client out" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Sign the client out" }));
    await waitFor(() => expect(client.endClientSession).toHaveBeenCalledOnce());
    expect(await screen.findByText("The client is signed out.")).toBeTruthy();
  });
});

describe("your gateways", () => {
  it("makes a token and shows it once, in its text form", async () => {
    const user = userEvent.setup();
    const client = owner();
    show(<GatewaysPage client={client} />);
    expect(await screen.findByRole("heading", { name: "Synthetic gateway" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Make a token" }));
    const form = await screen.findByRole("dialog");
    await user.type(within(form).getByLabelText("Name"), "Main machine");
    await user.click(within(form).getByLabelText("Keeping its entry in the public gateway list"));
    await user.click(within(form).getByRole("button", { name: "Make the token" }));
    await waitFor(() => expect(client.createToken).toHaveBeenCalledWith(ID, "Main machine", { sso: true, listing: false }, 90));
    expect(await screen.findByText(`bgt_${"ab".repeat(32)}`)).toBeTruthy();
    expect(screen.getByText(/Shown only now/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "I kept it" }));
    await waitFor(() => expect(screen.queryByText(`bgt_${"ab".repeat(32)}`)).toBeNull());
    expect(client.gateways).toHaveBeenCalledTimes(2);
  });

  it("refuses a token that allows nothing or has no name, before asking Central", async () => {
    const user = userEvent.setup();
    const client = owner();
    show(<GatewaysPage client={client} />);
    await user.click(await screen.findByRole("button", { name: "Make a token" }));
    const form = await screen.findByRole("dialog");
    await user.click(within(form).getByRole("button", { name: "Make the token" }));
    expect(within(form).getByText("Give the token a name of up to 64 characters.")).toBeTruthy();
    await user.type(within(form).getByLabelText("Name"), "x");
    await user.click(within(form).getByLabelText("Signing players in through Baylee Central"));
    await user.click(within(form).getByLabelText("Keeping its entry in the public gateway list"));
    await user.click(within(form).getByRole("button", { name: "Make the token" }));
    expect(within(form).getByText("A token must allow something.")).toBeTruthy();
    expect(client.createToken).not.toHaveBeenCalled();
  });

  it("revokes a live token after a second step", async () => {
    const user = userEvent.setup();
    const client = owner();
    show(<GatewaysPage client={client} />);
    await user.click(await screen.findByRole("button", { name: "Revoke" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Revoke “Old machine”?")).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Revoke" }));
    await waitFor(() => expect(client.revokeToken).toHaveBeenCalledWith(TOKEN));
  });

  it("registers a gateway by its origin and says when the address is taken", async () => {
    const user = userEvent.setup();
    const { ApiError } = await import("../../src/account/api");
    const client = owner({ addGateway: vi.fn<OwnerApi["addGateway"]>(async () => Promise.reject(new ApiError("conflict"))) });
    show(<GatewaysPage client={client} />);
    const address = await screen.findByLabelText("Address");
    await user.clear(address);
    await user.type(address, "https://Play.Example.org/some/path");
    await user.type(screen.getByLabelText("Name in the list"), "My table");
    await user.click(screen.getByRole("button", { name: "Register" }));
    await waitFor(() => expect(client.addGateway).toHaveBeenCalledWith("https://play.example.org", "My table"));
    expect(await screen.findByText("This address is registered already.")).toBeTruthy();
  });
});
