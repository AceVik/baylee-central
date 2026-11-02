import { expect, it } from "vitest";
import { trustStore } from "../../src/chat/storage";
import { ACCOUNT } from "../support/central";

it("keeps only ciphertext in an account-specific storage slot and rejects malformed data", () => {
  const first = trustStore(ACCOUNT);
  const other = trustStore(new Uint8Array(16).fill(8));
  const bytes = new Uint8Array([0, 255, 31, 62, 0]);
  expect(first.read()).toBeNull(); first.write(bytes);
  expect(first.read()).toEqual(bytes); expect(other.read()).toBeNull();
  const key = Object.keys(localStorage).find((k) => k.startsWith("baylee-chat-trust:"));
  expect(key).toBeDefined(); localStorage.setItem(key ?? "", "not base64!");
  expect(() => first.read()).toThrow("trust");
});
