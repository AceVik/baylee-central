import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ChatWindow } from "../../src/chat/ChatWindow";
import { GroupList, GroupSettings } from "../../src/chat/GroupPanel";
import type { ChatSession, Snapshot } from "../../src/chat/session";
import type { GroupState } from "../../src/chat/useGroups";
import type { Contact } from "../../src/social/api";
import { LangProvider } from "../../src/i18n";
import { standInDialogs } from "../support/dialog";

const hooks = vi.hoisted(() => ({ contacts: [] as Contact[] }));
vi.mock("../../src/social/useContacts", () => ({ useContacts: () => ({ items: hooks.contacts, error: false, reload: vi.fn<GroupState["reload"]>() }) }));
vi.mock("../../src/chat/useGroups", () => ({ useGroups: () => ({ groups: [], invites: [], busy: false, error: false, reload: vi.fn<GroupState["reload"]>(), create: vi.fn<GroupState["create"]>(), change: vi.fn<GroupState["change"]>() }) }));
const bob = { tag: 9, displayName: "Bob" };
const alice = { tag: 7, displayName: "Alice" };
const id = "019a3d70-0000-7000-8000-000000000001";
const conv = "dm:0007:0009";
const sealed = { hdr: "header", ct: "ciphertext", n: "nonce", epk: "ephemeral", sig: "signature", wraps: [] };
function show(snapshot: Partial<Snapshot> = {}) {
  const data: Snapshot = { messages: [], people: [bob], reviews: [], pending: null, loading: false, busy: false, denied: false, error: false, refused: 0, ...snapshot };
  const session = {
    subscribe: () => () => {}, getSnapshot: () => data,
    keys: vi.fn<ChatSession["keys"]>().mockResolvedValue([]), send: vi.fn<ChatSession["send"]>().mockResolvedValue(true), poll: vi.fn<ChatSession["poll"]>().mockResolvedValue(true),
    retry: vi.fn<ChatSession["retry"]>().mockResolvedValue(true), discard: vi.fn<ChatSession["discard"]>(), confirm: vi.fn<ChatSession["confirm"]>().mockResolvedValue(undefined),
  };
  window.history.replaceState(null, "", "/account/chat?with=0009");
  render(<LangProvider initial="en"><ChatWindow session={session} ownTag={7} onLock={vi.fn<() => void>()} /></LangProvider>);
  return { session, user: userEvent.setup() };
}
function state(): GroupState {
  return { groups: [], invites: [], busy: false, error: false, reload: vi.fn<GroupState["reload"]>().mockResolvedValue(undefined), create: vi.fn<GroupState["create"]>().mockResolvedValue(id), change: vi.fn<GroupState["change"]>().mockResolvedValue(undefined) };
}
beforeEach(() => { hooks.contacts = [{ person: bob, relation: "friend" }]; standInDialogs(); });

describe("chat interaction safeguards", () => {
  it("renders messages as plain text and bounds the visible history until requested", async () => {
    const messages = Array.from({ length: 125 }, (_, n) => ({ id: String(n), conv, sender: 9, text: n === 124 ? '<img src=x onerror="alert(1)">' : `message ${n}`, emote: false, at: 1000 + n }));
    const { user } = show({ messages });
    const log = screen.getByRole("log");
    expect(log.querySelectorAll("article")).toHaveLength(100);
    expect(log.querySelector("img")).toBeNull();
    expect(within(log).getByText('<img src=x onerror="alert(1)">')).toBeTruthy();
    expect(within(log).queryByText("message 0")).toBeNull();
    await user.click(within(log).getByRole("button"));
    expect(log.querySelectorAll("article")).toHaveLength(125);
    expect(within(log).getByText("message 0")).toBeTruthy();
  });
  it("blocks sends after a key change and confirms only the displayed snapshot", async () => {
    const snapshot = { tag: "0009", handle: "Bob#0009", devices: [] };
    const { session, user } = show({ reviews: [{ tag: "0009", number: "1".repeat(60), changed: true, snapshot }] });
    await user.type(screen.getByRole("textbox", { name: "Write a message" }), "hello");
    expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByText("Safety number", { selector: "summary" }));
    await user.click(screen.getByRole("button", { name: /Bob#0009.*⚠/ }));
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText(/11111 11111/)).toBeTruthy();
    expect(session.confirm).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Confirm these keys" }));
    expect(session.confirm).toHaveBeenCalledExactlyOnceWith(snapshot);
    expect(session.send).not.toHaveBeenCalled();
  });
  it("requires an explicit confirmation to abandon an uncertain send", async () => {
    const { session, user } = show({ pending: { conv, text: "perhaps delivered", members: [7, 9], sealed, failed: true } });
    expect((screen.getByRole("textbox", { name: "Write a message" }) as HTMLTextAreaElement).disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Discard send attempt" }));
    let dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText(/may already have been delivered/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(session.discard).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Discard send attempt" }));
    dialog = screen.getByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Discard send attempt" }));
    expect(session.discard).toHaveBeenCalledExactlyOnceWith(sealed);
  });
  it("keeps former contacts' history readable without presenting a send permission", () => {
    hooks.contacts = [];
    show({ messages: [{ id, conv, sender: 9, text: "old message", emote: false, at: 1000 }] });
    expect(screen.getByText("old message")).toBeTruthy();
    expect(screen.queryByRole("textbox", { name: "Write a message" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Send" })).toBeNull();
  });
});

describe("group permissions and confirmation", () => {
  it("does not navigate into an invitation whose acceptance failed", async () => {
    const groups = state(); groups.invites = [{ group: id, name: "Friends", inviter: bob, at: 1000 }];
    groups.change = vi.fn<GroupState["change"]>().mockResolvedValue(false);
    const onChoose = vi.fn<(id: string) => void>(); const user = userEvent.setup();
    render(<LangProvider initial="en"><GroupList state={groups} selected={null} ownTag={7} onChoose={onChoose} /></LangProvider>);
    await user.click(screen.getByRole("button", { name: "Accept" }));
    expect(groups.change).toHaveBeenCalledExactlyOnceWith(id, { change: "join" });
    expect(onChoose).not.toHaveBeenCalled();
  });
  it("names the ownership recipient and performs no mutation when cancelled", async () => {
    const groups = state(); const user = userEvent.setup();
    const group = { id, name: "Friends", members: [{ person: alice, owner: true }, { person: bob, owner: false }], invited: [] };
    render(<LangProvider initial="en"><GroupSettings group={group} ownTag={7} friends={[]} state={groups} /></LangProvider>);
    await user.click(screen.getByText(/Manage group/));
    await user.click(screen.getByRole("button", { name: "Transfer ownership" }));
    let dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText("Bob#0009")).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(groups.change).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Transfer ownership" }));
    dialog = screen.getByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Transfer ownership" }));
    expect(groups.change).toHaveBeenCalledExactlyOnceWith(id, { change: "transfer", tag: 9 });
  });
});
