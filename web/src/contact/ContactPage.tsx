import { DISCORD, ISSUES } from "../config";
import { Icon } from "../components/icons";
import { T, useTitle } from "../i18n";
import { Link, PATHS } from "../router";
import { ContactForm } from "./ContactForm";
import { Reports } from "./Reports";

export default function Contact() {
  useTitle("Contact – Baylee", "Kontakt – Baylee");
  return (
    <>
      <header className="page-head">
        <div className="section-inner">
          <p className="eyebrow">
            <T en="Contact" de="Kontakt" />
          </p>
          <h1>
            <T en="Write to us" de="Schreib uns" />
          </h1>
          <div className="two-col">
            <div>
              <p>
                <T
                  en="A question, an idea, a game you would like to see, or something you would rather not post in public: this form goes straight to the people behind Baylee. Leave an e-mail address if you want an answer; without one we can only read it."
                  de="Eine Frage, eine Idee, ein Spiel, das du gern sehen würdest, oder etwas, das du lieber nicht öffentlich schreibst: Dieses Formular geht direkt an die Leute hinter Baylee. Lass eine E-Mail-Adresse da, wenn du eine Antwort willst; ohne können wir es nur lesen."
                />
              </p>
              <p>
                <T
                  en={
                    <>
                      A card that resolved wrongly is best reported from the game with <kbd>F8</kbd>, because the report
                      can carry the whole game.
                    </>
                  }
                  de={
                    <>
                      Eine Karte, die falsch verrechnet wurde, meldest du am besten im Spiel mit <kbd>F8</kbd>, weil die
                      Meldung das ganze Spiel mitnehmen kann.
                    </>
                  }
                />
              </p>
              <ul className="channels">
                <li>
                  <a className="glass" href={DISCORD} rel="noopener">
                    <Icon name="chat" />
                    <span>
                      Discord
                      <small>
                        <T en="talk, find opponents, hear about builds" de="reden, Gegner finden, von Builds hören" />
                      </small>
                    </span>
                  </a>
                </li>
                <li>
                  <a className="glass" href={ISSUES} rel="noopener">
                    <Icon name="github" />
                    <span>
                      <T en="GitHub issues" de="GitHub-Issues" />
                      <small>
                        <T en="bugs and ideas, in public" de="Fehler und Ideen, öffentlich" />
                      </small>
                    </span>
                  </a>
                </li>
              </ul>
              <p className="small">
                <T
                  en={
                    <>
                      Who runs this site, with a postal address: the <Link to={PATHS.impressum}>legal notice</Link>.
                    </>
                  }
                  de={
                    <>
                      Wer diese Seite betreibt, mit Postanschrift: das <Link to={PATHS.impressum}>Impressum</Link>.
                    </>
                  }
                />
              </p>
            </div>
            <div className="contact-card glass">
              <ContactForm />
            </div>
          </div>
        </div>
      </header>

      <Reports />
    </>
  );
}
