// The frame of the account area and the administration
// (docs/website-design.md §"Components", the app frame): a side navigation
// beside the content at 1100 px and wider, a row of tabs above it below
// that. The administration's part is shown to an administrator only;
// Central refuses its requests to anybody else anyway.

import type { ReactNode } from "react";

import { Icon, type IconName } from "../components/icons";
import { T, t, useLang, type Key } from "../i18n";
import { Link, PATHS, type Page } from "../router";
import "../styles/app.css";

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
  { page: "admin", label: "app.users", icon: "people" },
  { page: "adminGateways", label: "app.adminGateways", icon: "globe" },
  { page: "adminAudit", label: "app.audit", icon: "list" },
  { page: "adminLegacy", label: "app.legacy", icon: "key" },
];

function Items({ items, page }: { items: readonly Item[]; page: Page }) {
  const lang = useLang();
  return (
    <ul className="app-nav-list">
      {items.map((item) => {
        const current = item.page === page || (item.page === "admin" && page === "adminUser");
        return (
          <li key={item.page}>
            <Link to={PATHS[item.page]} className="app-nav-link" aria-current={current ? "page" : undefined}>
              <Icon name={item.icon} className="app-nav-icon" />
              <span>{t(lang, item.label)}</span>
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
  eyebrow,
  title,
  lead,
  children,
}: {
  page: Page;
  admin: boolean;
  eyebrow: Key;
  title: ReactNode;
  lead?: ReactNode;
  children: ReactNode;
}) {
  const lang = useLang();
  return (
    <div className="app">
      <div className="app-inner">
        <nav className="app-nav" aria-label={t(lang, "app.nav")}>
          <p className="app-nav-head">
            <T id="app.account" />
          </p>
          <Items items={ACCOUNT} page={page} />
          {admin && (
            <>
              <p className="app-nav-head">
                <T id="app.admin" />
              </p>
              <Items items={ADMIN} page={page} />
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
