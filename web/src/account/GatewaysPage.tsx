// `/account/gateways`: the gateways this account registered, to host for
// others (docs/gateway-registry.md §"Gateway tokens"). Three steps: register
// the gateway's address here; make a token and paste it into the gateway's
// admin console, which then signs its players in through Central; a Central
// administrator approves it for the public list, where its token keeps it.

import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";

import { AppFrame, Failed, Panel, SignedOut, TableBox } from "../app/AppFrame";
import { Confirm } from "../app/Confirm";
import { day, when } from "../app/format";
import { useSummary } from "../app/useSummary";
import { Icon } from "../components/icons";
import { T, t, useLang, useTitle, type Key } from "../i18n";
import "../styles/account.css";
import { ApiError, type AccountSummary } from "./api";
import { BusyButton } from "./Busy";
import { ownerApi, shownName, type GatewayState, type MyGateway, type OwnerApi, type TokenInfo } from "./owner-api";
import { TokenDialog } from "./TokenDialog";
import { Address, CopyButton } from "../app/Copy";

export const STATE_WORD: Readonly<Record<GatewayState, Key>> = { pending: "gwo.pending", approved: "gwo.approved", disabled: "gwo.disabled" };
export const STATE_TONE: Readonly<Record<GatewayState, string>> = { pending: "warn", approved: "ok", disabled: "bad" };

/** A token's standing now: live, expired or revoked. */
export function standing(token: TokenInfo, now = Date.now() / 1000): { word: Key; tone: string } {
  if (token.revokedAt !== null) return { word: "tok.revoked", tone: "bad" };
  if (token.expiresAt <= now) return { word: "tok.expired", tone: "off" };
  return { word: "tok.live", tone: "ok" };
}

/** The tokens of one gateway, as a table; a live one can be revoked. */
export function TokenTable({ tokens, onRevoke }: { tokens: TokenInfo[]; onRevoke?: (token: TokenInfo) => void }) {
  const lang = useLang();
  if (tokens.length === 0) {
    return (
      <p className="small">
        <T id="tok.none" />
      </p>
    );
  }
  return (
    <TableBox label={t(lang, "tok.title2")}>
      <table className="table">
        <thead>
          <tr>
            <th scope="col"><T id="tok.name" /></th>
            <th scope="col"><T id="tok.scopes" /></th>
            <th scope="col"><T id="tok.expires" /></th>
            <th scope="col"><T id="tok.lastUsed" /></th>
            <th scope="col"><T id="tok.state" /></th>
            {onRevoke !== undefined && <th scope="col"><span className="vh"><T id="tok.actions" /></span></th>}
          </tr>
        </thead>
        <tbody>
          {tokens.map((token) => {
            const s = standing(token);
            return (
              <tr key={token.name + String(token.createdAt) + String(token.id[15])}>
                <th scope="row"><bdi>{token.name}</bdi></th>
                <td>
                  {token.scopes.sso && <span className="pill info">{t(lang, "tok.ssoShort")}</span>}{" "}
                  {token.scopes.listing && <span className="pill info">{t(lang, "tok.listingShort")}</span>}
                </td>
                <td className="num">{day(lang, token.expiresAt)}</td>
                <td className="num">{token.lastUsedAt === null ? "—" : when(lang, token.lastUsedAt)}</td>
                <td><span className={`pill ${s.tone}`}>{t(lang, s.word)}</span></td>
                {onRevoke !== undefined && (
                  <td className="table-actions">
                    {s.word === "tok.live" && (
                      <button type="button" className="button button-danger button-sm" onClick={() => onRevoke(token)}>
                        <T id="tok.revoke" />
                      </button>
                    )}
                  </td>
                )}
              </tr>
            );
          })}
        </tbody>
      </table>
    </TableBox>
  );
}

function AddGateway({ owner, onAdded }: { owner: OwnerApi; onAdded(): void }) {
  const lang = useLang();
  const [origin, setOrigin] = useState("https://");
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ bad: boolean; key: Key } | null>(null);
  const originField = useRef<HTMLInputElement>(null);
  const nameField = useRef<HTMLInputElement>(null);
  // which field the problem is in, for aria-invalid and the message it points at
  const wrong = status?.key === "gwo.errOrigin" || status?.key === "gwo.errTaken" ? "origin" : status?.key === "gwo.errName" || status?.key === "gwo.errNameChars" ? "name" : null;
  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (busy) return;
    let at: string;
    try {
      at = new URL(origin.trim()).origin;
    } catch {
      setStatus({ bad: true, key: "gwo.errOrigin" });
      originField.current?.focus();
      return;
    }
    if (name.trim() === "" || !shownName(name.trim())) {
      setStatus({ bad: true, key: name.trim() === "" ? "gwo.errName" : "gwo.errNameChars" });
      nameField.current?.focus();
      return;
    }
    setBusy(true);
    owner.addGateway(at, name.trim()).then(
      () => {
        setBusy(false);
        setOrigin("https://");
        setName("");
        setStatus({ bad: false, key: "gwo.added" });
        onAdded();
      },
      (error: unknown) => {
        setBusy(false);
        const kind = error instanceof ApiError ? error.kind : "network";
        setStatus({ bad: true, key: kind === "conflict" ? "gwo.errTaken" : kind === "busy" ? "gwo.errMany" : kind === "rejected" ? "gwo.errRefused" : "acct.errUnreachable" });
      },
    );
  };
  return (
    <form className="add-gateway" noValidate onSubmit={submit} aria-busy={busy}>
      <div className="field">
        <label htmlFor="gw-origin"><T id="gwo.origin" /></label>
        <input
          ref={originField}
          id="gw-origin"
          inputMode="url"
          autoComplete="off"
          spellCheck={false}
          value={origin}
          onChange={(e) => setOrigin(e.target.value)}
          aria-invalid={wrong === "origin" ? true : undefined}
          aria-describedby={wrong === "origin" ? "gw-origin-hint gw-add-status" : "gw-origin-hint"}
        />
        <p id="gw-origin-hint" className="field-hint"><T id="gwo.originHint" /></p>
      </div>
      <div className="field">
        <label htmlFor="gw-name"><T id="gwo.name" /></label>
        <input
          ref={nameField}
          id="gw-name"
          maxLength={80}
          autoComplete="off"
          value={name}
          onChange={(e) => setName(e.target.value)}
          aria-invalid={wrong === "name" ? true : undefined}
          aria-describedby={wrong === "name" ? "gw-add-status" : undefined}
        />
      </div>
      <div className="form-row">
        <BusyButton busy={busy} type="submit">
          <Icon name="plus" className="button-icon" />
          <T id="gwo.add" />
        </BusyButton>
        <output id="gw-add-status" className={`form-status${status?.bad === true ? " bad" : ""}`}>{status === null ? "" : t(lang, status.key)}</output>
      </div>
    </form>
  );
}

function Gateway({ g, onToken, onRevoke }: { g: MyGateway; onToken(): void; onRevoke(token: TokenInfo): void }) {
  const lang = useLang();
  return (
    <Panel
      title={<bdi>{g.metadata.name}</bdi>}
      actions={
        <button type="button" className="button button-ghost button-sm" onClick={onToken}>
          <Icon name="key" className="button-icon" />
          <T id="tok.make" />
        </button>
      }
    >
      <p className="gw-pills">
        <span className={`pill ${STATE_TONE[g.state]}`}>{t(lang, STATE_WORD[g.state])}</span>{" "}
        <span className={`pill ${g.online ? "ok" : "off"}`}>
          <span className={`dot ${g.online ? "ok live" : "off"}`} aria-hidden="true" />
          {t(lang, g.online ? "gw.online" : "gw.notSeen")}
        </span>
      </p>
      <dl className="kv">
        <div><dt><T id="gw.address" /></dt><dd><span className="copyable"><Address origin={g.origin} /><CopyButton text={g.origin} label={t(lang, "copy.address")} /></span></dd></div>
        {/* a registered gateway's version is a placeholder until its first heartbeat */}
        <div><dt><T id="gw.version" /></dt><dd>{g.lastSeen === null ? "—" : <code translate="no">{g.metadata.version}</code>}</dd></div>
        <div><dt><T id="gwo.registered" /></dt><dd className="num">{day(lang, g.createdAt)}</dd></div>
        <div><dt><T id="gwo.lastSeen" /></dt><dd className="num">{g.lastSeen === null ? "—" : when(lang, g.lastSeen)}</dd></div>
      </dl>
      {g.state === "pending" && <p className="small"><T id="gwo.pendingText" /></p>}
      {g.state === "disabled" && <p className="small"><T id="gwo.disabledText" /></p>}
      <h3 className="app-sub"><T id="tok.title2" /></h3>
      <TokenTable tokens={g.tokens} onRevoke={onRevoke} />
    </Panel>
  );
}

type Load = { at: "loading" } | { at: "failed" } | { at: "ready"; gateways: MyGateway[] };

export default function GatewaysPage({ client }: { client?: OwnerApi }) {
  useTitle("title.accountGateways");
  const lang = useLang();
  const { state, replace } = useSummary();
  const [owner] = useState(() => client ?? ownerApi());
  const [load, setLoad] = useState<Load>({ at: "loading" });
  const [making, setMaking] = useState<MyGateway | null>(null);
  const [revoking, setRevoking] = useState<TokenInfo | null>(null);
  const [said, setSaid] = useState<{ bad: boolean; key: Key } | null>(null);
  const saidRef = useRef<HTMLOutputElement>(null);
  const ready = state.at === "ready";
  // the revoked token's button goes away: focus the words that say what happened
  const tell = (next: { bad: boolean; key: Key }) => {
    setSaid(next);
    requestAnimationFrame(() => saidRef.current?.focus());
  };
  const read = useCallback(() => {
    owner.gateways().then(
      (gateways) => setLoad({ at: "ready", gateways }),
      () => setLoad({ at: "failed" }),
    );
  }, [owner]);
  useEffect(() => {
    if (ready) read();
  }, [ready, read]);

  const revoke = () => {
    if (revoking === null) return;
    const token = revoking;
    setRevoking(null);
    owner.revokeToken(token.id).then(
      () => {
        tell({ bad: false, key: "tok.revokedSaid" });
        read();
      },
      () => tell({ bad: true, key: "acct.errUnreachable" }),
    );
  };

  const summary: AccountSummary | null = state.at === "ready" ? state.summary : null;
  return (
    <AppFrame page="accountGateways" admin={summary?.admin ?? false} eyebrow="app.account" title={<T id="gwo.title" />} lead={<T id="gwo.lead" />}>
      {state.at === "signedOut" ? (
        <SignedOut />
      ) : (
        <>
          {load.at === "ready" && load.gateways.length > 0 ? (
            <details className="steps-more">
              <summary><T id="gwo.howTo" /></summary>
              <ol className="steps-small">
                {(["gwo.step1", "gwo.step2", "gwo.step3"] as const).map((key) => (
                  <li key={key}><T id={key} /></li>
                ))}
              </ol>
            </details>
          ) : (
          <ol className="steps stagger">
            {(["gwo.step1", "gwo.step2", "gwo.step3"] as const).map((key, i) => (
              <li key={key} className="step card reveal">
                <span className="step-n num" aria-hidden="true">{i + 1}</span>
                <T id={key} />
              </li>
            ))}
          </ol>
          )}
          <Panel title={<T id="gwo.addTitle" />}>
            <AddGateway owner={owner} onAdded={read} />
          </Panel>
          <output ref={saidRef} tabIndex={-1} className={`form-status${said?.bad === true ? " bad" : ""}`}>{said === null ? "" : t(lang, said.key)}</output>
          {load.at === "loading" && <div className="card skeleton app-skeleton" aria-hidden="true" />}
          {load.at === "failed" && <Failed onRetry={() => { setLoad({ at: "loading" }); read(); }} />}
          {load.at === "ready" && load.gateways.length === 0 && <p className="small app-none"><T id="gwo.none" /></p>}
          {load.at === "ready" &&
            load.gateways.map((g) => <Gateway key={g.origin} g={g} onToken={() => setMaking(g)} onRevoke={setRevoking} />)}
          {making !== null && summary !== null && (
            <TokenDialog
              gateway={making}
              owner={owner}
              summary={summary}
              onSummary={replace}
              onClose={(made) => {
                setMaking(null);
                if (made) read();
              }}
            />
          )}
          {revoking !== null && (
            <Confirm title={<T id="tok.revokeTitle" args={{ name: revoking.name }} />} action="tok.revoke" onConfirm={revoke} onCancel={() => setRevoking(null)}>
              <p><T id="tok.revokeText" /></p>
            </Confirm>
          )}
        </>
      )}
    </AppFrame>
  );
}
