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

let unlockedChat: { key: string; session: ChatSession } | null = null;

function closeUnlockedChat() {
  if (unlockedChat !== null) {
    unlockedChat.session.close();
    unlockedChat = null;
  }
}

function Unlock({ sessionKey, summary, api, ownTag }: { sessionKey: string; summary: AccountSummary; api: ChatApi; ownTag: number }) {
  const account = useAccount();
  const lang = useLang();
  const [password, setPassword] = useState("");
  const [active, setActive] = useState<ChatSession | null>(() => {
    if (unlockedChat !== null && unlockedChat.key !== sessionKey) closeUnlockedChat();
    return unlockedChat?.key === sessionKey ? unlockedChat.session : null;
  });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const owner = useRef<ChatSession | null>(unlockedChat?.key === sessionKey ? unlockedChat.session : null);
  useEffect(() => {
    if (unlockedChat !== null && unlockedChat.key !== sessionKey) closeUnlockedChat();
  }, [sessionKey]);
  const lock = () => {
    closeUnlockedChat();
    owner.current?.close();
    owner.current = null;
    setActive(null);
    setPassword("");
  };
  async function unlock() {
    if (owner.current !== null || password.length === 0) return;
    const session = new ChatSession(api, account.api, summary);
    owner.current = session; setBusy(true); setError(false);
    const secret = password; setPassword("");
    try {
      const expiresAt = await session.unlock(secret, lock);
      if (owner.current === session) {
        unlockedChat = { key: sessionKey, session };
        account.refreshed(expiresAt);
        setActive(session);
      }
    } catch (cause) {
      session.close();
      if (owner.current === session) {
        owner.current = null; setError(true);
        if (cause instanceof ApiError && cause.kind === "rejected") setPassword("");
      }
    } finally { if (owner.current === session || owner.current === null) setBusy(false); }
  }
  if (active !== null) return <ChatWindow session={active} ownTag={ownTag} onLock={lock} />;
  return <section className="card app-panel chat-unlock">
    <h2 className="app-panel-title"><T id="chat.unlockTitle" /></h2>
    <p><T id="chat.unlockLead" /></p>
    <form onSubmit={(event) => { event.preventDefault(); void unlock(); }}>
      <div className="field">
        <label htmlFor="chat-password"><T id="chat.password" /></label>
        <input id="chat-password" type="password" autoComplete="current-password" value={password} maxLength={1024} disabled={busy} onChange={(event) => setPassword(event.target.value)} required />
      </div>
      <div className="form-row">
        <button className="button" disabled={busy || password.length === 0}>{busy ? t(lang, "app.loading") : t(lang, "chat.unlock")}</button>
      </div>
    </form>
    {error && <p className="field-error" role="alert"><T id="chat.unlockFailed" /></p>}
  </section>;
}

export default function ChatPage({ client }: { client?: ChatApi }) {
  useTitle("title.accountChat");
  const { state, reload } = useSummary();
  const { generation } = useAccount();
  const [api] = useState(() => client ?? chatApi());
  useEffect(() => {
    if (state.at === "signedOut") closeUnlockedChat();
  }, [state.at]);
  const sessionKey = state.at === "ready" ? `${state.summary.identity.account.join()}:${generation}` : "";
  return <AppFrame page="accountChat" admin={state.at === "ready" && state.summary.admin} eyebrow="app.account" title={<T id="chat.title" />} lead={<T id="chat.lead" />}>
    {state.at === "signedOut" ? <SignedOut /> : state.at === "failed" ? <Failed onRetry={reload} />
      : state.at === "loading" ? <div className="card skeleton app-skeleton" aria-hidden="true"><span className="vh"><T id="app.loading" /></span></div>
      : state.summary.tag === null || state.summary.displayName === null ? <div className="card app-panel"><p><Link to={PATHS.account}><T id="social.profileNeeded" /></Link></p></div>
      : <Unlock key={sessionKey} sessionKey={sessionKey} summary={state.summary} api={api} ownTag={state.summary.tag} />}
  </AppFrame>;
}
