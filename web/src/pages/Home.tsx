// The start page: what Baylee is, what it looks like, who plays against
// you, where online play stands and that it stays open source. The words are
// the old landing page's (Baylee's web/site/index.html), the layout new.

import { DISCORD, GITHUB, ISSUES } from "../config";
import { CI_PATH, COMMIT_PATH, parseCi, parseCommit, parseRepo, relTime, useGitHub, dateText, type CiRun, type Commit } from "../github";
import { T, num, tt, useLang, useTitle } from "../i18n";
import { Icon, type IconName } from "../components/icons";
import { Logo } from "../components/Logo";
import { Section } from "../components/Layout";
import { Screens } from "../components/Screens";
import { DownloadButton, useRelease } from "../download/Download";
import { useParallax } from "../motion";
import { Link, PATHS } from "../router";

function Hero() {
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

function CiLink({ run }: { run: CiRun }) {
  const ok = run.conclusion === "success";
  const cls = ok ? "ok" : run.conclusion === "failure" ? "bad" : "warn";
  return (
    <a href={run.url} rel="noopener">
      <span className={`dot ${cls}`} aria-hidden="true" />
      {ok ? (
        <T en="passing" de="grün" />
      ) : run.conclusion === "failure" ? (
        <T en="failing" de="rot" />
      ) : (
        run.conclusion
      )}
    </a>
  );
}

function StatusStrip({ ci, commit }: { ci: CiRun | null | undefined; commit: Commit | null | undefined }) {
  const lang = useLang();
  const { release } = useRelease();
  return (
    <section className="strip" aria-label={tt(lang, "Project status", "Projektstand")}>
      <dl className="strip-inner">
        <div className="stat">
          <dt>
            <T en="Latest build" de="Neueste Version" />
          </dt>
          <dd>
            {release ? (
              <>
                <a href={release.html_url} rel="noopener">
                  {release.tag_name}
                </a>
                <span className="small"> · {dateText(release.published_at, lang)}</span>
              </>
            ) : (
              <a href={`${GITHUB}/releases`} rel="noopener">
                <T en="see releases" de="siehe Releases" />
              </a>
            )}
          </dd>
        </div>
        <div className="stat">
          <dt>
            <T en="CI on main" de="CI auf main" />
          </dt>
          <dd>
            {ci ? (
              <CiLink run={ci} />
            ) : (
              <a href={`${GITHUB}/actions/workflows/ci.yml`} rel="noopener">
                <T en="see on GitHub" de="auf GitHub ansehen" />
              </a>
            )}
          </dd>
        </div>
        <div className="stat">
          <dt>
            <T en="Last commit" de="Letzter Commit" />
          </dt>
          <dd>
            {commit ? (
              <>
                <a href={commit.url} rel="noopener">
                  {commit.sha.slice(0, 7)}
                </a>
                <span className="small"> · {relTime(commit.when, lang)}</span>
              </>
            ) : (
              <a href={`${GITHUB}/commits/main`} rel="noopener">
                <T en="see on GitHub" de="auf GitHub ansehen" />
              </a>
            )}
          </dd>
        </div>
        <div className="stat">
          <dt>
            <T en="Gateways" de="Gateways" />
          </dt>
          <dd>
            <Link to={PATHS.gateways}>
              <T en="the directory" de="das Verzeichnis" />
            </Link>
          </dd>
        </div>
      </dl>
    </section>
  );
}

const FEATURES: { icon: IconName; title: [string, string]; text: [string, string] }[] = [
  {
    icon: "eye",
    title: ["Hidden information stays hidden", "Verdeckte Information bleibt verdeckt"],
    text: [
      "Your client is sent only what you may see. Libraries and other hands arrive as counts, a face-down card carries no identity. A modified client cannot peek, because the data never reaches it.",
      "Dein Client bekommt nur, was du sehen darfst. Bibliotheken und fremde Hände kommen als Zahlen an, eine verdeckte Karte trägt keine Identität. Ein manipulierter Client kann nicht spicken, weil die Daten ihn nie erreichen.",
    ],
  },
  {
    icon: "people",
    title: ["Two to eight seats", "Zwei bis acht Plätze"],
    text: [
      "Duels, free-for-all and team tables. The host puts chairs on sides, and the sides need not be even. Teams keep separate turns and life totals; teammates can choose to show each other their hands.",
      "Duelle, Jeder-gegen-jeden und Teamtische. Der Gastgeber verteilt die Stühle auf Seiten, und die Seiten müssen nicht gleich groß sein. Teams haben eigene Züge und Lebenspunkte; Teamkollegen können sich gegenseitig die Hand zeigen.",
    ],
  },
  {
    icon: "crown",
    title: ["Commander and freeform", "Commander und Freiform"],
    text: [
      "A deck that names a commander plays Commander; any other deck plays freeform. Banlists and format-legality checks do not exist yet. That is a gap, not a feature.",
      "Ein Deck mit Commander spielt Commander, jedes andere spielt frei. Banlisten und Formatprüfungen gibt es noch nicht. Das ist eine Lücke, kein Feature.",
    ],
  },
  {
    icon: "chip",
    title: ["A house AI at five levels", "Eine Haus-KI in fünf Stufen"],
    text: [
      "Novice, casual, steady, sharp and expert. It plays from the same view a human gets, so it cannot see your hand either.",
      "Novice, casual, steady, sharp und expert. Sie spielt aus derselben Sicht wie ein Mensch und sieht deine Hand genauso wenig.",
    ],
  },
  {
    icon: "grid",
    title: ["One process per game", "Ein Prozess pro Spiel"],
    text: [
      "Each game runs in its own engine process. The gateway that handles accounts and rooms runs no rules at all, so one broken game is one broken game.",
      "Jedes Spiel läuft in einem eigenen Engine-Prozess. Das Gateway für Konten und Räume kennt keine Regeln, also ist ein kaputtes Spiel genau ein kaputtes Spiel.",
    ],
  },
  {
    icon: "globe",
    title: ["Card text in 19 languages", "Kartentext in 19 Sprachen"],
    text: [
      "When the gateway carries the card catalog, cards read in your language with English as the fallback. The interface itself is in English and German.",
      "Hat das Gateway den Kartenkatalog, lesen sich Karten in deiner Sprache, Englisch als Rückfall. Die Oberfläche selbst gibt es auf Englisch und Deutsch.",
    ],
  },
  {
    icon: "watch",
    title: ["Watch a running table", "Einem laufenden Tisch zuschauen"],
    text: [
      "The lobby's Watch button seats you as a spectator. You see what the whole table sees and nobody's hand. A host can switch spectators off for a room.",
      "Der Knopf Zuschauen in der Lobby setzt dich als Zuschauer an den Tisch. Du siehst, was der ganze Tisch sieht, und keine Hand. Ein Gastgeber kann Zuschauer für seinen Raum abschalten.",
    ],
  },
  {
    icon: "wifi",
    title: ["A lost connection is not a lost game", "Verbindung weg, Spiel nicht verloren"],
    text: [
      "When a player drops, the table shows a countdown while their chair waits for them. If time runs out, the house AI plays for them until they are back; with nobody else at the table, the game simply pauses.",
      "Fällt ein Spieler heraus, zeigt der Tisch einen Countdown, solange sein Platz auf ihn wartet. Läuft die Zeit ab, spielt die Haus-KI für ihn, bis er zurück ist; sitzt sonst niemand am Tisch, pausiert das Spiel einfach.",
    ],
  },
  {
    icon: "sliders",
    title: ["Tables your way", "Tische nach deinen Regeln"],
    text: [
      "A Mixed table seats decks of any format together, and fourteen decision clocks run from a 15-second bullet to untimed. Starting life and mulligans are the host's to set.",
      "Ein Tisch „Gemischt“ setzt Decks jedes Formats zusammen, und vierzehn Entscheidungsuhren reichen von 15 Sekunden bis ohne Uhr. Startleben und Mulligans legt der Gastgeber fest.",
    ],
  },
];

function About() {
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

function Opponents() {
  return (
    <Section
      id="ai"
      alt
      eyebrow={<T en="Opponents" de="Gegner" />}
      title={<T en="Opponents: the house, or a language model" de="Gegner: das Haus, oder ein Sprachmodell" />}
    >
      <div className="two-col reveal">
        <div>
          <h3>
            <T en="The house AI" de="Die Haus-KI" />
          </h3>
          <p>
            <T
              en="Five levels from novice to expert, available offline and at any online table. It plays from the same view a human gets, so hidden information stays hidden from it too. It is a heuristic player, not a trained one: good enough to learn a deck against, not a tournament opponent."
              de="Fünf Stufen von novice bis expert, offline und an jedem Online-Tisch. Sie spielt aus derselben Sicht wie ein Mensch, verdeckte Information bleibt also auch vor ihr verdeckt. Sie spielt heuristisch, nicht trainiert: gut genug, um ein Deck dagegen zu lernen, kein Turniergegner."
            />
          </p>
        </div>
        <div>
          <h3>
            <T en="A chair for a language model" de="Ein Stuhl für ein Sprachmodell" />
          </h3>
          <p>
            <T
              en="A seat bridge lets a model sit at the table as an ordinary player: a local model in LM Studio, one behind an OpenAI-compatible or Anthropic API, or an agent CLI on your own login. API keys never go in a file; a settings file names the environment variable they come from, and daily and monthly spending caps hold across games. When the model times out or answers something illegal, the house answers that question and the transcript says so."
              de="Eine Sitzbrücke lässt ein Modell als gewöhnlichen Spieler am Tisch sitzen: ein lokales Modell in LM Studio, eines hinter einer OpenAI-kompatiblen oder Anthropic-API oder ein Agent-CLI mit deinem eigenen Login. API-Schlüssel stehen nie in einer Datei; die Einstellungsdatei nennt die Umgebungsvariable, aus der sie kommen, und Tages- und Monatslimits gelten über alle Spiele. Wenn das Modell nicht antwortet oder etwas Unerlaubtes versucht, antwortet das Haus für diese Frage, und das Protokoll sagt es."
            />
          </p>
          <p>
            <T
              en={
                <>
                  At a gateway table the host seats one from the room: any open chair offers <i>language model</i> on a
                  desktop client, which starts the bridge (it ships beside the game) with your own settings and key.
                  Where the gateway&apos;s operator runs hosted models, a registered host can also pick one of those
                  instead, and it plays on the operator&apos;s machine, so no key or bridge is needed on yours.
                </>
              }
              de={
                <>
                  An einem Gateway-Tisch setzt der Gastgeber es im Raum hin: Jeder freie Stuhl bietet im Desktop-Client{" "}
                  <i>Sprachmodell</i> an, und der Client startet die Brücke (sie liegt dem Spiel bei) mit deinen eigenen
                  Einstellungen und deinem Schlüssel. Wo der Betreiber des Gateways gehostete Modelle anbietet, kann ein
                  registrierter Gastgeber stattdessen eines davon wählen; es spielt auf dem Rechner des Betreibers, auf
                  deinem braucht es dann weder Schlüssel noch Brücke.
                </>
              }
            />
          </p>
          <p className="small">
            <a href={`${GITHUB}#playing-against-a-language-model`} rel="noopener">
              <T en="How to set it up" de="So richtest du es ein (englisch)" />
            </a>
          </p>
        </div>
      </div>
    </Section>
  );
}

function Online() {
  return (
    <Section
      id="beta"
      eyebrow={<T en="Online" de="Online" />}
      title={<T en="The closed beta" de="Die geschlossene Beta" />}
    >
      <div className="two-col reveal">
        <div>
          <p>
            <T
              en={
                <>
                  The public gateway, <b>Baylee Sanctuary</b>, is where online tables live. It is a closed beta: a new
                  account and a new guest each need a key of the form <code>BAYLEE-XXXX-XXXX-XXXX-XXXX</code>, handed out
                  by the project owner. There is no public sign-up form, and we are not pretending otherwise.
                </>
              }
              de={
                <>
                  Das öffentliche Gateway <b>Baylee Sanctuary</b> ist der Ort für Online-Tische. Es ist eine geschlossene
                  Beta: Ein neues Konto und ein neuer Gast brauchen je einen Schlüssel der Form{" "}
                  <code>BAYLEE-XXXX-XXXX-XXXX-XXXX</code>, den der Projektinhaber vergibt. Ein öffentliches
                  Anmeldeformular gibt es nicht, und wir tun auch nicht so.
                </>
              }
            />
          </p>
          <p>
            <T
              en="Signing up asks for no e-mail address: an account is a username and a password. Other players see only your display name and a short tag. A guest needs just a display name and is deleted when it signs out or after 30 days without use. You can delete your account from the client at any time."
              de="Die Anmeldung fragt nach keiner E-Mail-Adresse: Ein Konto ist ein Benutzername und ein Passwort. Andere Spieler sehen nur deinen Anzeigenamen und ein kurzes Kürzel. Ein Gast braucht nur einen Anzeigenamen und wird beim Abmelden oder nach 30 Tagen ohne Nutzung gelöscht. Dein Konto kannst du jederzeit aus dem Client löschen."
            />
          </p>
          <p>
            <T
              en={
                <>
                  Keys come from the project owner,{" "}
                  <a href="https://github.com/AceVik" rel="noopener">
                    @AceVik
                  </a>{" "}
                  on GitHub. Without one, Baylee still plays offline against the house AI, with the deck builder and your
                  own decks, and you can run a gateway of your own for your group.
                </>
              }
              de={
                <>
                  Schlüssel kommen vom Projektinhaber,{" "}
                  <a href="https://github.com/AceVik" rel="noopener">
                    @AceVik
                  </a>{" "}
                  auf GitHub. Ohne Schlüssel spielt Baylee trotzdem offline gegen die Haus-KI, mit Deckbauer und eigenen
                  Decks, und du kannst für deine Runde ein eigenes Gateway betreiben.
                </>
              }
            />
          </p>
        </div>
        <div className="link-card glass">
          <span className="feat-icon">
            <Icon name="server" />
          </span>
          <h3>
            <T en="Every gateway, in one list" de="Jedes Gateway in einer Liste" />
          </h3>
          <p>
            <T
              en="Sanctuary and the gateways other people run for their groups: the directory shows each one's address, version and whether it is online right now."
              de="Sanctuary und die Gateways, die andere für ihre Runde betreiben: Das Verzeichnis zeigt von jedem Adresse, Version und ob es gerade online ist."
            />
          </p>
          <p className="link-card-actions">
            <Link className="button" to={PATHS.gateways}>
              <T en="Open the directory" de="Verzeichnis öffnen" />
            </Link>
            <Link className="button button-ghost" to={`${PATHS.gateways}#selfhost`}>
              <T en="Run your own" de="Selbst betreiben" />
            </Link>
          </p>
        </div>
      </div>
    </Section>
  );
}

function Source({ ci, commit }: { ci: CiRun | null | undefined; commit: Commit | null | undefined }) {
  const lang = useLang();
  const repo = useGitHub("", "repo", parseRepo);
  return (
    <Section
      id="source"
      alt
      eyebrow={<T en="Source" de="Quellcode" />}
      title={<T en="Open source, and it stays that way" de="Quelloffen, und das bleibt so" />}
    >
      <div className="two-col reveal">
        <div>
          <p className="promise">
            <T
              en="Baylee will always be open source and free. No paywall, no paid features, no ads. Not now, not later. The code is under the GNU Affero General Public License, which means anyone who runs a modified gateway for other people must offer them its source too."
              de="Baylee bleibt immer quelloffen und kostenlos. Keine Bezahlschranke, keine kostenpflichtigen Funktionen, keine Werbung. Nicht jetzt, nicht später. Der Code steht unter der GNU Affero General Public License: Wer ein verändertes Gateway für andere betreibt, muss auch ihnen den Quellcode anbieten."
            />
          </p>
          <p>
            <T
              en={
                <>
                  It is written in Rust, one person&apos;s project with a lot of help from coding agents, and every
                  change goes through the same gate: formatting, lints, the test suite, and a validation of every card
                  against its printed text. Issues and pull requests are welcome;{" "}
                  <a href={`${GITHUB}/blob/main/AGENTS.md`} rel="noopener">
                    AGENTS.md
                  </a>{" "}
                  and{" "}
                  <a href={`${GITHUB}/blob/main/CLAUDE.md`} rel="noopener">
                    CLAUDE.md
                  </a>{" "}
                  are the working rules for people and agents alike.
                </>
              }
              de={
                <>
                  Geschrieben in Rust, das Projekt einer Person mit viel Hilfe von Coding-Agenten, und jede Änderung geht
                  durch dasselbe Gate: Formatierung, Lints, die Testsuite und eine Prüfung jeder Karte gegen ihren
                  gedruckten Text. Issues und Pull Requests sind willkommen;{" "}
                  <a href={`${GITHUB}/blob/main/AGENTS.md`} rel="noopener">
                    AGENTS.md
                  </a>{" "}
                  und{" "}
                  <a href={`${GITHUB}/blob/main/CLAUDE.md`} rel="noopener">
                    CLAUDE.md
                  </a>{" "}
                  sind die Arbeitsregeln für Menschen und Agenten.
                </>
              }
            />
          </p>
          <p className="links-row">
            <a className="button button-ghost" href={GITHUB} rel="noopener">
              github.com/AceVik/baylee
            </a>
            <a className="button button-ghost" href={DISCORD} rel="noopener">
              <T en="Community on Discord" de="Community auf Discord" />
            </a>
            <a className="button button-ghost" href={`${GITHUB}/blob/main/LICENSE`} rel="noopener">
              AGPL-3.0
            </a>
            <a className="button button-ghost" href={`${GITHUB}/blob/main/docs/architecture.md`} rel="noopener">
              <T en="Architecture" de="Architektur" />
            </a>
          </p>
        </div>
        <div className="repo-card glass">
          <h3>
            <T en="The repository right now" de="Das Repository gerade jetzt" />
          </h3>
          <dl className="kv">
            <div>
              <dt>
                <T en="CI on main" de="CI auf main" />
              </dt>
              <dd>
                {ci ? (
                  <>
                    <CiLink run={ci} />
                    <span className="small">
                      {" "}
                      · {relTime(ci.updatedAt, lang)} · {ci.sha.slice(0, 7)}
                    </span>
                  </>
                ) : (
                  "–"
                )}
              </dd>
            </div>
            <div>
              <dt>
                <T en="Last commit" de="Letzter Commit" />
              </dt>
              <dd>
                {commit ? (
                  <>
                    <a href={commit.url} rel="noopener">
                      {commit.sha.slice(0, 7)}
                    </a>{" "}
                    · {relTime(commit.when, lang)} · <span className="small">{commit.subject}</span>
                  </>
                ) : (
                  "–"
                )}
              </dd>
            </div>
            <div>
              <dt>
                <T en="Stars" de="Sterne" />
              </dt>
              <dd>
                {repo ? (
                  <a href={`${GITHUB}/stargazers`} rel="noopener">
                    {num(lang, repo.stars)}
                  </a>
                ) : (
                  "–"
                )}
              </dd>
            </div>
            <div>
              <dt>
                <T en="Open issues and pull requests" de="Offene Issues und Pull Requests" />
              </dt>
              <dd>
                {repo ? (
                  <a href={ISSUES} rel="noopener">
                    {num(lang, repo.openIssues)}
                  </a>
                ) : (
                  "–"
                )}
              </dd>
            </div>
          </dl>
          <p className="small">
            <T
              en="Read from GitHub's public API when the page loads, cached in this tab for a while. Nothing about you is sent anywhere but to GitHub."
              de="Beim Laden aus GitHubs öffentlicher API gelesen und in diesem Tab eine Weile gemerkt. Über dich geht nichts irgendwohin außer zu GitHub."
            />
          </p>
        </div>
      </div>
    </Section>
  );
}

export default function Home() {
  useTitle("Baylee – a table that knows the rules", "Baylee – ein Tisch, der die Regeln kennt");
  const ci = useGitHub(CI_PATH, "ci", parseCi);
  const commit = useGitHub(COMMIT_PATH, "commit", parseCommit);
  return (
    <>
      <Hero />
      <StatusStrip ci={ci} commit={commit} />
      <About />
      <Screens />
      <Opponents />
      <Online />
      <Source ci={ci} commit={commit} />
    </>
  );
}
