// The start page's first screen: our painting in layers that drift with the
// scroll, the logo, what Baylee is in one sentence, and the download.

import { T, t, useLang, type Elems } from "../i18n";
import { Icon } from "../components/icons";
import { Logo } from "../components/Logo";
import { DownloadButton } from "../download/DownloadButton";
import { useRelease } from "../download/useRelease";
import { useParallax } from "../motion";
import { Link, PATHS } from "../router";

const GLOW: Elems = { glow: (words) => <span className="glow">{words}</span> };

export function Hero() {
  const sky = useParallax(0.35);
  const state = useRelease();
  const lang = useLang();
  return (
    <section className="hero" aria-labelledby="hero-title">
      <div className="sky" aria-hidden="true">
        <div className="sky-painting" ref={sky} />
        <div className="sky-light" />
        <div className="sky-mist" />
        <div className="sky-veil" />
        <div className="sky-petals">
          {Array.from({ length: 10 }, (_, i) => (
            <i key={i} />
          ))}
        </div>
      </div>
      <div className="hero-inner">
        <Logo className="hero-logo" sizes="(max-width: 700px) 82vw, 30rem" alt="" priority />
        <p className="kicker">
          <T id="hero.kicker" />
        </p>
        <h1 id="hero-title">
          <T id="hero.title" elems={GLOW} />
        </h1>
        <p className="lead">
          <T id="hero.lead" />
        </p>
        <DownloadButton state={state} />
        <p className="hero-note">
          <T id="hero.note" />
        </p>
        <p className="hero-actions">
          <a className="button button-ghost" href="#about">
            <T id="hero.what" />
          </a>
          <Link className="button button-ghost" to={PATHS.cards}>
            <T id="hero.cards" />
          </Link>
        </p>
      </div>
      <a className="scroll-cue" href="#about" aria-label={t(lang, "hero.scroll")}>
        <Icon name="down" />
      </a>
    </section>
  );
}
