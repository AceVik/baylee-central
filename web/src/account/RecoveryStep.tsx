// The recovery code, shown once (docs/web-accounts.md §"Flows"): copy it,
// download it as a text file, and confirm it was kept before going on.
// Leaving the page before that asks first. Nothing keeps the code: once the
// step ends it is gone from the page too.

import { useEffect, useRef, useState } from "react";

import { Icon } from "../components/icons";
import { T, t, useLang } from "../i18n";

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

export function RecoveryStep({ code, name, onKept }: { code: string; name: string; onKept(): void }) {
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
        <T id="recovery.lead" />
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
      <button type="button" className="button" disabled={!kept} onClick={onKept}>
        <T id="recovery.continue" />
      </button>
    </section>
  );
}
