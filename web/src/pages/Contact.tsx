// Contact and reports. The form posts with fetch to the feedback service of
// Baylee's repository (`POST /site/contact`, docs/feedback.md §"The landing
// page's contact form"): the name and address only if given, the message,
// the page's language and an empty honeypot. Nothing is kept in the browser.

import { useEffect, useRef, useState, type FormEvent } from "react";

import { DISCORD, GITHUB, ISSUES, contactEndpoint } from "../config";
import { Icon } from "../components/icons";
import { Section } from "../components/Layout";
import { T, num, tt, useLang, useTitle } from "../i18n";
import { reducedMotion } from "../motion";
import { Link, PATHS } from "../router";

const MAX = 5000;
type SendState = "idle" | "sending" | "sent" | "error";
type Problem = "message" | "email" | null;

export function ContactForm({ endpoint }: { endpoint?: string }) {
  const lang = useLang();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [message, setMessage] = useState("");
  const [website, setWebsite] = useState("");
  const [state, setState] = useState<SendState>("idle");
  const [status, setStatus] = useState<{ bad: boolean; en: string; de: string } | null>(null);
  const [problem, setProblem] = useState<Problem>(null);
  const [done, setDone] = useState(false);
  const messageRef = useRef<HTMLTextAreaElement>(null);
  const emailRef = useRef<HTMLInputElement>(null);
  const doneRef = useRef<HTMLDivElement>(null);
  const timer = useRef(0);

  useEffect(() => () => window.clearTimeout(timer.current), []);
  useEffect(() => {
    if (done) doneRef.current?.focus();
  }, [done]);

  const say = (bad: boolean, en: string, de: string) => setStatus({ bad, en, de });
  const unreachable = () =>
    say(
      true,
      "The form could not reach us just now. Please try again later, or write on Discord or GitHub.",
      "Das Formular hat uns gerade nicht erreicht. Bitte versuch es später noch einmal, oder schreib auf Discord oder GitHub.",
    );

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    // never `disabled` while sending, which would drop the keyboard's focus to the page; a second press is ignored
    if (state === "sending") return;
    const text = message.trim();
    const mail = email.trim();
    if (!text) {
      setProblem("message");
      say(true, "Please write a message first.", "Bitte schreib zuerst eine Nachricht.");
      messageRef.current?.focus();
      return;
    }
    if (mail && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(mail)) {
      setProblem("email");
      say(
        true,
        "That e-mail address does not look right. Leave it empty if you want no answer.",
        "Diese E-Mail-Adresse sieht nicht richtig aus. Lass sie leer, wenn du keine Antwort willst.",
      );
      emailRef.current?.focus();
      return;
    }
    setProblem(null);
    setState("sending");
    say(false, "Sending…", "Wird gesendet …");
    fetch(endpoint ?? contactEndpoint(), {
      method: "POST",
      credentials: "omit",
      referrerPolicy: "no-referrer",
      mode: "cors",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: name.trim() || null, email: mail || null, message: text, language: lang, website }),
    }).then(
      (r) => {
        if (r.status === 201) {
          setState("sent");
          say(false, "Sent.", "Gesendet.");
          // a moment for the button's tick, then the thank-you
          timer.current = window.setTimeout(
            () => {
              setName("");
              setEmail("");
              setMessage("");
              setWebsite("");
              setStatus(null);
              setState("idle");
              setDone(true);
            },
            reducedMotion() ? 0 : 650,
          );
          return;
        }
        setState("error");
        if (r.status === 429)
          say(
            true,
            "Too many messages from here in the last hour. Please try again later.",
            "Zu viele Nachrichten von hier in der letzten Stunde. Bitte versuch es später noch einmal.",
          );
        else if (r.status === 400 || r.status === 413)
          say(
            true,
            "The message was not accepted. Is it shorter than 5,000 characters, and the address a real one?",
            "Die Nachricht wurde nicht angenommen. Ist sie kürzer als 5.000 Zeichen und die Adresse eine echte?",
          );
        else unreachable();
      },
      () => {
        setState("error");
        unreachable();
      },
    );
  };

  if (done) {
    return (
      <div className="contact-done" tabIndex={-1} ref={doneRef}>
        <svg className="check" viewBox="0 0 52 52" aria-hidden="true" focusable="false">
          <circle cx="26" cy="26" r="24" />
          <path d="M15 27l7 7 15-16" />
        </svg>
        <h3>
          <T en="Thank you, it is on its way." de="Danke, sie ist unterwegs." />
        </h3>
        <p className="small">
          <T
            en="If you left an address, the answer comes from a person, by mail."
            de="Wenn du eine Adresse dagelassen hast, kommt die Antwort von einem Menschen, per Mail."
          />
        </p>
        <button
          type="button"
          className="button button-ghost"
          onClick={() => {
            setDone(false);
            window.requestAnimationFrame(() => messageRef.current?.focus());
          }}
        >
          <T en="Write another" de="Noch eine schreiben" />
        </button>
      </div>
    );
  }

  const count = `${num(lang, message.length)} / ${num(lang, MAX)}`;
  return (
    <form className="contact-form" noValidate onSubmit={submit}>
      <div className="field">
        <label htmlFor="c-name">
          <T en="Name" de="Name" />{" "}
          <span className="opt">
            <T en="optional" de="freiwillig" />
          </span>
        </label>
        <input id="c-name" type="text" maxLength={100} autoComplete="name" value={name} onChange={(e) => setName(e.target.value)} />
      </div>
      <div className="field">
        <label htmlFor="c-email">
          <T en="E-mail" de="E-Mail" />{" "}
          <span className="opt">
            <T en="optional, only if you want an answer" de="freiwillig, nur wenn du eine Antwort willst" />
          </span>
        </label>
        <input
          ref={emailRef}
          id="c-email"
          type="email"
          maxLength={254}
          autoComplete="email"
          inputMode="email"
          spellCheck={false}
          value={email}
          aria-invalid={problem === "email" ? true : undefined}
          aria-describedby={problem === "email" ? "c-status" : undefined}
          onChange={(e) => {
            setEmail(e.target.value);
            if (problem === "email") setProblem(null);
          }}
        />
      </div>
      <div className="field">
        <label htmlFor="c-message">
          <T en="Message" de="Nachricht" />
        </label>
        <textarea
          ref={messageRef}
          id="c-message"
          maxLength={MAX}
          required
          rows={6}
          value={message}
          aria-invalid={problem === "message" ? true : undefined}
          aria-describedby={problem === "message" ? "c-status" : "c-count"}
          onChange={(e) => {
            setMessage(e.target.value);
            if (problem === "message") setProblem(null);
          }}
        />
        <div className="field-foot">
          <span className={`count${message.length > MAX * 0.9 ? " near" : ""}`} id="c-count">
            {count}
          </span>
        </div>
      </div>
      <div className="trap" aria-hidden="true">
        <label htmlFor="c-website">Website</label>
        <input id="c-website" type="text" tabIndex={-1} autoComplete="off" value={website} onChange={(e) => setWebsite(e.target.value)} />
      </div>
      <p className="privacy-note">
        <T
          en={
            <>
              I keep your message, and your name and e-mail address only if you give them, to read and answer it: no IP
              address, no cookie, no tracker. It is deleted 90 days after I have dealt with it, after a year at the latest.{" "}
              <Link to={`${PATHS.privacy}#contact`}>More in the privacy notice</Link>.
            </>
          }
          de={
            <>
              Ich speichere deine Nachricht, und Namen und E-Mail-Adresse nur, wenn du sie angibst, um sie zu lesen und zu
              beantworten: keine IP-Adresse, kein Cookie, kein Tracker. Sie wird 90 Tage, nachdem ich sie erledigt habe,
              gelöscht, spätestens nach einem Jahr. <Link to={`${PATHS.privacy}#contact`}>Mehr in der Datenschutzerklärung</Link>.
            </>
          }
        />
      </p>
      <div className="form-row">
        <button type="submit" className="button button-send" data-state={state} aria-disabled={state === "sending" ? true : undefined}>
          {state === "sending" ? (
            <>
              <svg className="button-icon i-spin" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <circle cx="12" cy="12" r="8.5" />
              </svg>
              <T en="Sending…" de="Wird gesendet …" />
            </>
          ) : state === "sent" ? (
            <>
              <Icon name="check" className="button-icon" />
              <T en="Sent" de="Gesendet" />
            </>
          ) : state === "error" ? (
            <>
              <Icon name="retry" className="button-icon" />
              <T en="Try again" de="Erneut senden" />
            </>
          ) : (
            <>
              <Icon name="send" className="button-icon" />
              <T en="Send" de="Senden" />
            </>
          )}
        </button>
        <output className={`form-status${status?.bad === true ? " bad" : ""}`} id="c-status">
          {status ? tt(lang, status.en, status.de) : ""}
        </output>
      </div>
    </form>
  );
}

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

      <Section
        id="reports"
        alt
        eyebrow={<T en="Reports" de="Meldungen" />}
        title={<T en="Found something wrong? Tell us" de="Etwas gefunden? Sag es uns" />}
      >
        <div className="two-col">
          <div>
            <p>
              <T
                en={
                  <>
                    Press <kbd>F8</kbd> at the table or in the lobby. A report carries its kind, your text, the client
                    version and, at a table, the game&apos;s id. Everything more has a checkbox of its own, and every box
                    starts ticked, so untick what you would rather keep before you press Send: system and hardware, the
                    table as you see it, your log (other players renamed &quot;Player A&quot;, &quot;Player B&quot;, …),
                    your settings, a picture of the Baylee window only, and at a table the game&apos;s whole record (every
                    move and every seat&apos;s cards, no names). <i>Show what is sent</i> shows every byte. Session tokens
                    are never sent.
                  </>
                }
                de={
                  <>
                    Drück <kbd>F8</kbd> am Tisch oder in der Lobby. Eine Meldung enthält ihre Art, deinen Text, die
                    Client-Version und am Tisch die Spiel-ID. Alles Weitere hat ein eigenes Kästchen, und jedes Kästchen
                    ist anfangs angehakt; nimm vor dem Senden das Häkchen weg, wo du etwas lieber behältst: System und
                    Hardware, der Tisch wie du ihn siehst, dein Protokoll (andere Spieler als „Player A“, „Player B“, …),
                    deine Einstellungen, ein Bild nur des Baylee-Fensters und am Tisch die ganze Aufzeichnung des Spiels
                    (jeder Zug und die Karten jedes Platzes, keine Namen). <i>Zeigen, was gesendet wird</i> zeigt jedes
                    Byte. Sitzungs-Tokens werden nie gesendet.
                  </>
                }
              />
            </p>
            <p>
              <T
                en={
                  <>
                    While you play offline, the client says once a minute that a game is running: no account, no device
                    id, nothing about the game. <i>Send anonymous usage count</i> under Settings › Privacy &amp; Data
                    switches it off.
                  </>
                }
                de={
                  <>
                    Beim Offline-Spielen sagt der Client einmal pro Minute, dass ein Spiel läuft: kein Konto, keine
                    Geräte-ID, nichts über das Spiel. <i>Anonyme Nutzungszahl senden</i> unter Einstellungen › Datenschutz
                    &amp; Daten schaltet das ab.
                  </>
                }
              />
            </p>
          </div>
          <div>
            <p>
              <T
                en={
                  <>
                    Want to talk, find opponents or hear first about new builds? Join the{" "}
                    <a href={DISCORD} rel="noopener">
                      Baylee community on Discord
                    </a>
                    .
                  </>
                }
                de={
                  <>
                    Lust zu reden, Gegner zu finden oder als Erstes von neuen Builds zu erfahren? Komm in die{" "}
                    <a href={DISCORD} rel="noopener">
                      Baylee-Community auf Discord
                    </a>
                    .
                  </>
                }
              />
            </p>
            <p>
              <T
                en={
                  <>
                    Rules questions, wrong card behaviour, ideas:{" "}
                    <a href={ISSUES} rel="noopener">
                      open an issue on GitHub
                    </a>
                    . A card that resolved wrongly is the most useful report we can get, because every card has a test and
                    yours becomes one.
                  </>
                }
                de={
                  <>
                    Regelfragen, falsches Kartenverhalten, Ideen:{" "}
                    <a href={ISSUES} rel="noopener">
                      eröffne ein Issue auf GitHub
                    </a>
                    . Eine Karte, die falsch verrechnet wurde, ist die nützlichste Meldung, die wir bekommen können, denn
                    jede Karte hat einen Test, und deine wird einer.
                  </>
                }
              />
            </p>
            <p>
              <T
                en={
                  <>
                    <a href={`${GITHUB}/blob/main/docs/privacy.md`} rel="noopener">
                      docs/privacy.md
                    </a>{" "}
                    lists everything the client, the gateway and the feedback service keep. This site keeps nothing about
                    you: no cookies, no analytics, no fonts from anyone else&apos;s server.{" "}
                    <Link to={PATHS.privacy}>The privacy notice</Link> names every other host it talks to, and when.
                  </>
                }
                de={
                  <>
                    <a href={`${GITHUB}/blob/main/docs/privacy.md`} rel="noopener">
                      docs/privacy.md
                    </a>{" "}
                    listet alles, was Client, Gateway und Feedback-Dienst speichern. Diese Seite speichert nichts über
                    dich: keine Cookies, keine Analyse, keine Schriften von fremden Servern.{" "}
                    <Link to={PATHS.privacy}>Die Datenschutzerklärung</Link> nennt jeden anderen Host, mit dem sie spricht,
                    und wann.
                  </>
                }
              />
            </p>
          </div>
        </div>
      </Section>
    </>
  );
}
