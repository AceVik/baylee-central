// Accepting Central's terms after signing in (`TermsStatus`, then
// `AcceptTerms`; docs/terms.md). The page shows the texts Central serves
// (`Terms`) in its language, and accepts only when those texts hash to the
// digest of the version in force: an acceptance names exactly what was
// shown. While no version is in force (today: `draft`) there is nothing to
// accept and the step ends at once.

import { useCallback, useEffect, useRef, useState } from "react";

import { T, useLang, type Key } from "../i18n";
import { LegalText } from "../legal/LegalText";
import { ApiError, type TermsDocument } from "./api";
import { equalBytes, termsDigest } from "./digest";
import { useAccount } from "./session";

type Stage = { at: "checking" } | { at: "shown"; terms: TermsDocument } | { at: "failed"; key: Key } | { at: "done" };

export function TermsStep({ onDone }: { onDone(): void }) {
  const lang = useLang();
  const { api, signOut } = useAccount();
  const [stage, setStage] = useState<Stage>({ at: "checking" });
  const [accepting, setAccepting] = useState(false);
  const heading = useRef<HTMLHeadingElement>(null);
  const live = useRef(true);
  // the latest `onDone`, so a parent's new function starts no new check
  const done = useRef(onDone);
  useEffect(() => {
    done.current = onDone;
  });
  useEffect(() => {
    live.current = true;
    return () => {
      live.current = false;
    };
  }, []);

  const finish = useCallback(() => {
    setStage({ at: "done" });
    done.current();
  }, []);

  /** The step's next stage, from Central's answers. */
  const next = useCallback(async (): Promise<Stage> => {
    const status = await api.termsStatus();
    const current = status.current;
    if (status.accepted || current === null) return { at: "done" };
    const terms = await api.terms();
    // what Central serves must be the version it asks for, and hash to its digest
    const same = terms !== null && terms.version.version === current.version && equalBytes(terms.version.digest, current.digest);
    if (!same || !equalBytes(await termsDigest(terms.texts), terms.version.digest)) return { at: "failed", key: "accept.mismatch" };
    return { at: "shown", terms };
  }, [api]);

  const load = useCallback(
    () =>
      next().then(
        (found) => {
          if (!live.current) return;
          if (found.at === "done") finish();
          else setStage(found);
        },
        (error: unknown) => {
          if (!live.current) return;
          if (error instanceof ApiError && error.kind === "rejected") {
            // the session ended meanwhile
            void signOut();
            setStage({ at: "failed", key: "accept.signedOut" });
          } else setStage({ at: "failed", key: error instanceof ApiError && error.kind === "busy" ? "acct.errBusy" : "acct.errUnreachable" });
        },
      ),
    [next, finish, signOut],
  );

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (stage.at === "shown") heading.current?.focus();
  }, [stage.at]);

  const again = () => {
    setStage({ at: "checking" });
    void load();
  };

  if (stage.at === "done") return null;
  if (stage.at === "checking") {
    return (
      <output className="form-status">
        <T id="accept.loading" />
      </output>
    );
  }
  if (stage.at === "failed") {
    return (
      <div className="account-note">
        <p className="form-status bad" role="alert">
          <T id={stage.key} />
        </p>
        <button type="button" className="button button-ghost" onClick={again}>
          <T id="accept.retry" />
        </button>
      </div>
    );
  }

  const { terms } = stage;
  const text = terms.texts.find((x) => x.lang === lang) ?? terms.texts.find((x) => x.lang === "en") ?? terms.texts[0];
  const accept = () => {
    if (accepting) return;
    setAccepting(true);
    api.acceptTerms(terms.version).then(finish, (error: unknown) => {
      setAccepting(false);
      // another version came into force meanwhile: show that one
      if (error instanceof ApiError && error.kind === "conflict") again();
      else setStage({ at: "failed", key: "acct.errUnreachable" });
    });
  };
  return (
    <section className="accept" aria-labelledby="accept-title">
      <h2 id="accept-title" ref={heading} tabIndex={-1}>
        <T id="accept.title" />
      </h2>
      <p>
        <T id="accept.lead" args={{ version: terms.version.version }} />
      </p>
      <div className="accept-text legal" lang={text?.lang}>
        {text !== undefined && <LegalText text={text.markdown} />}
      </div>
      <div className="links-row">
        <button type="button" className="button" aria-disabled={accepting ? true : undefined} onClick={accept}>
          <T id="accept.accept" />
        </button>
        <button type="button" className="button button-ghost" onClick={() => void signOut()}>
          <T id="accept.later" />
        </button>
      </div>
    </section>
  );
}
