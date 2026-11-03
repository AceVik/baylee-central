// The chat identity kept in this browser while its sign-in lasts, so that the
// chat opens again after a reload without the password (docs/central-chat.md
// §"Identity and confidentiality"). The chat's Worker seals the identity's
// bytes with AES-256-GCM under a key the Web Crypto API made non-extractable,
// bound to the account, epoch and tag, and keeps key and ciphertext in
// IndexedDB; the bytes never reach the page. Signing out, the session's end
// and another account's chat forget it. One record, for one account.

const DB = "baylee-chat";
const STORE = "kept";
const DOMAIN = "baylee-central/browser-chat/v1";

/** Whose identity a record is, as the AAD binds it. */
export interface Owner { account: Uint8Array; epoch: number; tag: number }
export interface KeptRecord { owner: string; key: CryptoKey; iv: Uint8Array<ArrayBuffer>; sealed: ArrayBuffer }

const hex = (b: Uint8Array) => Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");
export const ownerOf = (o: Owner): string => `${hex(o.account)}:${o.epoch}:${o.tag}`;
const aad = (o: Owner) => new TextEncoder().encode(`${DOMAIN}\0${ownerOf(o)}`);

/** Seals `bytes` for `owner` under a new non-extractable key. */
export async function sealIdentity(bytes: Uint8Array<ArrayBuffer>, owner: Owner, subtle: SubtleCrypto = crypto.subtle): Promise<KeptRecord> {
  const key = await subtle.generateKey({ name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const sealed = await subtle.encrypt({ name: "AES-GCM", iv, additionalData: aad(owner) }, key, bytes);
  return { owner: ownerOf(owner), key, iv, sealed };
}

/** The bytes `record` holds for `owner`, or null when it is another's, altered or unreadable. */
export async function openIdentity(record: KeptRecord, owner: Owner, subtle: SubtleCrypto = crypto.subtle): Promise<Uint8Array | null> {
  if (record.owner !== ownerOf(owner)) return null;
  try {
    return new Uint8Array(await subtle.decrypt({ name: "AES-GCM", iv: record.iv, additionalData: aad(owner) }, record.key, record.sealed));
  } catch {
    return null;
  }
}

function database(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const open = indexedDB.open(DB, 1);
    open.addEventListener("upgradeneeded", () => { open.result.createObjectStore(STORE); });
    open.addEventListener("success", () => resolve(open.result));
    open.addEventListener("error", () => reject(open.error ?? new Error("unavailable")));
  });
}

function run<T>(mode: IDBTransactionMode, work: (store: IDBObjectStore) => IDBRequest<T> | null): Promise<T | undefined> {
  return database().then((db) => new Promise<T | undefined>((resolve, reject) => {
    const tx = db.transaction(STORE, mode);
    const request = work(tx.objectStore(STORE));
    const failed = () => { db.close(); reject(tx.error ?? new Error("unavailable")); };
    tx.addEventListener("complete", () => { db.close(); resolve(request?.result); });
    tx.addEventListener("error", failed);
    tx.addEventListener("abort", failed);
  }));
}

/** Keeps `record` as the browser's one kept identity. */
export function storeKept(record: KeptRecord): Promise<void> {
  return run("readwrite", (store) => { store.clear(); store.put(record, "identity"); return null; }).then(() => {});
}

/** The kept identity, if any. */
export function readKept(): Promise<KeptRecord | null> {
  return run<KeptRecord>("readonly", (store) => store.get("identity") as IDBRequest<KeptRecord>).then((r) => r ?? null);
}

/** Forgets every kept identity; where IndexedDB is missing there is nothing to forget. */
export function forgetKept(): Promise<void> {
  if (typeof indexedDB === "undefined") return Promise.resolve();
  return run("readwrite", (store) => { store.clear(); return null; }).then(() => {}, () => {});
}
