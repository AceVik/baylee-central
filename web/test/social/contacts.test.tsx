import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ApiError } from "../../src/account/api";
import { useContacts } from "../../src/social/useContacts";
import type { Contact, SocialApi } from "../../src/social/api";

const friend: Contact = { person: { tag: 0x3fa2, displayName: "Synthetic" }, relation: "friend" };
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
function api(read: SocialApi["contacts"], change: SocialApi["change"] = async () => {}): SocialApi {
  return { contacts: read, lookup: async () => null, change };
}

describe("contact refresh and mutation races", () => {
  it("a delayed initial read cannot resurrect a contact after its removal", async () => {
    const old = deferred<Contact[]>();
    const read = vi.fn<SocialApi["contacts"]>().mockImplementationOnce(() => old.promise).mockResolvedValue([]);
    const apiClient = api(read);
    const { result } = renderHook(() => useContacts(apiClient));
    await waitFor(() => expect(read).toHaveBeenCalledTimes(1));
    await act(async () => { await result.current.change(friend.person.tag, "remove"); });
    expect(result.current.items).toEqual([]);
    await act(async () => { old.resolve([friend]); });
    expect(result.current.items).toEqual([]);
  });
  it("retains the last list on a failed refresh and clears it when the session is rejected", async () => {
    const read = vi.fn<SocialApi["contacts"]>().mockResolvedValueOnce([friend]).mockRejectedValueOnce(new ApiError("network")).mockRejectedValueOnce(new ApiError("rejected"));
    const apiClient = api(read);
    const { result } = renderHook(() => useContacts(apiClient));
    await waitFor(() => expect(result.current.items).toEqual([friend]));
    await act(async () => { await result.current.reload(); });
    expect(result.current.error).toBe(true);
    expect(result.current.items).toEqual([friend]);
    await act(async () => { await result.current.reload(); });
    expect(result.current.denied).toBe(true);
    expect(result.current.items).toBeNull();
  });

  it("coalesces overlapping reads and prevents double-click mutations", async () => {
    const pending = deferred<void>();
    const change = vi.fn<SocialApi["change"]>(() => pending.promise);
    const read = vi.fn<SocialApi["contacts"]>().mockResolvedValue([friend]);
    const apiClient = api(read, change);
    const { result } = renderHook(() => useContacts(apiClient));
    await waitFor(() => expect(result.current.items).toEqual([friend]));
    let first!: Promise<boolean>;
    await act(async () => {
      first = result.current.change(friend.person.tag, "block");
      expect(await result.current.change(friend.person.tag, "block")).toBe(false);
      await result.current.reload();
    });
    expect(change).toHaveBeenCalledTimes(1);
    expect(read).toHaveBeenCalledTimes(1);
    await act(async () => { pending.resolve(); await first; });
    expect(result.current.busy).toBe(false);
  });
});
