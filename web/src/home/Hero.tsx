// Baylee's cloud above the garden city, with the project and its download.

import { T, t, useLang, type Elems } from "../i18n";
import { Icon } from "../components/icons";
import { Logo } from "../components/Logo";
import { DownloadButton } from "../download/DownloadButton";
import { useRelease } from "../download/useRelease";
import { useParallax } from "../motion";
import { Remembrance } from "./Remembrance";
import "../styles/hero.css";
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
      </div>
      <div className="hero-inner">
        <div className="hero-heading">
          <Logo className="hero-logo" sizes="(max-width: 480px) 12rem, 21rem" alt="" priority />
          <p className="kicker">
            <T id="hero.kicker" />
          </p>
          <h1 id="hero-title">
            <T id="hero.title" elems={GLOW} />
          </h1>
        </div>
        <Remembrance />
        <div className="hero-details">
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
      </div>
      <a className="scroll-cue" href="#about" aria-label={t(lang, "hero.scroll")}>
        <Icon name="down" />
      </a>
    </section>
  );
}
