import { useEffect, useRef, useState } from "react";
import { ApiError, type AccountSummary } from "../account/api";
import { useAccount } from "../account/session";
import { AppFrame, Failed, SignedOut } from "../app/AppFrame";
import { useSummary } from "../app/useSummary";
import { T, t, useLang, useTitle } from "../i18n";
import { Link, PATHS } from "../router";
import { chatApi, type ChatApi } from "./api";
import { ChatSession } from "./session";
import { ChatWindow } from "./ChatWindow";
import "../styles/chat.css";

function Unlock({ summary, api, ownTag }: { summary: AccountSummary; api: ChatApi; ownTag: number }) {
  const account = useAccount();
  const lang = useLang();
  const [password, setPassword] = useState("");
  const [active, setActive] = useState<ChatSession | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const owner = useRef<ChatSession | null>(null);
  useEffect(() => () => { owner.current?.close(); owner.current = null; }, []);
  const lock = () => { owner.current?.close(); owner.current = null; setActive(null); setPassword(""); };
  async function unlock() {
    if (owner.current !== null || password.length === 0) return;
    const session = new ChatSession(api, account.api, summary);
    owner.current = session; setBusy(true); setError(false);
    const secret = password; setPassword("");
    try {
      const expiresAt = await session.unlock(secret, lock);
      if (owner.current === session) { account.refreshed(expiresAt); setActive(session); }
    } catch (cause) {
      session.close();
      if (owner.current === session) {
        owner.current = null; setError(true);
        if (cause instanceof ApiError && cause.kind === "rejected") setPassword("");
      }
    } finally { if (owner.current === session || owner.current === null) setBusy(false); }
  }
  if (active !== null) return <ChatWindow session={active} ownTag={ownTag} onLock={lock} />;
  return <section className="chat-unlock">
    <h2><T id="chat.unlockTitle" /></h2>
    <p><T id="chat.unlockLead" /></p>
    <form onSubmit={(event) => { event.preventDefault(); void unlock(); }}>
      <label htmlFor="chat-password"><T id="chat.password" /></label>
      <input id="chat-password" type="password" autoComplete="current-password" value={password} maxLength={1024} disabled={busy} onChange={(event) => setPassword(event.target.value)} required />
      <button className="button" disabled={busy || password.length === 0}>{busy ? t(lang, "app.loading") : t(lang, "chat.unlock")}</button>
    </form>
    {error && <p role="alert"><T id="chat.unlockFailed" /></p>}
  </section>;
}

export default function ChatPage({ client }: { client?: ChatApi }) {
  useTitle("title.accountChat");
  const { state, reload } = useSummary();
  const { generation } = useAccount();
  const [api] = useState(() => client ?? chatApi());
  return <AppFrame page="accountChat" admin={state.at === "ready" && state.summary.admin} eyebrow="app.account" title={<T id="chat.title" />} lead={<T id="chat.lead" />}>
    {state.at === "signedOut" ? <SignedOut /> : state.at === "failed" ? <Failed onRetry={reload} />
      : state.at === "loading" ? <output><T id="app.loading" /></output>
      : state.summary.tag === null || state.summary.displayName === null ? <p><Link to={PATHS.account}><T id="social.profileNeeded" /></Link></p>
      : <Unlock key={`${state.summary.identity.account.join()}:${generation}`} summary={state.summary} api={api} ownTag={state.summary.tag} />}
  </AppFrame>;
}
