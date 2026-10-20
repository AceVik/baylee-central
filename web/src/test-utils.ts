// A stand-in for the network in component tests: `fetch` answers from a
// handler, every call is kept for the test to read, and anything unanswered
// is a 404. GitHub's API, Central, the feedback service and the card status
// are all answered here; no test reaches a real service.

import { vi } from "vitest";

export interface Call {
  method: string;
  url: string;
  body: unknown;
  init: RequestInit | undefined;
}

type Handler = (call: Call) => { status?: number; body?: unknown; text?: string } | undefined;

export function serve(handler: Handler): Call[] {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: string, init?: RequestInit) => {
      const call: Call = {
        method: init?.method ?? "GET",
        url: input,
        body: typeof init?.body === "string" ? JSON.parse(init.body) : undefined,
        init,
      };
      calls.push(call);
      const answer = handler(call) ?? { status: 404, text: "not found\n" };
      const status = answer.status ?? 200;
      const body = answer.text ?? JSON.stringify(answer.body ?? {});
      return new Response(status === 204 ? null : body, { status });
    }),
  );
  return calls;
}
