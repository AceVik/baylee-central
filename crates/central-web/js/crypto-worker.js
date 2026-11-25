// GENERATED from crates/central-web/ts/crypto-worker.ts via TypeScript 7.1 — do not edit directly.
// The browser crypto in a module Worker (docs/web-accounts.md §"Browser
// crypto"): central-web-crypto's committed WebAssembly module, unchanged,
// with its Argon2 (64 MiB, three passes) off the page's thread. A Worker
// needs a script of its own; this is the whole of it, everything the page
// does is Rust (crates/central-web/src/islands).
//
// Protocol: the page posts JSON `{id, op, ...}`; the Worker answers JSON
// `{id, ok: {...}}` or `{id, error}` with error one of "rejected",
// "unavailable", "failed". Bytes travel as arrays of numbers, as on
// Central's wire. Requests are answered in order.
//
// `cargo xtask site` writes the hashed addresses of the glue and the module
// over "./central_web_crypto.js" and "./central_web_crypto_bg.wasm".
import init, { chat_resume, chat_unlock, enrollment_finish, enrollment_start, login_confirm, login_finish, login_start, password_finish, password_start, recovery_create, reset_keep, reset_new, reset_start, work_start, } from "./central_web_crypto.js";
const workerSelf = self;
const ready = init({ module_or_path: "./central_web_crypto_bg.wasm" });
let enrollment = null;
let login = null;
let key = null;
let chat = null;
let work = null;
let reset = null;
let change = null;
const bytes = (value) => {
    if (!Array.isArray(value) || value.length > 1 << 20 || !value.every((b) => Number.isInteger(b) && b >= 0 && b <= 255)) {
        throw new Error("failed");
    }
    return Uint8Array.from(value);
};
const array = (value) => Array.from(value);
const take = (state) => {
    if (state === null)
        throw new Error("failed");
    return state;
};
const encode = (value) => new TextEncoder().encode(JSON.stringify(value));
const decode = (value) => JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(value));
const toArrayBufferBytes = (src) => {
    const copy = new Uint8Array(src.byteLength);
    copy.set(src);
    return copy;
};
// ---- The chat identity kept in this browser (docs/central-chat.md): sealed
// with AES-256-GCM under a non-extractable key, bound to account, epoch and
// tag; IndexedDB "baylee-chat"/"kept", one record. Unchanged from before.
const DB = "baylee-chat";
const STORE = "kept";
const DOMAIN = "baylee-central/browser-chat/v1";
const hex = (b) => Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");
const ownerOf = (o) => `${hex(o.account)}:${String(o.epoch)}:${String(o.tag)}`;
const aad = (o) => toArrayBufferBytes(new TextEncoder().encode(`${DOMAIN}\0${ownerOf(o)}`));
function database() {
    return new Promise((resolve, reject) => {
        const open = indexedDB.open(DB, 1);
        open.addEventListener("upgradeneeded", () => open.result.createObjectStore(STORE));
        open.addEventListener("success", () => resolve(open.result));
        open.addEventListener("error", () => reject(new Error("unavailable")));
    });
}
async function store(mode, fn) {
    const db = await database();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(STORE, mode);
        const request = fn(tx.objectStore(STORE));
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
async function keep(ownerInfo) {
    const secret = take(chat).keep();
    const plain = toArrayBufferBytes(secret);
    try {
        const aesKey = await crypto.subtle.generateKey({ name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
        const iv = crypto.getRandomValues(new Uint8Array(12));
        const sealed = await crypto.subtle.encrypt({ name: "AES-GCM", iv, additionalData: aad(ownerInfo) }, aesKey, plain);
        await store("readwrite", (s) => {
            s.clear();
            s.put({ owner: ownerOf(ownerInfo), key: aesKey, iv, sealed }, "identity");
            return null;
        });
    }
    finally {
        plain.fill(0);
        secret.fill(0);
    }
}
async function resume(ownerInfo) {
    const record = (await store("readonly", (s) => s.get("identity"))) ?? null;
    let secret = null;
    if (record && record.owner === ownerOf(ownerInfo)) {
        try {
            secret = new Uint8Array(await crypto.subtle.decrypt({ name: "AES-GCM", iv: toArrayBufferBytes(record.iv), additionalData: aad(ownerInfo) }, record.key, record.sealed));
        }
        catch {
            secret = null;
        }
    }
    if (secret === null) {
        // another account's, altered or unreadable: of no use to anybody
        if (record) {
            await store("readwrite", (s) => {
                s.clear();
                return null;
            });
        }
        throw new Error("unavailable");
    }
    try {
        chat = chat_resume(secret, ownerInfo.tag);
    }
    finally {
        secret.fill(0);
    }
}
const count = (value, most) => {
    if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0 || value > most)
        throw new Error("failed");
    return value;
};
const rest = () => {
    if (work !== null)
        work.free();
    work = null;
};
const unlocked = () => {
    const active = take(chat);
    return { device: active.device, registration: decode(active.registration()) };
};
const owner = (m) => ({
    account: bytes(m.account),
    epoch: m.epoch,
    tag: Number(m.tag),
});
async function step(m) {
    switch (m.op) {
        case "work_start":
            rest();
            work = work_start(String(m.nonce), count(m.memory, 0xffffffff), count(m.passes, 0xffffffff), count(m.parts, 255), count(m.difficulty, 255));
            return {};
        case "work_search": {
            const found = take(work).search(count(m.part, 255), BigInt(count(m.from, Number.MAX_SAFE_INTEGER)), count(m.tries, 0xffffffff));
            return { counter: found === undefined ? null : Number(found) };
        }
        case "work_end":
            rest();
            return {};
        case "enrollment_start":
            enrollment = enrollment_start(String(m.password), bytes(m.account), BigInt(Number(m.epoch)));
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
            }
            finally {
                done.free();
            }
        }
        case "login_start":
            if (login !== null || key !== null || chat !== null)
                throw new Error("failed");
            login = login_start(String(m.password), bytes(m.account), BigInt(Number(m.epoch)));
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
            }
            finally {
                made.free();
            }
        }
        case "reset_start":
            if (reset !== null)
                throw new Error("failed");
            reset = reset_start(String(m.password), bytes(m.account), BigInt(Number(m.epoch)), m.recovery_code === null ? undefined : String(m.recovery_code));
            return { message: array(reset.message) };
        case "reset_keep": {
            const state = take(reset);
            reset = null;
            const done = reset_keep(state, bytes(m.response), bytes(m.recovery));
            try {
                return { record: array(done.record), wrapped_master: array(done.wrapped_master) };
            }
            finally {
                done.free();
            }
        }
        case "reset_new": {
            const state = take(reset);
            reset = null;
            const done = reset_new(state, bytes(m.response));
            try {
                return {
                    record: array(done.record),
                    wrapped_master: array(done.wrapped_master),
                    recovery: array(done.recovery),
                    document: array(done.document),
                    recovery_code: done.recovery_code,
                };
            }
            finally {
                done.free();
            }
        }
        case "password_start": {
            const state = take(key);
            key = null;
            change = password_start(state, bytes(m.wrapped_master), String(m.current), String(m.new));
            return { login: array(change.login), registration: array(change.registration) };
        }
        case "password_finish": {
            const state = take(change);
            change = null;
            const done = password_finish(state, bytes(m.challenge), bytes(m.registration));
            try {
                return { proof: array(done.proof), record: array(done.record), wrapped_master: array(done.wrapped_master) };
            }
            finally {
                done.free();
            }
        }
        case "chat_unlock": {
            const state = take(key);
            key = null;
            chat = chat_unlock(state, bytes(m.wrapped_master), Number(m.tag));
            return unlocked();
        }
        case "resume":
            if (login !== null || key !== null || chat !== null)
                throw new Error("failed");
            await resume(owner(m));
            return unlocked();
        case "unlocked":
            take(chat);
            return unlocked();
        case "keep":
            await keep(owner(m));
            return {};
        case "forget":
            await store("readwrite", (s) => {
                s.clear();
                return null;
            });
            return {};
        case "restore":
            take(chat).restore_trust(bytes(m.trust));
            return {};
        case "observe": {
            const entries = Array.isArray(m.keys) ? m.keys : [];
            return {
                reviews: entries.filter((k) => k.devices.length > 0).map((k) => decode(take(chat).observe(encode(k)))),
                trust: array(take(chat).save_trust()),
            };
        }
        case "confirm":
            take(chat).confirm(encode(m.snapshot));
            return { trust: array(take(chat).save_trust()) };
        case "seal": {
            const members = Array.isArray(m.members) ? m.members : [];
            return {
                sealed: decode(take(chat).seal(String(m.conv), String(m.text), false, Uint32Array.from(members), BigInt(Number(m.now)))),
            };
        }
        case "open": {
            const list = Array.isArray(m.messages) ? m.messages : [];
            return {
                messages: list.map((delivery) => {
                    try {
                        return { seq: delivery.seq, message: decode(take(chat).open(encode(delivery))) };
                    }
                    catch {
                        return { seq: delivery.seq, message: null };
                    }
                }),
            };
        }
        default:
            throw new Error("failed");
    }
}
let queue = Promise.resolve();
workerSelf.addEventListener("message", (event) => {
    let parsed;
    try {
        parsed = JSON.parse(String(event.data));
    }
    catch {
        return;
    }
    if (typeof parsed !== "object" || parsed === null)
        return;
    const request = parsed;
    if (typeof request.id !== "number" || !Number.isInteger(request.id))
        return;
    const requestId = request.id;
    queue = queue.then(async () => {
        let answer;
        try {
            await ready;
            answer = { id: requestId, ok: await step(request) };
        }
        catch (error) {
            const said = error instanceof Error ? error.message : "";
            answer = { id: requestId, error: ["rejected", "unavailable"].includes(said) ? said : "failed" };
        }
        workerSelf.postMessage(JSON.stringify(answer));
    });
});
