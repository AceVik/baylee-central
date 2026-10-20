// The app: the frame, and the page the address names. Every page but the
// start page loads when it is first opened, so the first visit fetches only
// what it shows.

import { Suspense, lazy, useEffect, useRef } from "react";

import { Footer } from "./components/Footer";
import { Header } from "./components/Header";
import { NotFound } from "./components/NotFound";
import { SkipLink } from "./components/Section";
import Home from "./home/Home";
import { T } from "./i18n";
import { reducedMotion, useReveal } from "./motion";
import { navigate, pageAt, useLocation, type Page } from "./router";

const Download = lazy(() => import("./download/DownloadPage"));
const Cards = lazy(() => import("./cards/CardsPage"));
const Gateways = lazy(() => import("./gateways/GatewaysPage"));
const Contact = lazy(() => import("./contact/ContactPage"));
const Impressum = lazy(() => import("./impressum/ImpressumPage"));
const Privacy = lazy(() => import("./privacy/PrivacyPage"));
const Terms = lazy(() => import("./legal/TermsPage"));

function Loading() {
  return (
    <output className="page-loading">
      <span className="vh">
        <T en="Loading…" de="Lädt …" />
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

  return (
    <>
      <SkipLink />
      <Header page={page} />
      <main id="main" tabIndex={-1} ref={main}>
        <Suspense fallback={<Loading />}>
          <PageView page={page} />
        </Suspense>
      </main>
      <Footer />
    </>
  );
}
