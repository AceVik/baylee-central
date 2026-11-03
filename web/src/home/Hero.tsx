// The start page's hero: Baylee's interactive logo (which opens her picture
// and storybook), the calm centered introduction, the download button with
// quick-access dropdown, prominent Discord, WhatsApp, and GitHub links, and
// the beta/open-source/fan-content notice below.

import { useEffect, useRef } from "react";

import { useAccount } from "../account/session";
import { DISCORD, GITHUB, WHATSAPP } from "../config";
import { T, type Elems } from "../i18n";
import { Icon } from "../components/icons";
import { DownloadButton } from "../download/DownloadButton";
import { useRelease } from "../download/useRelease";
import { useParallax } from "../motion";
import { Link, PATHS } from "../router";
import { HeroShader } from "./HeroShader";
import { Remembrance } from "./Remembrance";
import "../styles/hero.css";

const GLOW: Elems = { glow: (words) => <span className="glow">{words}</span> };

const NOTE_ELEMS: Elems = {
  selfhost: (words) => <Link to={`${PATHS.gateways}#selfhost`}>{words}</Link>,
};

const SIGNUP_ELEMS: Elems = {
  register: (words) => <Link to={PATHS.register}>{words}</Link>,
  signin: (words) => <Link to={PATHS.signIn}>{words}</Link>,
};

/**
 * The hero's slow light and shader move only while the hero is on screen
 * (`data-away` pauses them, hero.css): off screen they would repaint a
 * blended layer for nobody.
 */
function useAwayMark(): { current: HTMLElement | null } {
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    const hero = ref.current;
    if (!hero || typeof IntersectionObserver !== "function") return;
    const watch = new IntersectionObserver(([entry]) => {
      if (entry?.isIntersecting) delete hero.dataset["away"];
      else hero.dataset["away"] = "";
    });
    watch.observe(hero);
    return () => watch.disconnect();
  }, []);
  return ref;
}

export function Hero() {
  const sky = useParallax(0.35);
  const state = useRelease();
  const { account } = useAccount();
  const hero = useAwayMark();
  return (
    <section ref={hero} className="hero" aria-labelledby="hero-title">
      <div className="sky" aria-hidden="true">
        <div className="sky-painting" ref={sky} />
        <HeroShader />
        <div className="sky-light" />
        <div className="sky-mist" />
        <div className="sky-veil" />
      </div>
      <div className="hero-inner">
        <Remembrance />
        <div className="hero-heading">
          <h1 id="hero-title">
            <T id="hero.title" elems={GLOW} />
          </h1>
        </div>
        <div className="hero-details">
          <p className="lead">
            <T id="hero.lead" />
          </p>
          <DownloadButton state={state} />
          <p className="hero-note">
            <T id="hero.note" elems={NOTE_ELEMS} />
          </p>
          {account === null && (
            <p className="hero-signup">
              <T id="hero.signup" elems={SIGNUP_ELEMS} />
            </p>
          )}
          <div className="hero-actions">
            <a className="button button-ghost hero-social hero-discord" href={DISCORD} target="_blank" rel="noopener">
              <span className="discord-beacon" aria-hidden="true" />
              <Icon name="discord" className="hero-social-icon discord-icon" />
              <span>
                <T id="hero.discord" />
              </span>
            </a>
            <a className="button button-ghost hero-social hero-whatsapp" href={WHATSAPP} target="_blank" rel="noopener">
              <span className="whatsapp-beacon" aria-hidden="true" />
              <Icon name="whatsapp" className="hero-social-icon whatsapp-icon" />
              <span>
                <T id="hero.whatsapp" />
              </span>
            </a>
            <a className="button button-ghost hero-social hero-github" href={GITHUB} target="_blank" rel="noopener">
              <Icon name="github" className="hero-social-icon github-icon" />
              <span>
                <T id="hero.github" />
              </span>
            </a>
          </div>
          <p className="kicker">
            <T id="hero.kicker" />
          </p>
        </div>
      </div>
    </section>
  );
}
