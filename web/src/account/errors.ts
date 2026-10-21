// What the account pages say when a flow stops.

import type { Key } from "../i18n";
import type { FlowFailure } from "./flows";

export const FAILURE: Readonly<Record<FlowFailure, Key>> = {
  badKey: "register.errBadKey",
  taken: "register.errTaken",
  wrongLogin: "signin.errWrong",
  busy: "acct.errBusy",
  unreachable: "acct.errUnreachable",
  crypto: "acct.errCrypto",
  mismatch: "acct.errMismatch",
};
