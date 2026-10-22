// A recovery code made after a sign-in, for an account moved here from the
// official gateway without one (docs/web-accounts.md §"A recovery code after
// a sign-in"): shown with RecoveryStep, and stored (`SetRecovery`) only once
// the person says they kept it. "Not now" throws it away and nothing reaches
// Central. The page holds the 73 bytes until Central answers, so pressing
// Continue again after a lost answer sends the same bytes, which Central
// takes again; "Not now" is gone once a request may have stored the code. A
// code Central refuses does not count, and the step says so.

import { useState } from "react";

import { T, type Key } from "../i18n";
import { FAILURE } from "./errors";
import { FlowError, storeCode, type NewCode } from "./flows";
import { RecoveryStep } from "./RecoveryStep";
import { useAccount } from "./session";

export function NewCodeStep({ code, name, onDone }: { code: NewCode; name: string; onDone(stored: boolean): void }) {
  const { api } = useAccount();
  const [busy, setBusy] = useState(false);
  /** A request was sent: Central may hold the code, so it is no longer put off. */
  const [sent, setSent] = useState(false);
  /** A request got no answer: Central may hold the code without having said so. */
  const [unsure, setUnsure] = useState(false);
  const [status, setStatus] = useState<{ bad: boolean; key: Key } | null>(null);
  /** Central refused the code: it does not count, and is not sent again. */
  const [refused, setRefused] = useState<Key | null>(null);

  const kept = () => {
    if (busy) return;
    setBusy(true);
    setSent(true);
    setStatus({ bad: false, key: "recovery.saving" });
    storeCode(code, { api }).then(
      () => onDone(true),
      (error: unknown) => {
        setBusy(false);
        const kind = error instanceof FlowError ? error.kind : "unreachable";
        if (kind === "codeRefused" || kind === "codeTaken") {
          setStatus(null);
          // a refusal after a lost answer may mean the session ended with the code stored
          setRefused(kind === "codeRefused" && unsure ? "recovery.errUnsure" : FAILURE[kind]);
        } else {
          if (kind !== "busy") setUnsure(true);
          setStatus({ bad: true, key: kind === "busy" ? "acct.errBusy" : "recovery.errSave" });
        }
      },
    );
  };

  if (refused !== null)
    return (
      <section className="recovery glass" aria-labelledby="recovery-refused">
        <h2 id="recovery-refused">
          <T id="recovery.title" />
        </h2>
        <p className="account-note bad" role="alert">
          <T id={refused} />
        </p>
        <button type="button" className="button" onClick={() => onDone(false)}>
          <T id="recovery.continue" />
        </button>
      </section>
    );
  return (
    <RecoveryStep
      code={code.code}
      name={name}
      lead="recovery.leadMoved"
      busy={busy}
      status={status}
      onKept={kept}
      onLater={sent ? undefined : () => onDone(false)}
    />
  );
}
