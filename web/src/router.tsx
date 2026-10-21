// A handful of pages need no router library: the start page, the download,
// the cards, the gateway directory, contact, the legal pages, and the
// account pages (registration, sign-in, the account). Links
// change the address with the History API; the server answers every one of
// these paths with index.html (README.md §"Serving it").

import { useEffect, useState, type AnchorHTMLAttributes, type MouseEvent } from "react";

export const PAGES = ["home", "download", "cards", "gateways", "contact", "impressum", "privacy", "terms", "register", "signIn", "account"] as const;
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
};

/** The page at `pathname`, or null for one this site does not have. */
export function pageAt(pathname: string): Page | null {
  const path = pathname.length > 1 ? pathname.replace(/\/+$/, "") : pathname;
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

/** Goes to `to` (a path on this site, with or without a hash) without a reload. */
export function navigate(to: string, replace = false): void {
  if (replace) window.history.replaceState(null, "", to);
  else window.history.pushState(null, "", to);
  window.dispatchEvent(new Event(EVENT));
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
