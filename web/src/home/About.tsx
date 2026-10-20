import { T, tt, useLang } from "../i18n";
import { Icon } from "../components/icons";
import { Section } from "../components/Section";
import { FEATURES } from "./features";

export function About() {
  const lang = useLang();
  return (
    <Section
      id="about"
      eyebrow={<T en="What it is" de="Was es ist" />}
      title={<T en="Two halves, one game" de="Zwei Hälften, ein Spiel" />}
    >
      <div className="two-col reveal">
        <div>
          <h3>
            <T en="An engine that enforces the game" de="Eine Engine, die das Spiel durchsetzt" />
          </h3>
          <p>
            <T
              en="Priority and the stack, the layer system, replacement effects, triggers, state-based actions and combat: the engine applies the Comprehensive Rules and lists, at every step, exactly the choices a player has. The client offers those and nothing else. It never decides what is legal."
              de="Priorität und Stapel, das Schichtensystem, Ersetzungseffekte, Trigger, zustandsbasierte Aktionen und Kampf: Die Engine wendet die Comprehensive Rules an und zählt bei jedem Schritt genau die Entscheidungen auf, die ein Spieler hat. Der Client bietet genau diese an und sonst nichts. Er entscheidet nie, was erlaubt ist."
            />
          </p>
          <p>
            <T
              en="It is deterministic: a seeded random-number generator, no clock and no hash-map order in the rules, so the same game with the same answers always ends the same way. Every hosted game is recorded move by move and can be replayed to the same state, which is also what a bug report carries."
              de="Sie ist deterministisch: ein geseedeter Zufallsgenerator, keine Uhr und keine Hash-Map-Reihenfolge in den Regeln. Dasselbe Spiel mit denselben Antworten endet immer gleich. Jedes gehostete Spiel wird Zug für Zug aufgezeichnet und lässt sich auf denselben Zustand zurückspielen – das ist auch, was eine Fehlermeldung mitbringt."
            />
          </p>
        </div>
        <div>
          <h3>
            <T en="A 3D table built with Bevy" de="Ein 3D-Tisch auf Bevy" />
          </h3>
          <p>
            <T
              en={
                <>
                  Battlefield, hands, stack and graveyards on a table you can read at a glance, plus a lobby for
                  accounts, decks and rooms. Card images come from Scryfall while you play; a text view draws every card
                  from its rules text instead, and <kbd>T</kbd> switches between the two.
                </>
              }
              de={
                <>
                  Schlachtfeld, Hände, Stapel und Friedhöfe auf einem Tisch, den man auf einen Blick lesen kann, dazu
                  eine Lobby für Konten, Decks und Räume. Die Kartenbilder kommen beim Spielen von Scryfall; eine
                  Textansicht zeichnet jede Karte stattdessen aus ihrem Regeltext, und <kbd>T</kbd> schaltet um.
                </>
              }
            />
          </p>
          <p>
            <T
              en={
                <>
                  You can play with the mouse or entirely from the keyboard: <kbd>Space</kbd> passes, <kbd>Tab</kbd>{" "}
                  fast-forwards to the next phase while every decision stays yours, <kbd>L</kbd> opens a game log that
                  reads like a chat. Every key can be rebound.
                </>
              }
              de={
                <>
                  Mit der Maus oder ganz über die Tastatur: <kbd>Leertaste</kbd> passt, <kbd>Tab</kbd> spult zur
                  nächsten Phase vor, wobei jede Entscheidung bei dir bleibt, <kbd>L</kbd> öffnet ein Spielprotokoll, das
                  sich wie ein Chat liest. Jede Taste lässt sich umbelegen.
                </>
              }
            />
          </p>
        </div>
      </div>
      <ul className="features">
        {FEATURES.map((f) => (
          <li key={f.icon} className="glass reveal">
            <span className="feat-icon">
              <Icon name={f.icon} />
            </span>
            <h3>{tt(lang, f.title[0], f.title[1])}</h3>
            <p>{tt(lang, f.text[0], f.text[1])}</p>
          </li>
        ))}
      </ul>
    </Section>
  );
}
