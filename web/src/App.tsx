// The app: the frame, and the page the address names. Every page but the
// start page loads when it is first opened, so the first visit fetches only
// what it shows.

import { Suspense, lazy, useEffect, useRef } from "react";

import { AccountProvider } from "./account/session";
import { Footer } from "./components/Footer";
import { Header } from "./components/Header";
import { WorldBackdrop } from "./components/WorldBackdrop";
import { NotFound } from "./components/NotFound";
import { SkipLink } from "./components/Section";
import Home from "./home/Home";
import { T } from "./i18n";
import { reducedMotion, useReveal } from "./motion";
import { navigate, onPrefetch, pageAt, useLocation, type Page } from "./router";

/** Each page but the start page, as the code that draws it, fetched once. */
const LOAD = {
  download: () => import("./download/DownloadPage"),
  cards: () => import("./cards/CardsPage"),
  gateways: () => import("./gateways/GatewaysPage"),
  contact: () => import("./contact/ContactPage"),
  impressum: () => import("./impressum/ImpressumPage"),
  privacy: () => import("./privacy/PrivacyPage"),
  terms: () => import("./legal/TermsPage"),
  register: () => import("./account/RegisterPage"),
  signIn: () => import("./account/SignInPage"),
  account: () => import("./account/AccountPage"),
  accountSecurity: () => import("./account/SecurityPage"),
  accountGateways: () => import("./account/GatewaysPage"),
  admin: () => import("./admin/OverviewPage"),
  adminUsers: () => import("./admin/UsersPage"),
  adminUser: () => import("./admin/UserPage"),
  adminGateways: () => import("./admin/GatewaysPage"),
  adminAudit: () => import("./admin/AuditPage"),
  adminLegacy: () => import("./admin/LegacyPage"),
} satisfies Record<Exclude<Page, "home">, () => Promise<unknown>>;

const Download = lazy(LOAD.download);
const Cards = lazy(LOAD.cards);
const Gateways = lazy(LOAD.gateways);
const Contact = lazy(LOAD.contact);
const Impressum = lazy(LOAD.impressum);
const Privacy = lazy(LOAD.privacy);
const Terms = lazy(LOAD.terms);
const Register = lazy(LOAD.register);
const SignIn = lazy(LOAD.signIn);
const Account = lazy(LOAD.account);
const Security = lazy(LOAD.accountSecurity);
const MyGateways = lazy(LOAD.accountGateways);
const AdminOverview = lazy(LOAD.admin);
const AdminUsers = lazy(LOAD.adminUsers);
const AdminUser = lazy(LOAD.adminUser);
const AdminGateways = lazy(LOAD.adminGateways);
const AdminAudit = lazy(LOAD.adminAudit);
const AdminLegacy = lazy(LOAD.adminLegacy);

// a link pointed at or focused fetches its page's code ahead, unless the
// visitor asked their browser to save data
onPrefetch((to) => {
  const saving = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection?.saveData === true;
  const page = pageAt(new URL(to, window.location.href).pathname);
  if (saving || page === null || page === "home") return;
  void LOAD[page]().catch(() => {});
});

function Loading() {
  return (
    <output className="page-loading">
      <span className="vh">
        <T id="app.loading" />
      </span>
    </output>
  );
}

function PageView({ page }: { page: Page | null }) {
  switch (page) {
    case "home":
      return <Home />;
    case "download":
      return <Download />;
    case "cards":
      return <Cards />;
    case "gateways":
      return <Gateways />;
    case "contact":
      return <Contact />;
    case "impressum":
      return <Impressum />;
    case "privacy":
      return <Privacy />;
    case "terms":
      return <Terms />;
    case "register":
      return <Register />;
    case "signIn":
      return <SignIn />;
    case "account":
      return <Account />;
    case "accountSecurity":
      return <Security />;
    case "accountGateways":
      return <MyGateways />;
    case "admin":
      return <AdminOverview />;
    case "adminUsers":
      return <AdminUsers />;
    case "adminUser":
      return <AdminUser />;
    case "adminGateways":
      return <AdminGateways />;
    case "adminAudit":
      return <AdminAudit />;
    case "adminLegacy":
      return <AdminLegacy />;
    case null:
      return <NotFound />;
  }
}

export function App() {
  const location = useLocation();
  const page = pageAt(location.pathname);
  const main = useRef<HTMLElement>(null);
  const shownPath = useRef<string | null>(null);
  const hash = useRef(location.hash);
  useEffect(() => {
    hash.current = location.hash;
  });

  // the old site's links to one set (`/#set-lea`) live on at /cards
  useEffect(() => {
    if (page === "home" && location.hash.startsWith("#set-")) navigate(`/cards${location.hash}`, true);
  }, [page, location.hash]);

  // a new page starts at its top (or its anchor), and the keyboard and a
  // screen reader start at its main content; the first load keeps the
  // browser's own scroll restoration
  const { pathname } = location;
  useEffect(() => {
    const isFirst = shownPath.current === null;
    if (shownPath.current === pathname) return;
    shownPath.current = pathname;
    const id = hash.current.slice(1);
    if (!isFirst) main.current?.focus({ preventScroll: true });
    if (id === "") {
      if (!isFirst) window.scrollTo(0, 0);
      return;
    }
    // a set's link is the card status's to follow (it opens the set first)
    if (id.startsWith("set-")) return;
    // the anchor may be on a page that is still loading: look for it a while
    let frame = 0;
    let tries = 0;
    const find = () => {
      const target = document.getElementById(id);
      if (target) target.scrollIntoView({ behavior: reducedMotion() || isFirst ? "auto" : "smooth" });
      else if (tries++ < 120) frame = requestAnimationFrame(find);
    };
    find();
    return () => cancelAnimationFrame(frame);
  }, [pathname]);

  useReveal(main);

  // Ctrl+K (Cmd+K on a Mac) reaches the page's search field, where it has
  // one (`data-find`): two keys, so no single letter fires by accident
  // (WCAG 2.1.4), and the browser's own Ctrl+K is left alone elsewhere
  useEffect(() => {
    const find = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() !== "k" || !(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey) return;
      const field = document.querySelector<HTMLInputElement>("[data-find]");
      if (!field) return;
      e.preventDefault();
      field.focus();
      field.select();
    };
    document.addEventListener("keydown", find);
    return () => document.removeEventListener("keydown", find);
  }, []);

  // when the element that had focus goes away (a button that did its job, a
  // list read again), the keyboard goes on from the main content, never from
  // the top of the page; pages that know a better place move it themselves
  useEffect(() => {
    let frame = 0;
    const lost = (e: FocusEvent) => {
      const gone = e.target;
      if (e.relatedTarget !== null || !(gone instanceof Node)) return;
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        if (!gone.isConnected && (document.activeElement === null || document.activeElement === document.body)) main.current?.focus({ preventScroll: true });
      });
    };
    document.addEventListener("focusout", lost);
    return () => {
      cancelAnimationFrame(frame);
      document.removeEventListener("focusout", lost);
    };
  }, []);

  return (
    <AccountProvider>
      <WorldBackdrop companion={page === "signIn" || page === "register"} />
      <SkipLink />
      <Header page={page} />
      <main id="main" tabIndex={-1} ref={main}>
        <Suspense fallback={<Loading />}>
          <PageView page={page} />
        </Suspense>
      </main>
      <Footer />
    </AccountProvider>
  );
}
