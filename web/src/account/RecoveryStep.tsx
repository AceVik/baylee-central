// The recovery code, shown once (docs/web-accounts.md §"Flows"): copy it,
// download it as a text file, and confirm it was kept before going on.
// Leaving the page before that asks first. Nothing keeps the code: once the
// step ends it is gone from the page too. A code made after a sign-in, for
// an account moved here without one, can also be put off ("Not now"), which
// throws it away, and is stored only after the confirmation (NewCodeStep).

import { useEffect, useRef, useState } from "react";

import { Icon } from "../components/icons";
import { T, t, useLang, type Key } from "../i18n";

/** Saves `text` as a file named `name` (a download, never a navigation). */
function save(name: string, text: string): void {
  const url = URL.createObjectURL(new Blob([text], { type: "text/plain;charset=utf-8" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.rel = "noopener";
  document.body.append(a);
  a.click();
  a.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function RecoveryStep({
  code,
  name,
  onKept,
  lead = "recovery.lead",
  onLater,
  busy = false,
  status = null,
}: {
  code: string;
  name: string;
  onKept(): void;
  /** What the step says first: the registration's words, or a moved account's. */
  lead?: Key;
  /** "Not now": the code is thrown away. */
  onLater?: (() => void) | undefined;
  /** While the code is being stored: Continue ignores a second press. */
  busy?: boolean;
  /** What storing it came to, when it did not work. */
  status?: { bad: boolean; key: Key } | null;
}) {
  const lang = useLang();
  const [kept, setKept] = useState(false);
  const [copied, setCopied] = useState<"yes" | "no" | null>(null);
  const heading = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    heading.current?.focus();
    // the code is shown only now: leaving before it is kept asks first
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, []);

  const done = (ok: boolean) => setCopied(ok ? "yes" : "no");
  const copy = () => {
    if (navigator.clipboard === undefined) done(false);
    else navigator.clipboard.writeText(code).then(() => done(true), () => done(false));
  };

  return (
    <section className="recovery glass" aria-labelledby="recovery-title">
      <h2 id="recovery-title" ref={heading} tabIndex={-1}>
        <T id="recovery.title" />
      </h2>
      <p>
        <T id={lead} />
      </p>
      <p className="recovery-code" id="recovery-code" aria-label={t(lang, "recovery.label")}>
        <code translate="no">{code}</code>
      </p>
      <div className="links-row">
        <button type="button" className="button button-ghost" onClick={copy}>
          <Icon name="copy" className="button-icon" />
          <T id="recovery.copy" />
        </button>
        <button type="button" className="button button-ghost" onClick={() => save(t(lang, "recovery.fileName"), `${t(lang, "recovery.file", { name, code })}\n`)}>
          <Icon name="download" className="button-icon" />
          <T id="recovery.download" />
        </button>
      </div>
      <output className="form-status">
        {copied !== null && <T id={copied === "yes" ? "recovery.copied" : "recovery.copyFailed"} />}
      </output>
      <p className="small">
        <T id="recovery.keep" />
      </p>
      <p className="field">
        <label className="check-label">
          <input type="checkbox" checked={kept} onChange={(e) => setKept(e.target.checked)} />
          <span>
            <T id="recovery.confirm" />
          </span>
        </label>
      </p>
      <div className="form-row">
        {/* while storing, never `disabled`, which would drop the focus; a second press is ignored */}
        <button type="button" className="button" disabled={!kept} aria-disabled={busy ? true : undefined} onClick={busy ? undefined : onKept}>
          <T id="recovery.continue" />
        </button>
        {onLater !== undefined && (
          <button type="button" className="button button-ghost" onClick={onLater}>
            <T id="recovery.later" />
          </button>
        )}
        <output className={`form-status${status?.bad ? " bad" : ""}`}>{status !== null && <T id={status.key} />}</output>
      </div>
    </section>
  );
}
