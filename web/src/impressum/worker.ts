// The puzzle in a worker, so the page stays responsive while it is solved.
// A file of its own (bundled under assets/): the CSP allows no blob: or
// inline worker.

import { solve } from "./pow";

interface Scope {
  postMessage(message: unknown): void;
  addEventListener(type: "message", listener: (e: MessageEvent<unknown>) => void): void;
}

const scope = self as unknown as Scope;

/* oxlint-disable unicorn/require-post-message-target-origin -- a worker's postMessage takes no target origin */
scope.addEventListener("message", (e) => {
  const m = (e.data ?? {}) as { nonce?: unknown; parts?: unknown; difficulty?: unknown };
  const parts = typeof m.parts === "number" ? m.parts | 0 : 0;
  const difficulty = typeof m.difficulty === "number" ? m.difficulty | 0 : 0;
  const counters = solve(String(m.nonce), parts, difficulty, (done, all) => scope.postMessage({ progress: done / all }));
  scope.postMessage({ counters });
});
