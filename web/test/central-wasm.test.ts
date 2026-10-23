// The committed browser crypto (src/central-wasm/README.md): its files are
// the ones SHA256SUMS names, and the module itself runs: bound to the live
// Central, its messages the sizes of the fixed suite, its states single-use,
// its errors only `rejected`. The full exchange against a server is
// crates/central-web-crypto/tests/flow.rs (Rust) and, against a real local
// Central, the account pages' run in docs/web-accounts.md.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { beforeAll, describe, expect, it } from "vitest";

import {
  central_origin,
  enrollment_finish,
  enrollment_start,
  initSync,
  login_confirm,
  login_finish,
  login_start,
  recovery_create,
} from "../src/central-wasm/central_web_crypto.js";

// relative to web/, where vitest runs (as test/boot.test.ts reads public/)
const file = (name: string) => readFileSync(resolve("src/central-wasm", name));
const ACCOUNT = Uint8Array.from([
  0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d,
]);
const PASSWORD = "synthetic test-only password";

describe("the committed files", () => {
  it("are the ones SHA256SUMS names, each with its hash", () => {
    const lines = file("SHA256SUMS").toString("utf8").trimEnd().split("\n");
    expect(lines.map((l) => l.split("  ")[1])).toEqual([
      "central_web_crypto.d.ts",
      "central_web_crypto.js",
      "central_web_crypto_bg.wasm",
      "central_web_crypto_bg.wasm.d.ts",
    ]);
    for (const line of lines) {
      const [hash = "", name = ""] = line.split("  ");
      const actual = createHash("sha256")
        .update(file(name))
        .digest("hex");
      expect({ name, sha256: actual }).toEqual({ name, sha256: hash });
    }
  });
});

describe("the module", () => {
  beforeAll(() => {
    initSync({ module: file("central_web_crypto_bg.wasm") });
  });

  it("is bound to the live Central", () => {
    expect(central_origin()).toBe("https://central.baylee.app");
  });

  it("starts a registration and a sign-in with the suite's message sizes, fresh each time", () => {
    const first = enrollment_start(PASSWORD, ACCOUNT, 1n);
    const second = enrollment_start(PASSWORD, ACCOUNT, 1n);
    expect(first.message).toHaveLength(32);
    expect(first.message).not.toEqual(second.message);
    first.free();
    second.free();
    const login = login_start(PASSWORD, ACCOUNT, 1n);
    expect(login.message).toHaveLength(96);
    login.free();
  });

  it("refuses an account that is not a UUIDv7, epoch zero and an empty or over-long password", () => {
    const v4 = ACCOUNT.slice();
    v4[6] = 0x4a;
    for (const start of [enrollment_start, login_start]) {
      expect(() => start(PASSWORD, ACCOUNT.slice(0, 15), 1n)).toThrow("rejected");
      expect(() => start(PASSWORD, v4, 1n)).toThrow("rejected");
      expect(() => start(PASSWORD, ACCOUNT, 0n)).toThrow("rejected");
      expect(() => start("", ACCOUNT, 1n)).toThrow("rejected");
      expect(() => start("x".repeat(1025), ACCOUNT, 1n)).toThrow("rejected");
    }
  });

  it("rejects a malformed answer and spends the state doing so", () => {
    const registering = enrollment_start(PASSWORD, ACCOUNT, 1n);
    expect(() => enrollment_finish(registering, new Uint8Array(64))).toThrow("rejected");
    // the handle was consumed: it cannot be finished again
    expect(() => enrollment_finish(registering, new Uint8Array(64))).toThrow("null pointer passed to rust");
    const signingIn = login_start(PASSWORD, ACCOUNT, 1n);
    expect(() => login_finish(signingIn, new Uint8Array(320))).toThrow("rejected");
    expect(() => login_finish(signingIn, new Uint8Array(320))).toThrow("null pointer passed to rust");
    expect(() => login_finish(login_start(PASSWORD, ACCOUNT, 1n), new Uint8Array(319))).toThrow("rejected");
    expect(login_confirm).toBeTypeOf("function");
  });

  it("makes a recovery code only from a finished sign-in's state", () => {
    // Both steps that end a sign-in take its LoginKey, which only a real KE2
    // makes (the exchange is crates/central-web-crypto/tests/flow.rs); any
    // other handle is refused before a key is touched.
    expect(recovery_create).toBeTypeOf("function");
    const signingIn = login_start(PASSWORD, ACCOUNT, 1n);
    // (the two classes have the same shape to TypeScript; the glue checks the class)
    expect(() => recovery_create(signingIn, new Uint8Array(73))).toThrow("expected instance of LoginKey");
    // @ts-expect-error -- nor is a plain object
    expect(() => recovery_create({}, new Uint8Array(73))).toThrow("expected instance of LoginKey");
    signingIn.free();
  });
});
