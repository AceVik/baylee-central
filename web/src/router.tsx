// A handful of pages need no router library: the start page, the download,
// the cards, the gateway directory, contact, the legal pages, and the
// account pages (registration, sign-in, the account). Links
// change the address with the History API; the server answers every one of
// these paths with index.html (README.md §"Serving it").

import { useEffect, useState, type AnchorHTMLAttributes, type MouseEvent } from "react";
import { flushSync } from "react-dom";

export const PAGES = [
  "home",
  "download",
  "cards",
  "gateways",
  "contact",
  "impressum",
  "privacy",
  "terms",
  "register",
  "signIn",
  "account",
  "accountSecurity",
  "accountGateways",
  "admin",
  "adminUser",
  "adminGateways",
  "adminAudit",
] as const;
export type Page = (typeof PAGES)[number];

export const PATHS: Record<Page, string> = {
  home: "/",
  download: "/download",
  cards: "/cards",
  gateways: "/gateways",
  contact: "/contact",
  impressum: "/impressum",
  privacy: "/privacy",
  terms: "/terms",
  register: "/register",
  signIn: "/signin",
  account: "/account",
  accountSecurity: "/account/security",
  accountGateways: "/account/gateways",
  admin: "/admin",
  // one account's page is `/admin/users/<its id in hex>` (`userPath`)
  adminUser: "/admin/users",
  adminGateways: "/admin/gateways",
  adminAudit: "/admin/audit",
};

/** The administration's page of the account with the id `hex` (32 hex digits). */
export const userPath = (hex: string): string => `${PATHS.adminUser}/${hex}`;

/** The account id in an administration's account page path, or null. */
export function userAt(pathname: string): string | null {
  return /^\/admin\/users\/([0-9a-f]{32})\/?$/.exec(pathname)?.[1] ?? null;
}

/** The page at `pathname`, or null for one this site does not have. */
export function pageAt(pathname: string): Page | null {
  if (userAt(pathname) !== null) return "adminUser";
  const path = pathname.length > 1 ? pathname.replace(/\/+$/, "") : pathname;
  // the list of accounts is the administration's first page
  if (path === PATHS.adminUser) return "admin";
  for (const page of PAGES) if (PATHS[page] === path) return page;
  return null;
}

export interface Location {
  pathname: string;
  search: string;
  hash: string;
}

const EVENT = "baylee:navigate";

function here(): Location {
  const { pathname, search, hash } = window.location;
  return { pathname, search, hash };
}

/** Changes the address and tells every `useLocation`. */
function go(to: string, replace: boolean): void {
  if (replace) window.history.replaceState(null, "", to);
  else window.history.pushState(null, "", to);
  window.dispatchEvent(new Event(EVENT));
}

type Transitioning = Document & { startViewTransition?: (update: () => Promise<void>) => unknown };

/**
 * Goes to `to` (a path on this site, with or without a hash) without a
 * reload. Another page cross-fades in where the browser has View Transitions
 * and motion is welcome (docs/website-design.md §"Motion").
 */
export function navigate(to: string, replace = false): void {
  const doc = document as Transitioning;
  const samePage = new URL(to, window.location.href).pathname === window.location.pathname;
  const still = typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (samePage || still || typeof doc.startViewTransition !== "function") {
    go(to, replace);
    return;
  }
  // the browser draws nothing while the update runs, so no frame callback
  // would come: React draws the new page synchronously instead
  doc.startViewTransition(() => {
    flushSync(() => go(to, replace));
    return Promise.resolve();
  });
}

export function useLocation(): Location {
  const [location, setLocation] = useState(here);
  useEffect(() => {
    const update = () => setLocation(here());
    window.addEventListener("popstate", update);
    window.addEventListener("hashchange", update);
    window.addEventListener(EVENT, update);
    return () => {
      window.removeEventListener("popstate", update);
      window.removeEventListener("hashchange", update);
      window.removeEventListener(EVENT, update);
    };
  }, []);
  return location;
}

type LinkProps = Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href"> & { to: string };

/** A link within the site: a plain click stays in the app, anything else is the browser's. */
export function Link({ to, onClick, children, ...rest }: LinkProps) {
  const click = (e: MouseEvent<HTMLAnchorElement>) => {
    onClick?.(e);
    if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    e.preventDefault();
    navigate(to);
  };
  return (
    <a href={to} onClick={click} {...rest}>
      {children}
    </a>
  );
}
