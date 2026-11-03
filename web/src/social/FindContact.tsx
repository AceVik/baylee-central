import { useEffect, useRef, useState, type FormEvent } from "react";
import { T, t, useLang, type Key } from "../i18n";
import { Avatar } from "../chat/pieces";
import { handleOf, tagOf, type Person, type SocialApi } from "./api";

/** Adding a contact by the exact public handle or tag: a lookup, then a request. */
export function FindContact({ api, ownTag, busy, onAdd }: { api: SocialApi; ownTag: number; busy: boolean; onAdd(tag: number): Promise<boolean> }) {
  const lang = useLang();
  const [query, setQuery] = useState("");
  const [found, setFound] = useState<Person | null>(null);
  const [searching, setSearching] = useState(false);
  const [message, setMessage] = useState<Key | null>(null);
  const generation = useRef(0);
  useEffect(() => () => { ++generation.current; }, []);
  const search = async (e: FormEvent) => {
    e.preventDefault();
    const tag = tagOf(query);
    setFound(null);
    if (tag === null || tag === ownTag) { setMessage("social.badTag"); return; }
    const run = ++generation.current;
    setSearching(true); setMessage(null);
    try {
      const person = await api.lookup(tag);
      if (generation.current !== run) return;
      setFound(person);
      if (person === null) setMessage("social.notFound");
    } catch { if (generation.current === run) setMessage("social.failed"); }
    finally { if (generation.current === run) setSearching(false); }
  };
  return <section className="chat-find" aria-labelledby="contact-add">
    <h3 id="contact-add" className="chat-side-head"><T id="social.add" /></h3>
    <form onSubmit={(e) => { void search(e); }} className="chat-inline-row">
      <label className="vh" htmlFor="contact-tag"><T id="social.tag" /></label>
      <input id="contact-tag" value={query} maxLength={128} autoComplete="off" spellCheck={false} placeholder={t(lang, "social.tagHint")}
        onChange={(e) => { ++generation.current; setQuery(e.target.value); setFound(null); setMessage(null); setSearching(false); }} />
      <button type="submit" className="button button-ghost button-sm" disabled={searching || busy}><T id="social.find" /></button>
    </form>
    <output className={`form-status small${message === "social.badTag" || message === "social.failed" ? " bad" : ""}`} aria-live="polite">{searching ? <T id="app.loading" /> : message !== null ? <T id={message} /> : null}</output>
    {found !== null && <div className="chat-person chat-found">
      <Avatar name={found.displayName} seed={String(found.tag)} size="sm" />
      <bdi className="chat-person-name" translate="no">{handleOf(found)}</bdi>
      <button type="button" className="button button-sm" disabled={busy} onClick={() => { void onAdd(found.tag).then((ok) => { if (ok) { setFound(null); setQuery(""); } }); }}><T id="social.request" /></button>
    </div>}
  </section>;
}
