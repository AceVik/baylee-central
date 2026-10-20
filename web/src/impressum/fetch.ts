// One round with the feedback service (Baylee's docs/feedback.md §"The
// Impressum"): a challenge, solved in a worker, answered; the blob back. A
// challenge that expired or was answered already (410, 409) is asked again
// once. What the page sees of its own browser goes along (`s`): a browser
// driven by a program, one without languages or with a software renderer
// gets a harder puzzle, never a refusal.

export interface Challenge {
  nonce: string;
  parts: number;
  difficulty: number;
  expires: number;
  sig: string;
}

export function browserBits(): number {
  let bits = 0;
  if (navigator.webdriver) bits |= 1;
  if (navigator.languages.length === 0) bits |= 2;
  try {
    const gl = document.createElement("canvas").getContext("webgl");
    const info = gl?.getExtension("WEBGL_debug_renderer_info");
    const renderer = gl && info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";
    if (/swiftshader|llvmpipe|software/i.test(renderer)) bits |= 4;
  } catch {
    /* no WebGL: nothing to say */
  }
  return bits;
}

export function parseChallenge(raw: unknown): Challenge | null {
  if (typeof raw !== "object" || raw === null) return null;
  const c = raw as Record<string, unknown>;
  const { nonce, parts, difficulty, expires, sig } = c;
  if (typeof nonce !== "string" || nonce.length === 0 || nonce.length > 128 || !/^[A-Za-z0-9_-]+$/.test(nonce)) return null;
  if (typeof parts !== "number" || !Number.isInteger(parts) || parts < 1 || parts > 64) return null;
  if (typeof difficulty !== "number" || !Number.isInteger(difficulty) || difficulty < 1 || difficulty > 32) return null;
  if (typeof expires !== "number" || typeof sig !== "string" || sig.length > 256) return null;
  return { nonce, parts, difficulty, expires, sig };
}

export type Solver = (c: Challenge, onProgress: (p: number) => void) => Promise<number[]>;

/** Solves in a worker (src/impressum/worker.ts). */
export const solveInWorker: Solver = (c, onProgress) =>
  new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
    worker.addEventListener("message", (e: MessageEvent<{ progress?: number; counters?: number[] }>) => {
      if (typeof e.data.progress === "number") {
        onProgress(e.data.progress);
        return;
      }
      worker.terminate();
      if (Array.isArray(e.data.counters)) resolve(e.data.counters);
      else reject(new Error("no answer"));
    });
    worker.addEventListener("error", () => {
      worker.terminate();
      reject(new Error("worker"));
    });
    // oxlint-disable-next-line unicorn/require-post-message-target-origin -- a Worker's postMessage takes no target origin
    worker.postMessage({ nonce: c.nonce, parts: c.parts, difficulty: c.difficulty });
  });

const OPTS: RequestInit = { credentials: "omit", referrerPolicy: "no-referrer", cache: "no-store" };

export async function fetchBlob(base: string, onProgress: (p: number) => void, solver: Solver = solveInWorker, retry = true): Promise<string> {
  const r = await fetch(`${base}/challenge?s=${browserBits()}`, OPTS);
  if (!r.ok) throw new Error(`HTTP ${r.status}`);
  const c = parseChallenge(await r.json());
  if (!c) throw new Error("not a challenge");
  const counters = await solver(c, onProgress);
  const answer = await fetch(base, {
    ...OPTS,
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ nonce: c.nonce, parts: c.parts, difficulty: c.difficulty, expires: c.expires, sig: c.sig, counters }),
  });
  if ((answer.status === 409 || answer.status === 410) && retry) return fetchBlob(base, onProgress, solver, false);
  if (!answer.ok) throw new Error(`HTTP ${answer.status}`);
  return answer.text();
}
