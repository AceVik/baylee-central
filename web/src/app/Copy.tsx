// Copying a value with one press, and an address that breaks where a reader
// expects it. The copy button is a quiet round icon with its name in words;
// for a moment it shows a check, and the live words say what happened, also
// when the same thing is copied twice.

import { Fragment, useEffect, useState } from "react";

import { Icon } from "../components/icons";
import { t, useLang } from "../i18n";

/** How long the button shows that it worked. */
const COPIED_MS = 1600;

export function CopyButton({ text, label }: { text: string; label: string }) {
  const lang = useLang();
  const [copied, setCopied] = useState(false);
  const [said, setSaid] = useState("");
  useEffect(() => {
    if (!copied) return;
    const back = window.setTimeout(() => setCopied(false), COPIED_MS);
    return () => window.clearTimeout(back);
  }, [copied]);
  const copy = () => {
    if (!navigator.clipboard?.writeText) {
      setSaid(t(lang, "copy.impossible"));
      return;
    }
    navigator.clipboard.writeText(text).then(
      () => {
        setCopied(true);
        // a different string each time, so a second copy is announced again
        setSaid((now) => (now === t(lang, "copy.done") ? `${t(lang, "copy.done")} ` : t(lang, "copy.done")));
      },
      () => setSaid(t(lang, "copy.failed")),
    );
  };
  return (
    <>
      <button type="button" className="round-btn copy-btn" onClick={copy} title={label} data-copied={copied ? "" : undefined}>
        <Icon name={copied ? "check" : "copy"} />
        <span className="vh">{label}</span>
      </button>
      <output className="vh">{said}</output>
    </>
  );
}

/** An origin as code, breakable after `//` and before each dot, so a long one wraps between its names, not inside one. */
export function Address({ origin }: { origin: string }) {
  const [scheme, rest] = origin.includes("://") ? origin.split("://", 2) : ["", origin];
  const parts = (rest ?? "").split(".");
  return (
    <code translate="no" className="address">
      {scheme !== "" && `${scheme}://`}
      {scheme !== "" && <wbr />}
      {parts.map((part, i) => (
        <Fragment key={i}>
          {i > 0 && <wbr />}
          {i > 0 && "."}
          {part}
        </Fragment>
      ))}
    </code>
  );
}
