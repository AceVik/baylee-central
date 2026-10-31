// The frame of the account area and the administration
// (docs/website-design.md §"Components", the app frame): a side navigation
// beside the content at 1100 px and wider, a row of tabs above it below
// that. The administration's part is shown to an administrator only;
// Central refuses its requests to anybody else anyway.

import { useEffect, useRef, type ReactNode } from "react";

import { Icon, type IconName } from "../components/icons";
import { T, num, t, useLang, type Key } from "../i18n";
import { Link, PATHS, type Page } from "../router";
import "../styles/app.css";
import "../styles/tables.css";

interface Item {
  page: Page;
  label: Key;
  icon: IconName;
}

const ACCOUNT: readonly Item[] = [
  { page: "account", label: "app.profile", icon: "user" },
  { page: "accountSecurity", label: "app.security", icon: "lock" },
  { page: "accountGateways", label: "app.gateways", icon: "server" },
];

const ADMIN: readonly Item[] = [
  { page: "admin", label: "app.overview", icon: "grid" },
  { page: "adminUsers", label: "app.users", icon: "people" },
  { page: "adminGateways", label: "app.adminGateways", icon: "globe" },
  { page: "adminAudit", label: "app.audit", icon: "list" },
  { page: "adminLegacy", label: "app.legacy", icon: "key" },
];

/** A number beside an item of the navigation, such as the gateways awaiting approval. */
export type Badges = Partial<Record<Page, number>>;

const NO_BADGES: Badges = {};

function Items({ items, page, badges = NO_BADGES }: { items: readonly Item[]; page: Page; badges?: Badges }) {
  const lang = useLang();
  return (
    <ul className="app-nav-list">
      {items.map((item) => {
        const current = item.page === page || (item.page === "adminUsers" && page === "adminUser");
        const badge = badges[item.page] ?? 0;
        return (
          <li key={item.page}>
            <Link to={PATHS[item.page]} className="app-nav-link" data-page={item.page} aria-current={current ? "page" : undefined}>
              <Icon name={item.icon} className="app-nav-icon" />
              <span>{t(lang, item.label)}</span>
              {badge > 0 && " "}
              {badge > 0 && (
                <span className="app-nav-badge num">
                  {num(lang, badge)}{" "}
                  <span className="vh">{t(lang, "app.waiting")}</span>
                </span>
              )}
            </Link>
          </li>
        );
      })}
    </ul>
  );
}

export function AppFrame({
  page,
  admin,
  badges,
  eyebrow,
  title,
  lead,
  children,
}: {
  page: Page;
  admin: boolean;
  badges?: Badges;
  eyebrow: Key;
  title: ReactNode;
  lead?: ReactNode;
  children: ReactNode;
}) {
  const lang = useLang();
  const nav = useRef<HTMLElement>(null);
  // where the tabs are one row that scrolls, the current one is in sight
  useEffect(() => {
    const current = nav.current?.querySelector<HTMLElement>(`[data-page="${page === "adminUser" ? "admin" : page}"]`);
    // (jsdom has no scrollIntoView)
    current?.scrollIntoView?.({ block: "nearest", inline: "center" });
  }, [page]);
  return (
    <div className="app">
      <div className="app-inner">
        <nav ref={nav} className="app-nav" aria-label={t(lang, "app.nav")}>
          <p className="app-nav-head">
            <T id="app.account" />
          </p>
          <Items items={ACCOUNT} page={page} />
          {admin && (
            <>
              <p className="app-nav-head">
                <T id="app.admin" />
              </p>
              <Items items={ADMIN} page={page} {...(badges === undefined ? {} : { badges })} />
            </>
          )}
        </nav>
        <section className="app-main" aria-labelledby="app-title">
          <header className="app-head">
            <p className="eyebrow">
              <T id={eyebrow} />
            </p>
            <h1 id="app-title">{title}</h1>
            {lead !== undefined && <p className="app-lead">{lead}</p>}
          </header>
          {children}
        </section>
      </div>
    </div>
  );
}

/** What the area says to a visitor who is not signed in. */
export function SignedOut() {
  return (
    <div className="card app-empty">
      <p>
        <T id="app.signedOut" />
      </p>
      <Link className="button" to={PATHS.signIn}>
        <T id="nav.signIn" />
      </Link>
    </div>
  );
}

/**
 * A list that could not be read says why and offers to try again; it never
 * shows an empty list instead (docs/website-design.md §"Components", the
 * states of a list).
 */
export function Failed({ onRetry, reason = "acct.errUnreachable" }: { onRetry(): void; reason?: Key }) {
  return (
    <div className="app-failed" role="alert">
      <p className="small">
        <T id={reason} />
      </p>
      <button type="button" className="button button-ghost button-sm" onClick={onRetry}>
        <Icon name="retry" className="button-icon" />
        <T id="gw.tryAgain" />
      </button>
    </div>
  );
}

/**
 * A table that may be wider than the screen scrolls in its own box, which
 * the keyboard can reach and a screen reader names (Safari does not focus a
 * scroll box by itself).
 */
export function TableBox({ label, children }: { label: string; children: ReactNode }) {
  return (
    // oxlint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- a scroll box must be reachable by keyboard (WCAG 2.1.1)
    <section className="table-wrap" aria-label={label} tabIndex={0}>
      {children}
    </section>
  );
}

/** A section of the area: a heading, and its content in a card. */
export function Panel({ title, actions, children }: { title: ReactNode; actions?: ReactNode; children: ReactNode }) {
  return (
    <section className="card app-panel reveal">
      <div className="app-panel-head">
        <h2 className="app-panel-title">{title}</h2>
        {actions !== undefined && <div className="app-panel-actions">{actions}</div>}
      </div>
      {children}
    </section>
  );
}
