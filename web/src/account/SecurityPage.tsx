// `/account/security`: the account's sessions as Central knows them. Central
// keeps no device: an account has at most one session on this website and
// one in the client, each with its password sign-in and its end, and the
// client's with the gateways it is signed in to now. The client's session
// can be ended from here, the website's by signing out.

import { useCallback, useEffect, useState } from "react";

import { AppFrame, Panel, SignedOut } from "../app/AppFrame";
import { Confirm } from "../app/Confirm";
import { clock, when } from "../app/format";
import { useSummary } from "../app/useSummary";
import { Icon } from "../components/icons";
import { T, t, useLang, useTitle, type Key } from "../i18n";
import "../styles/account.css";
import { ApiError } from "./api";
import { ownerApi, type OwnerApi, type SessionInfo } from "./owner-api";
import { useAccount } from "./session";

type Load = { at: "loading" } | { at: "failed" } | { at: "ready"; sessions: SessionInfo[] };

function Session({ s, onEnd }: { s: SessionInfo; onEnd?: () => void }) {
  const lang = useLang();
  const client = s.kind === "native";
  return (
    <li className="session">
      <span className="session-icon" aria-hidden="true">
        <Icon name={client ? "chip" : "globe"} />
      </span>
      <div className="session-body">
        <p className="session-title">
          <T id={client ? "sec.client" : "sec.website"} />
          {s.current && (
            <span className="pill info">
              <T id="sec.current" />
            </span>
          )}
        </p>
        <p className="small num">
          <T id="sec.since" args={{ time: when(lang, s.signedInAt) }} /> · <T id="sec.until" args={{ time: clock(lang, s.expiresAt) }} />
        </p>
        {client && (
          <p className="small">
            {s.gateways.length === 0 ? (
              <T id="sec.noGateway" />
            ) : (
              <>
                <T id="sec.gateways" />{" "}
                {s.gateways.map((g, i) => (
                  <span key={g}>
                    {i > 0 && ", "}
                    <code translate="no">{g}</code>
                  </span>
                ))}
              </>
            )}
          </p>
        )}
      </div>
      {onEnd !== undefined && (
        <button type="button" className="button button-danger button-sm" onClick={onEnd}>
          <T id="sec.endClient" />
        </button>
      )}
    </li>
  );
}

export default function SecurityPage({ client }: { client?: OwnerApi }) {
  useTitle("title.accountSecurity");
  const lang = useLang();
  const { signOut } = useAccount();
  const { state } = useSummary();
  const [owner] = useState(() => client ?? ownerApi());
  const [load, setLoad] = useState<Load>({ at: "loading" });
  const [asking, setAsking] = useState(false);
  const [said, setSaid] = useState<{ bad: boolean; key: Key } | null>(null);
  const ready = state.at === "ready";

  const read = useCallback(() => {
    owner.sessions().then(
      (sessions) => setLoad({ at: "ready", sessions }),
      () => setLoad({ at: "failed" }),
    );
  }, [owner]);
  useEffect(() => {
    if (ready) read();
  }, [ready, read]);

  const end = () => {
    setAsking(false);
    owner.endClientSession().then(
      () => {
        setSaid({ bad: false, key: "sec.ended" });
        read();
      },
      (error: unknown) => setSaid({ bad: true, key: error instanceof ApiError && error.kind === "busy" ? "acct.errBusy" : "acct.errUnreachable" }),
    );
  };

  const admin = state.at === "ready" && state.summary.admin;
  return (
    <AppFrame page="accountSecurity" admin={admin} eyebrow="app.account" title={<T id="sec.title" />} lead={<T id="sec.lead" />}>
      {state.at === "signedOut" ? (
        <SignedOut />
      ) : (
        <>
          <Panel title={<T id="sec.sessions" />}>
            {load.at === "loading" && <div className="skeleton session-skeleton" aria-hidden="true" />}
            {load.at === "failed" && (
              <p className="small" role="alert">
                <T id="acct.errUnreachable" />
              </p>
            )}
            {load.at === "ready" && (
              <ul className="sessions">
                {load.sessions.map((s) => (
                  <Session key={s.kind} s={s} {...(s.kind === "native" ? { onEnd: () => setAsking(true) } : {})} />
                ))}
                {!load.sessions.some((s) => s.kind === "native") && (
                  <li className="session session-none small">
                    <T id="sec.clientNone" />
                  </li>
                )}
              </ul>
            )}
            <output className={`form-status${said?.bad === true ? " bad" : ""}`}>{said === null ? "" : t(lang, said.key)}</output>
          </Panel>
          <Panel title={<T id="sec.signOutTitle" />}>
            <p>
              <T id="sec.signOutText" />
            </p>
            <button type="button" className="button button-ghost" onClick={() => void signOut()}>
              <Icon name="logout" className="button-icon" />
              <T id="account.signOut" />
            </button>
          </Panel>
          {asking && (
            <Confirm title={<T id="sec.endTitle" />} action="sec.endClient" onConfirm={end} onCancel={() => setAsking(false)}>
              <p>
                <T id="sec.endText" />
              </p>
            </Confirm>
          )}
        </>
      )}
    </AppFrame>
  );
}
