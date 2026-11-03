// The kept chat identity: sealed under a non-extractable key, opened only
// for the owner it was sealed for, refused when altered.
import { webcrypto } from "node:crypto";

import { describe, expect, it } from "vitest";

import { openIdentity, ownerOf, sealIdentity, type Owner } from "../../src/chat/kept";

const subtle = webcrypto.subtle as unknown as SubtleCrypto;
const owner: Owner = { account: Uint8Array.from({ length: 16 }, (_, i) => i + 1), epoch: 1, tag: 0x3fa2 };
const secret = Uint8Array.from({ length: 96 }, (_, i) => (i * 7) & 255);

describe("the kept chat identity", () => {
  it("opens for its owner, under a key that cannot be exported", async () => {
    const record = await sealIdentity(secret.slice(), owner, subtle);
    expect(record.owner).toBe(ownerOf(owner));
    expect(record.key.extractable).toBe(false);
    expect(new Uint8Array(record.sealed)).not.toEqual(secret);
    expect(await openIdentity(record, owner, subtle)).toEqual(secret);
    await expect(subtle.exportKey("raw", record.key)).rejects.toThrow(/extractable/i);
  });

  it("opens for nobody else and not once altered", async () => {
    const record = await sealIdentity(secret.slice(), owner, subtle);
    for (const other of [{ ...owner, tag: 0x3fa3 }, { ...owner, epoch: 2 }, { ...owner, account: new Uint8Array(16) }]) {
      expect(await openIdentity(record, other, subtle)).toBeNull();
      // the AAD binds it too, whatever the record claims
      expect(await openIdentity({ ...record, owner: ownerOf(other) }, other, subtle)).toBeNull();
    }
    const sealed = new Uint8Array(record.sealed.slice(0));
    sealed[0] = (sealed[0] ?? 0) ^ 1;
    expect(await openIdentity({ ...record, sealed: sealed.buffer }, owner, subtle)).toBeNull();
    const iv = record.iv.slice();
    iv[0] = (iv[0] ?? 0) ^ 1;
    expect(await openIdentity({ ...record, iv }, owner, subtle)).toBeNull();
  });
});
