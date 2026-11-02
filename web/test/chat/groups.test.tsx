import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ApiError } from "../../src/account/api";
import type { GroupApi, Groups } from "../../src/chat/groups";
import { useGroups } from "../../src/chat/useGroups";

function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done; }); return { promise, resolve }; }
const id = "019a3d70-0000-7000-8000-000000000001";
const empty: Groups = { groups: [], invites: [] };
const joined: Groups = { groups: [{ id, name: "Friends", members: [{ person: { tag: 7, displayName: "Alice" }, owner: true }], invited: [] }], invites: [] };
function client(): GroupApi { return { list: vi.fn<GroupApi["list"]>().mockResolvedValue(joined), create: vi.fn<GroupApi["create"]>().mockResolvedValue(id), change: vi.fn<GroupApi["change"]>().mockResolvedValue(undefined) }; }

describe("group lifecycle and races", () => {
  it("does not resurrect a deleted group from a late refresh", async () => {
    const api = client(); const old = deferred<Groups>();
    api.list = vi.fn<GroupApi["list"]>().mockImplementationOnce(() => old.promise).mockResolvedValue(empty);
    const { result } = renderHook(() => useGroups(api));
    await waitFor(() => expect(api.list).toHaveBeenCalledTimes(1));
    await act(async () => { await result.current.change(id, { change: "delete" }); });
    await act(async () => { old.resolve(joined); });
    expect(result.current.groups).toEqual([]);
  });
  it("suppresses double submissions and does no follow-up after leaving the account", async () => {
    const api = client(); const pending = deferred<void>();
    api.change = vi.fn<GroupApi["change"]>(() => pending.promise);
    const { result, unmount } = renderHook(() => useGroups(api));
    await waitFor(() => expect(result.current.groups).toHaveLength(1));
    let first!: Promise<string | undefined | false>;
    await act(async () => { first = result.current.change(id, { change: "delete" }); expect(await result.current.change(id, { change: "delete" })).toBe(false); });
    expect(api.change).toHaveBeenCalledTimes(1);
    unmount(); pending.resolve(); expect(await first).toBe(false);
    expect(api.list).toHaveBeenCalledTimes(1);
  });
  it("keeps a snapshot during outages and clears it when authorization is rejected", async () => {
    const api = client(); api.list = vi.fn<GroupApi["list"]>().mockResolvedValueOnce(joined).mockRejectedValueOnce(new ApiError("network")).mockRejectedValueOnce(new ApiError("rejected"));
    const { result } = renderHook(() => useGroups(api));
    await waitFor(() => expect(result.current.groups).toHaveLength(1));
    await act(async () => { await result.current.reload(); });
    expect(result.current.groups).toHaveLength(1); expect(result.current.error).toBe(true);
    await act(async () => { await result.current.reload(); });
    expect(result.current.groups).toEqual([]);
  });
});
