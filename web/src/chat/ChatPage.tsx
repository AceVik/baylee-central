// /account/chat and /account/contacts: the messenger in the account area.
// The chat a sign-in unlocked becomes its session here on the first visit
// and stays for the page's life (held.ts); after a reload it resumes the
// identity this browser kept (kept.ts). Only where none is kept (another
// browser's storage cleared, or none to write to) a password typed here
// unlocks it, once.
import { useEffect, useRef, useState } from "react";

import type { AccountSummary } from "../account/api";
import { useAccount } from "../account/session";
import { AppFrame, Failed, SignedOut } from "../app/AppFrame";
import { useSummary } from "../app/useSummary";
import { T, useTitle } from "../i18n";
import { Link, PATHS } from "../router";
import { chatApi, type ChatApi } from "./api";
import { heldChat, holdChat, lockChat, takeKept, useHeld } from "./held";
import { Messenger, type Tab } from "./Messenger";
import { ChatSession } from "./session";
import "../styles/chat.css";

function Chat({ summary, api, tab }: { summary: AccountSummary; api: ChatApi; tab: Tab }) {
  const account = useAccount();
  useHeld();
  const own = summary.identity.account;
  const held = heldChat(own);
  const session = held?.session ?? null;
  // after a reload: the kept identity, once per page; only its absence asks for the password
  const [resuming, setResuming] = useState(held === null);
  const resumed = useRef(false);
  useEffect(() => {
    if (resumed.current || heldChat(own) !== null) return;
    resumed.current = true;
    const fresh = new ChatSession(api, account.api, summary);
    fresh.resume(() => { if (heldChat(own)?.session === fresh) lockChat(); }).then(
      () => { holdChat(own, fresh); setResuming(false); },
      () => { fresh.close(); setResuming(false); },
    );
  }, [own, api, account.api, summary]);
  // a sign-in's unlocked chat becomes this page's session
  useEffect(() => {
    if (held === null || !held.kept) return;
    const kept = takeKept(own);
    if (kept === null) return;
    const { crypto, device, registration } = kept.unlocked;
    holdChat(own, new ChatSession(api, kept.accountApi, kept.summary, crypto, undefined, undefined, { device, registration }));
  }, [held, own, api]);
  useEffect(() => {
    if (session === null) return;
    void session.start(() => { if (heldChat(own)?.session === session) lockChat(); }).catch(() => {});
  }, [session, own]);
  const unlock = async (password: string): Promise<boolean> => {
    const fresh = new ChatSession(api, account.api, summary);
    try {
      const expiresAt = await fresh.unlock(password, () => { if (heldChat(own)?.session === fresh) lockChat(); });
      holdChat(own, fresh);
      // the password's sign-in replaced this browser's session
      account.refreshed(expiresAt);
      return true;
    } catch {
      fresh.close();
      return false;
    }
  };
  return <Messenger ownTag={summary.tag ?? 0} ownName={summary.displayName ?? ""} session={session} opening={session === null && (held?.kept === true || resuming)} initialTab={tab} onUnlock={unlock} />;
}

export function MessengerPage({ client, tab }: { client?: ChatApi; tab: Tab }) {
  const { state, reload } = useSummary();
  const { generation } = useAccount();
  const [api] = useState(() => client ?? chatApi());
  useEffect(() => {
    if (state.at === "signedOut") lockChat();
  }, [state.at]);
  const page = tab === "chats" ? "accountChat" : "accountContacts";
  return <AppFrame page={page} admin={state.at === "ready" && state.summary.admin} eyebrow="app.account" title={<T id={tab === "chats" ? "chat.title" : "social.title"} />} wide>
    {state.at === "signedOut" ? <SignedOut /> : state.at === "failed" ? <Failed onRetry={reload} />
      : state.at === "loading" ? <div className="card skeleton chat-skeleton" aria-hidden="true"><span className="vh"><T id="app.loading" /></span></div>
      : state.summary.tag === null || state.summary.displayName === null ? <div className="card app-panel app-empty"><p><Link to={PATHS.account}><T id="social.profileNeeded" /></Link></p></div>
      : <Chat key={`${state.summary.identity.account.join()}:${generation}`} summary={state.summary} api={api} tab={tab} />}
  </AppFrame>;
}

export default function ChatPage({ client }: { client?: ChatApi }) {
  useTitle("title.accountChat");
  return <MessengerPage tab="chats" {...(client === undefined ? {} : { client })} />;
}
