/* tslint:disable */
/* eslint-disable */

/**
 * Chat cryptography retained only for the lifetime of the unlocked worker.
 */
export class Chat {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Confirm exactly the public snapshot displayed to the person.
     *
     * # Errors
     * Unknown peers, malformed or stale confirmations.
     */
    confirm(keys: Uint8Array): void;
    /**
     * Validate a peer's keys and return its safety-number review as JSON.
     *
     * # Errors
     * Malformed, duplicated or substituted own-account keys.
     */
    observe(keys: Uint8Array): Uint8Array;
    /**
     * Authenticate and open a delivery for display.
     *
     * # Errors
     * Malformed, forged, misaddressed or unconfirmed messages.
     */
    open(delivery: Uint8Array): Uint8Array;
    /**
     * Public signed registration as JSON bytes.
     *
     * # Errors
     * Invalid binding or encoding.
     */
    registration(): Uint8Array;
    /**
     * Restore an authenticated account-bound trust state before reading new keys.
     *
     * # Errors
     * Wrong account, corruption, duplicates or size overflow.
     */
    restore_trust(record: Uint8Array): void;
    /**
     * Authenticated ciphertext for the browser's peer-key trust state.
     *
     * # Errors
     * Randomness, encoding or encryption failure.
     */
    save_trust(): Uint8Array;
    /**
     * Seal one message with fresh randomness and a new correlation ID.
     *
     * # Errors
     * Invalid text/membership, changed keys or randomness failure.
     */
    seal(conv: string, text: string, emote: boolean, members: Uint32Array, now: bigint): Uint8Array;
    /**
     * Stable public device fingerprint.
     */
    readonly device: string;
}

/**
 * A finished registration: what `EnrollmentFinish` sends, and the
 * recovery code to show once. Free it once read; that wipes the code.
 */
export class Enrolled {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The initial account document, sealed.
     */
    readonly document: Uint8Array;
    /**
     * The OPAQUE registration upload.
     */
    readonly record: Uint8Array;
    /**
     * The master key under the recovery code (73 bytes).
     */
    readonly recovery: Uint8Array;
    /**
     * The recovery code's written form. Read from the wiped buffer rather
     * than handed over, so no copy stays behind in the module's memory.
     */
    readonly recovery_code: string;
    /**
     * The master key under the password (73 bytes).
     */
    readonly wrapped_master: Uint8Array;
}

/**
 * A registration under way. `message` is the OPAQUE registration request
 * for `EnrollmentStart`.
 */
export class Enrollment {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The OPAQUE registration request.
     */
    readonly message: Uint8Array;
}

/**
 * A sign-in under way. `message` is KE1 for `LoginStart`.
 */
export class Login {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * KE1.
     */
    readonly message: Uint8Array;
}

/**
 * A sign-in whose KE3 is ready: `message` goes to `LoginFinish`, and the
 * handle checks the master key that answer brings.
 */
export class LoginKey {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * KE3.
     */
    readonly message: Uint8Array;
}

/**
 * A recovery code made after a sign-in: what `SetRecovery` sends, and the
 * code to show once. Free it once read; that wipes the code.
 */
export class Recovered {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The master key under the recovery code (73 bytes).
     */
    readonly recovery: Uint8Array;
    /**
     * The recovery code's written form. Read from the wiped buffer rather
     * than handed over, so no copy stays behind in the module's memory.
     */
    readonly recovery_code: string;
}

/**
 * The origin of the Central this module is bound to: the live one.
 */
export function central_origin(): string;

/**
 * Consume an authenticated sign-in and derive the account's chat identity.
 *
 * # Errors
 * Rejects an invalid wrapper, identity or public tag.
 */
export function chat_unlock(state: LoginKey, wrapped_master: Uint8Array, tag: number): Chat;

/**
 * Finish registering with `EnrollmentChallenge`'s message.
 *
 * # Errors
 *
 * `rejected` for a malformed response, `unavailable` without randomness.
 */
export function enrollment_finish(state: Enrollment, response: Uint8Array): Enrolled;

/**
 * Start registering: the `identity` of `Register`'s answer and the password.
 *
 * # Errors
 *
 * `rejected` for an empty or over-long password, an account that is not a
 * `UUIDv7` or epoch zero.
 */
export function enrollment_start(password: string, account: Uint8Array, epoch: bigint): Enrollment;

/**
 * Check that `BrowserSession`'s `wrapped_master` opens, and keep nothing.
 *
 * # Errors
 *
 * `rejected` when it is malformed or does not open.
 */
export function login_confirm(state: LoginKey, wrapped_master: Uint8Array): void;

/**
 * Answer `LoginChallenge`'s KE2. A wrong password is `rejected` here.
 *
 * # Errors
 *
 * `rejected` for a wrong password or a malformed or foreign response.
 */
export function login_finish(state: Login, response: Uint8Array): LoginKey;

/**
 * Start signing in: `Discover`'s identity and the password.
 *
 * # Errors
 *
 * As [`enrollment_start`].
 */
export function login_start(password: string, account: Uint8Array, epoch: bigint): Login;

/**
 * In place of [`login_confirm`], for an account without a recovery code:
 * open `BrowserSession`'s `wrapped_master` and wrap the same master key
 * under a new recovery code, keeping no key.
 *
 * # Errors
 *
 * `rejected` when the wrapper is malformed or does not open, `unavailable`
 * without randomness.
 */
export function recovery_create(state: LoginKey, wrapped_master: Uint8Array): Recovered;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_chat_free: (a: number, b: number) => void;
    readonly __wbg_enrolled_free: (a: number, b: number) => void;
    readonly __wbg_enrollment_free: (a: number, b: number) => void;
    readonly __wbg_login_free: (a: number, b: number) => void;
    readonly __wbg_loginkey_free: (a: number, b: number) => void;
    readonly __wbg_recovered_free: (a: number, b: number) => void;
    readonly central_origin: () => [number, number];
    readonly chat_confirm: (a: number, b: number, c: number) => [number, number];
    readonly chat_device: (a: number) => [number, number];
    readonly chat_observe: (a: number, b: number, c: number) => [number, number, number, number];
    readonly chat_open: (a: number, b: number, c: number) => [number, number, number, number];
    readonly chat_registration: (a: number) => [number, number, number, number];
    readonly chat_restore_trust: (a: number, b: number, c: number) => [number, number];
    readonly chat_save_trust: (a: number) => [number, number, number, number];
    readonly chat_seal: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: bigint) => [number, number, number, number];
    readonly chat_unlock: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly enrolled_document: (a: number) => [number, number];
    readonly enrolled_record: (a: number) => [number, number];
    readonly enrolled_recovery: (a: number) => [number, number];
    readonly enrolled_recovery_code: (a: number) => any;
    readonly enrolled_wrapped_master: (a: number) => [number, number];
    readonly enrollment_finish: (a: number, b: number, c: number) => [number, number, number];
    readonly enrollment_message: (a: number) => [number, number];
    readonly enrollment_start: (a: number, b: number, c: number, d: number, e: bigint) => [number, number, number];
    readonly login_confirm: (a: number, b: number, c: number) => [number, number];
    readonly login_finish: (a: number, b: number, c: number) => [number, number, number];
    readonly login_message: (a: number) => [number, number];
    readonly login_start: (a: number, b: number, c: number, d: number, e: bigint) => [number, number, number];
    readonly loginkey_message: (a: number) => [number, number];
    readonly recovered_recovery: (a: number) => [number, number];
    readonly recovered_recovery_code: (a: number) => any;
    readonly recovery_create: (a: number, b: number, c: number) => [number, number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
