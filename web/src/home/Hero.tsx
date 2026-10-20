// The start page's first screen: our painting in layers that drift with the
// scroll, the logo, what Baylee is in one sentence, and the download.

import { T } from "../i18n";
import { Icon } from "../components/icons";
import { Logo } from "../components/Logo";
import { DownloadButton } from "../download/DownloadButton";
import { useRelease } from "../download/useRelease";
import { useParallax } from "../motion";
import { Link, PATHS } from "../router";

export function Hero() {
  const sky = useParallax(0.35);
  const state = useRelease();
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
          <T
            en="Closed beta · free · open source · unofficial fan content"
            de="Geschlossene Beta · kostenlos · quelloffen · inoffizieller Fan-Content"
          />
        </p>
        <h1 id="hero-title">
          <T
            en={
              <>
                A table that <span className="glow">knows the rules.</span>
              </>
            }
            de={
              <>
                Ein Tisch, der <span className="glow">die Regeln kennt.</span>
              </>
            }
          />
        </h1>
        <p className="lead">
          <T
            en="Baylee is a rules engine and a 3D table for playing Magic: The Gathering with friends or against the house AI. The engine enforces the Comprehensive Rules, so nobody has to. Free, open source under the AGPL, and it stays that way."
            de="Baylee ist eine Regel-Engine und ein 3D-Spieltisch für Magic: The Gathering – mit Freunden oder gegen die Haus-KI. Die Engine setzt die Comprehensive Rules durch, damit es niemand am Tisch tun muss. Kostenlos, quelloffen unter der AGPL, und das bleibt so."
          />
        </p>
        <DownloadButton state={state} />
        <p className="hero-note">
          <T
            en="Plays offline against the house AI without an account. Online play on the Sanctuary gateway is a closed beta and needs a key."
            de="Offline gegen die Haus-KI geht ohne Konto. Online auf dem Sanctuary-Gateway ist eine geschlossene Beta und braucht einen Schlüssel."
          />
        </p>
        <p className="hero-actions">
          <a className="button button-ghost" href="#about">
            <T en="What Baylee is" de="Was Baylee ist" />
          </a>
          <Link className="button button-ghost" to={PATHS.cards}>
            <T en="Where the cards stand" de="Stand der Karten" />
          </Link>
        </p>
      </div>
      <a className="scroll-cue" href="#about" aria-label="Scroll down / Nach unten">
        <Icon name="down" />
      </a>
    </section>
  );
}
