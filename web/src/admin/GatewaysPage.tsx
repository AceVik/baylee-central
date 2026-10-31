// `/admin/gateways`: every registered gateway, newest first, by state, with
// its owner and its tokens. An administrator approves a gateway after
// checking outside Central who runs its address (docs/gateway-registry.md),
// disables one (it leaves the list, its credentials and tokens stop, its
// players' sign-ins there end), and revokes a token.

import { useCallback, useEffect, useId, useRef, useState } from "react";

import type { AccountSummary } from "../account/api";
import { ApiError } from "../account/api";
import { STATE_TONE, STATE_WORD, TokenTable } from "../account/GatewaysPage";
import { idText, type GatewayState, type TokenInfo } from "../account/owner-api";
import { Failed, Panel } from "../app/AppFrame";
import { Confirm } from "../app/Confirm";
import { day, when } from "../app/format";
import { T, t, useLang, useTitle, type Key } from "../i18n";
import { Link, userPath } from "../router";
import type { AdminApi, AdminGateway } from "./admin-api";
import { AdminFrame, failureKey, useAdminApi, useChange, who } from "./shared";

const FILTERS: readonly (GatewayState | null)[] = ["pending", "approved", "disabled", null];
const FILTER_WORD: Readonly<Record<string, Key>> = { pending: "gwo.pending", approved: "gwo.approved", disabled: "gwo.disabled", all: "agw.all" };

type Asking = { at: "approve" | "disable"; g: AdminGateway } | { at: "revoke"; token: TokenInfo } | null;

function GatewayRow({ g, open, onOpen, api, onAsk }: { g: AdminGateway; open: boolean; onOpen(): void; api: AdminApi; onAsk(a: Asking): void }) {
  const lang = useLang();
  // each row's buttons are named with its gateway ("Approve", for Garden Table)
  const name = useId();
  const [tokens, setTokens] = useState<TokenInfo[] | null>(null);
  useEffect(() => {
    if (open) api.gateway(g.id).then((d) => setTokens(d.tokens), () => setTokens([]));
  }, [api, g.id, open]);
  return (
    <li className="card agw reveal">
      <div className="agw-head">
        <div>
          <h3 id={name} className="agw-name" tabIndex={-1}><bdi>{g.metadata.name}</bdi></h3>
          <code translate="no">{g.origin}</code>
        </div>
        <p className="gw-pills">
          {g.official && <span className="pill gold"><T id="gw.official" /></span>}{" "}
          <span className={`pill ${STATE_TONE[g.state]}`}>{t(lang, STATE_WORD[g.state])}</span>{" "}
          <span className={`pill ${g.online ? "ok" : "off"}`}>{t(lang, g.online ? "gw.online" : "gw.notSeen")}</span>
        </p>
      </div>
      <dl className="kv agw-kv">
        <div><dt><T id="agw.owner" /></dt><dd>{g.owner === null ? t(lang, "agw.noOwner") : <Link to={userPath(idText(g.owner))}><bdi translate="no">{who({ displayName: g.ownerName, tag: g.ownerTag })}</bdi></Link>}</dd></div>
        <div><dt><T id="gwo.registered" /></dt><dd className="num">{day(lang, g.createdAt)}</dd></div>
        <div><dt><T id="gwo.lastSeen" /></dt><dd className="num">{g.lastSeen === null ? "—" : when(lang, g.lastSeen)}</dd></div>
        <div><dt><T id="agw.tokens" /></dt><dd className="num">{g.liveTokens}{g.credentialPair ? ` · ${t(lang, "agw.pair")}` : ""}</dd></div>
      </dl>
      <div className="action-row">
        {g.state !== "approved" && (
          <button type="button" className="button button-sm" aria-describedby={name} onClick={() => onAsk({ at: "approve", g })}>
            <T id={g.state === "pending" ? "agw.approve" : "agw.enable"} />
          </button>
        )}
        {g.state !== "disabled" && (
          <button type="button" className="button button-danger button-sm" aria-describedby={name} onClick={() => onAsk({ at: "disable", g })}>
            <T id="agw.disable" />
          </button>
        )}
        <button type="button" className="button button-ghost button-sm" aria-expanded={open} aria-describedby={name} onClick={onOpen}>
          <T id={open ? "agw.hideTokens" : "agw.showTokens"} />
        </button>
      </div>
      {open && (tokens === null ? <div className="skeleton app-skeleton" aria-hidden="true" /> : <TokenTable tokens={tokens} onRevoke={(token) => onAsk({ at: "revoke", token })} />)}
    </li>
  );
}

function Gateways({ api, summary, replace }: { api: AdminApi; summary: AccountSummary; replace(s: AccountSummary): void }) {
  const lang = useLang();
  const [filter, setFilter] = useState<GatewayState | null>("pending");
  const [items, setItems] = useState<AdminGateway[] | null>(null);
  const [more, setMore] = useState(false);
  const [open, setOpen] = useState<string | null>(null);
  const [asking, setAsking] = useState<Asking>(null);
  const [said, setSaid] = useState<{ bad: boolean; key: Key } | null>(null);
  const [failed, setFailed] = useState(false);
  const list = useRef<HTMLUListElement>(null);
  const saidRef = useRef<HTMLOutputElement>(null);
  // the row acted on: after the list is read again the keyboard goes on there,
  // at the next gateway when this one left the filter, so an administrator
  // works down the queue with Tab and Enter
  const acted = useRef<number | null>(null);
  const { run, pending } = useChange(summary, replace);
  const read = useCallback(
    (before: AdminGateway | null) => {
      api.gateways(filter, before?.id ?? null).then(
        (page) => {
          setItems((now) => (before === null || now === null ? page.items : [...now, ...page.items]));
          setMore(page.more);
          const at = acted.current;
          acted.current = null;
          if (at !== null)
            requestAnimationFrame(() => {
              const headings = list.current?.querySelectorAll<HTMLElement>(".agw-name") ?? [];
              (headings[Math.min(at, headings.length - 1)] ?? saidRef.current)?.focus();
            });
        },
        () => setFailed(true),
      );
    },
    [api, filter],
  );
  useEffect(() => read(null), [read]);
  const fail = (error: unknown) => {
    setSaid({ bad: true, key: failureKey(error instanceof ApiError ? error.kind : "network") });
    requestAnimationFrame(() => saidRef.current?.focus());
  };
  // the rows stay while the list is read again, so focus has somewhere to be
  const done = (key: Key) => () => {
    setSaid({ bad: false, key });
    read(null);
  };
  const show = (f: GatewayState | null) => {
    if (f === filter) return;
    setFailed(false);
    setItems(null);
    setFilter(f);
  };
  const act = () => {
    const a = asking;
    setAsking(null);
    if (a === null) return;
    const id = a.at === "revoke" ? null : a.g.id;
    const index = items?.findIndex((g) => g.id === id) ?? -1;
    acted.current = index >= 0 ? index : null;
    run(() => {
      if (a.at === "revoke") api.revokeGatewayToken(a.token.id).then(done("tok.revokedSaid"), fail);
      else api.setGatewayApproval(a.g.id, a.at === "approve").then(done(a.at === "approve" ? "agw.approvedSaid" : "agw.disabledSaid"), fail);
    });
  };
  return (
    <>
      <fieldset className="chips">
        <legend className="vh">{t(lang, "agw.filter")}</legend>
        {FILTERS.map((f) => (
          <button key={f ?? "all"} type="button" className="chip-btn" aria-pressed={filter === f} onClick={() => show(f)}>
            {t(lang, FILTER_WORD[f ?? "all"] ?? "agw.all")}
          </button>
        ))}
      </fieldset>
      <output ref={saidRef} tabIndex={-1} className={`form-status${said?.bad === true ? " bad" : ""}`}>{said === null ? "" : t(lang, said.key)}</output>
      {failed && (
        <Failed
          onRetry={() => {
            setFailed(false);
            read(null);
          }}
        />
      )}
      {items === null && !failed && <div className="card skeleton app-skeleton" aria-hidden="true" />}
      {items !== null && items.length === 0 && <p className="small app-none"><T id="agw.none" /></p>}
      {items !== null && items.length > 0 && (
        <ul ref={list} className="agws stagger">
          {items.map((g) => (
            <GatewayRow key={g.origin} g={g} api={api} open={open === g.origin} onOpen={() => setOpen(open === g.origin ? null : g.origin)} onAsk={setAsking} />
          ))}
        </ul>
      )}
      {items !== null && more && (
        <p className="dir-more">
          <button type="button" className="button button-ghost" onClick={() => read(items.at(-1) ?? null)}><T id="adm.more" /></button>
        </p>
      )}
      {asking !== null && asking.at !== "revoke" && (
        <Confirm
          title={<T id={asking.at === "approve" ? "agw.approveTitle" : "agw.disableTitle"} args={{ name: asking.g.metadata.name }} />}
          action={asking.at === "approve" ? "agw.approve" : "agw.disable"}
          danger={asking.at === "disable"}
          onConfirm={act}
          onCancel={() => setAsking(null)}
        >
          <p><T id={asking.at === "approve" ? "agw.approveText" : "agw.disableText"} args={{ origin: asking.g.origin }} /></p>
        </Confirm>
      )}
      {asking?.at === "revoke" && (
        <Confirm title={<T id="tok.revokeTitle" args={{ name: asking.token.name }} />} action="tok.revoke" onConfirm={act} onCancel={() => setAsking(null)}>
          <p><T id="tok.revokeText" /></p>
        </Confirm>
      )}
      {pending}
    </>
  );
}

export default function AdminGatewaysPage({ client }: { client?: AdminApi }) {
  useTitle("title.adminGateways");
  const api = useAdminApi(client);
  return (
    <AdminFrame page="adminGateways" title={<T id="agw.title" />} lead={<T id="agw.lead" />}>
      {({ summary, replace }) => (
        <Panel title={<T id="agw.listTitle" />}>
          <Gateways api={api} summary={summary} replace={replace} />
        </Panel>
      )}
    </AdminFrame>
  );
}
