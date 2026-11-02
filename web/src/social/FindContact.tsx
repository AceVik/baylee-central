import { useEffect, useRef, useState, type FormEvent } from "react";
import { T, type Key } from "../i18n";
import { handleOf, tagOf, type Person, type SocialApi } from "./api";

export function FindContact({ api, ownTag, busy, onAdd }: { api: SocialApi; ownTag: number; busy: boolean; onAdd(tag: number): Promise<boolean> }) {
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
  return <section className="card app-panel">
    <h2><T id="social.add" /></h2>
    <form onSubmit={(e) => { void search(e); }} className="social-search">
      <div className="field">
        <label htmlFor="contact-tag"><T id="social.tag" /></label>
        <input id="contact-tag" value={query} maxLength={128} autoComplete="off" spellCheck={false}
          onChange={(e) => { ++generation.current; setQuery(e.target.value); setFound(null); setMessage(null); setSearching(false); }} />
      </div>
      <button type="submit" className="button button-ghost" disabled={searching || busy}><T id="social.find" /></button>
    </form>
    <output aria-live="polite">{searching ? <T id="app.loading" /> : message !== null ? <T id={message} /> : null}</output>
    {found !== null && <div className="social-row">
      <bdi translate="no">{handleOf(found)}</bdi>
      <button className="button" disabled={busy} onClick={() => { void onAdd(found.tag).then((ok) => { if (ok) setFound(null); }); }}><T id="social.request" /></button>
    </div>}
  </section>;
}
