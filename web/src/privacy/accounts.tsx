// The notice's account part (docs/web-accounts.md §"Web pages"): what
// Central keeps for an account, the session cookie and what the browser
// remembers of it, and the counters against guessing, kept in memory only.
// No third party receives any of it.

import type { Part } from "./part";

export const ACCOUNTS: Part[] = [
  { id: "account", title: "privacy.accountTitle", body: ["privacy.account"], basis: "privacy.accountBasis" },
  { id: "cookie", title: "privacy.cookieTitle", body: ["privacy.cookie"], basis: "privacy.cookieBasis" },
  { id: "counters", title: "privacy.countersTitle", body: ["privacy.counters"], basis: "privacy.countersBasis" },
];
