// The browser crypto in a module Worker (docs/web-accounts.md §"Browser
// crypto"): central-web-crypto's committed WebAssembly module, unchanged,
// with its Argon2 (64 MiB, three passes) off the page's thread. A Worker
// needs a script of its own; this is the whole of it, everything the page
// does is Rust (crates/central-web/src/islands).
//
// One flow per Worker: a registration, a sign-in (which may keep the chat
// identity and then serve the chat for as long as it lives), a deletion's
// password proof, or a chat resumed after a reload. The master key never
// leaves this Worker; the page sends and receives bytes and words only.
// Terminating the Worker discards the module's memory.
//
// "unlocked" answers the device and registration of the identity a
// sign-in in this Worker unlocked (the chat page's way back when nothing
// was kept in this browser).
//
// Protocol: the page posts JSON `{id, op, ...}`; the Worker answers JSON
// `{id, ok: {...}}` or `{id, error}` with error one of "rejected",
// "unavailable", "failed". Bytes travel as arrays of numbers, as on
// Central's wire. Requests are answered in order.
//
// `cargo xtask site` writes the hashed addresses of the glue and the module
// over "./central_web_crypto.js" and "./central_web_crypto_bg.wasm".
import init, {
  chat_resume,
  chat_unlock,
  enrollment_finish,
  enrollment_start,
  login_confirm,
  login_finish,
  login_start,
  recovery_create,
} from "./central_web_crypto.js";

const ready = init({ module_or_path: "./central_web_crypto_bg.wasm" });

let enrollment = null;
let login = null;
let key = null;
let chat = null;

const bytes = (value) => {
  if (!Array.isArray(value) || value.length > 1 << 20 || !value.every((b) => Number.isInteger(b) && b >= 0 && b <= 255)) {
    throw new Error("failed");
  }
  return Uint8Array.from(value);
};
const array = (value) => Array.from(value);
const take = (state) => {
  if (state === null) throw new Error("failed");
  return state;
};
const encode = (value) => new TextEncoder().encode(JSON.stringify(value));
const decode = (value) => JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(value));

// ---- The chat identity kept in this browser (docs/central-chat.md): sealed
// with AES-256-GCM under a non-extractable key, bound to account, epoch and
// tag; IndexedDB "baylee-chat"/"kept", one record. Unchanged from before.

const DB = "baylee-chat";
const STORE = "kept";
const DOMAIN = "baylee-central/browser-chat/v1";
const hex = (b) => Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");
const ownerOf = (o) => `${hex(o.account)}:${o.epoch}:${o.tag}`;
const aad = (o) => new TextEncoder().encode(`${DOMAIN}\0${ownerOf(o)}`);

function database() {
  return new Promise((resolve, reject) => {
    const open = indexedDB.open(DB, 1);
    open.addEventListener("upgradeneeded", () => open.result.createObjectStore(STORE));
    open.addEventListener("success", () => resolve(open.result));
    open.addEventListener("error", () => reject(new Error("unavailable")));
  });
}

async function store(mode, work) {
  const db = await database();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, mode);
    const request = work(tx.objectStore(STORE));
    const failed = () => {
      db.close();
      reject(new Error("unavailable"));
    };
    tx.addEventListener("complete", () => {
      db.close();
      resolve(request?.result);
    });
    tx.addEventListener("error", failed);
    tx.addEventListener("abort", failed);
  });
}

async function keep(owner) {
  const secret = take(chat).keep();
  try {
    const key = await crypto.subtle.generateKey({ name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const sealed = await crypto.subtle.encrypt({ name: "AES-GCM", iv, additionalData: aad(owner) }, key, secret);
    await store("readwrite", (s) => {
      s.clear();
      s.put({ owner: ownerOf(owner), key, iv, sealed }, "identity");
      return null;
    });
  } finally {
    secret.fill(0);
  }
}

async function resume(owner) {
  const record = await store("readonly", (s) => s.get("identity"));
  let secret = null;
  if (record && record.owner === ownerOf(owner)) {
    try {
      secret = new Uint8Array(
        await crypto.subtle.decrypt({ name: "AES-GCM", iv: record.iv, additionalData: aad(owner) }, record.key, record.sealed),
      );
    } catch {
      secret = null;
    }
  }
  if (secret === null) {
    // another account's, altered or unreadable: of no use to anybody
    if (record) await store("readwrite", (s) => (s.clear(), null));
    throw new Error("unavailable");
  }
  try {
    chat = chat_resume(secret, owner.tag);
  } finally {
    secret.fill(0);
  }
}

const unlocked = () => ({ device: chat.device, registration: decode(chat.registration()) });
const owner = (m) => ({ account: bytes(m.account), epoch: m.epoch, tag: m.tag });

async function step(m) {
  switch (m.op) {
    case "enrollment_start":
      enrollment = enrollment_start(String(m.password), bytes(m.account), BigInt(m.epoch));
      return { message: array(enrollment.message) };
    case "enrollment_finish": {
      const state = take(enrollment);
      enrollment = null;
      const done = enrollment_finish(state, bytes(m.response));
      try {
        return {
          record: array(done.record),
          wrapped_master: array(done.wrapped_master),
          recovery: array(done.recovery),
          document: array(done.document),
          recovery_code: done.recovery_code,
        };
      } finally {
        done.free();
      }
    }
    case "login_start":
      if (login !== null || key !== null || chat !== null) throw new Error("failed");
      login = login_start(String(m.password), bytes(m.account), BigInt(m.epoch));
      return { message: array(login.message) };
    case "login_finish": {
      const state = take(login);
      login = null;
      key = login_finish(state, bytes(m.response));
      return { message: array(key.message) };
    }
    case "login_confirm": {
      const state = take(key);
      key = null;
      login_confirm(state, bytes(m.wrapped_master));
      return {};
    }
    case "recovery_create": {
      const state = take(key);
      key = null;
      const made = recovery_create(state, bytes(m.wrapped_master));
      try {
        return { recovery: array(made.recovery), recovery_code: made.recovery_code };
      } finally {
        made.free();
      }
    }
    case "chat_unlock": {
      const state = take(key);
      key = null;
      chat = chat_unlock(state, bytes(m.wrapped_master), m.tag);
      return unlocked();
    }
    case "resume":
      if (login !== null || key !== null || chat !== null) throw new Error("failed");
      await resume(owner(m));
      return unlocked();
    case "unlocked":
      take(chat);
      return unlocked();
    case "keep":
      await keep(owner(m));
      return {};
    case "forget":
      await store("readwrite", (s) => (s.clear(), null));
      return {};
    case "restore":
      take(chat).restore_trust(bytes(m.trust));
      return {};
    case "observe":
      return {
        reviews: m.keys.filter((k) => k.devices.length > 0).map((k) => decode(take(chat).observe(encode(k)))),
        trust: array(take(chat).save_trust()),
      };
    case "confirm":
      take(chat).confirm(encode(m.snapshot));
      return { trust: array(take(chat).save_trust()) };
    case "seal":
      return {
        sealed: decode(take(chat).seal(String(m.conv), String(m.text), false, Uint32Array.from(m.members), BigInt(m.now))),
      };
    case "open":
      return {
        messages: m.messages.map((delivery) => {
          try {
            return { seq: delivery.seq, message: decode(take(chat).open(encode(delivery))) };
          } catch {
            return { seq: delivery.seq, message: null };
          }
        }),
      };
    default:
      throw new Error("failed");
  }
}

let queue = Promise.resolve();
self.addEventListener("message", (event) => {
  let request;
  try {
    request = JSON.parse(String(event.data));
  } catch {
    return;
  }
  if (typeof request !== "object" || request === null || !Number.isInteger(request.id)) return;
  queue = queue.then(async () => {
    let answer;
    try {
      await ready;
      answer = { id: request.id, ok: await step(request) };
    } catch (error) {
      const said = error instanceof Error ? error.message : "";
      answer = { id: request.id, error: ["rejected", "unavailable"].includes(said) ? said : "failed" };
    }
    self.postMessage(JSON.stringify(answer));
  });
});
