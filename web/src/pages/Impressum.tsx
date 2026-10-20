// The legal notice (§ 5 DDG). The provider's details are in no file of this
// repository and in nothing a crawler reads as text: on the visitor's click
// the browser solves a small proof of work, the feedback service hands out
// the blob for it, and the page draws the details scrambled (README.md
// §"The Impressum"). Not indexed: the page says so, robots.txt disallows it,
// and the server adds X-Robots-Tag.

import { useEffect, useMemo, useRef, useState } from "react";

import { impressumEndpoint } from "../config";
import { decode, type Fields } from "../impressum/blob";
import { fetchBlob, type Solver } from "../impressum/fetch";
import { Scrambled, readLines, readRun, scatter, type Glyph } from "../impressum/Scrambled";
import { T, tt, useLang, useTitle } from "../i18n";
import { Link, PATHS } from "../router";
import "../styles/impressum.css";

type State = "idle" | "working" | "shown" | "failed";

function ready(): boolean {
  return (
    typeof crypto !== "undefined" &&
    typeof crypto.subtle !== "undefined" &&
    typeof crypto.getRandomValues === "function" &&
    typeof Worker === "function" &&
    typeof TextDecoder === "function"
  );
}

function useNoIndex(): void {
  useEffect(() => {
    const meta = document.createElement("meta");
    meta.name = "robots";
    meta.content = "noindex, noarchive, nosnippet";
    document.head.append(meta);
    return () => meta.remove();
  }, []);
}

/** The scrambled run inside `el`, read back for a click handler. */
function run(el: HTMLElement | null): string {
  return el?.firstElementChild ? readRun(el.firstElementChild) : "";
}

function ContactLink({ children }: { children: string }) {
  return <Link to={PATHS.contact}>{children}</Link>;
}

interface Drawn {
  name: Glyph[];
  street: Glyph[];
  postcode: Glyph[];
  city: Glyph[];
  country_de: Glyph[];
  country_en: Glyph[];
  email: Glyph[];
  phone: Glyph[];
}

function draw(f: Fields): Drawn {
  return {
    name: scatter(f.name),
    street: scatter(f.street),
    postcode: scatter(f.postcode),
    city: scatter(f.city),
    country_de: scatter(f.country_de),
    country_en: scatter(f.country_en),
    email: scatter(f.email),
    phone: scatter(f.phone),
  };
}

function Details({ drawn }: { drawn: Drawn }) {
  const lang = useLang();
  const address = useRef<HTMLDivElement>(null);
  const mail = useRef<HTMLSpanElement>(null);
  const phone = useRef<HTMLSpanElement>(null);
  const first = useRef<HTMLButtonElement>(null);
  const [addressSaid, setAddressSaid] = useState("");
  const [contactSaid, setContactSaid] = useState("");

  useEffect(() => first.current?.focus(), []);

  const copy = (say: (s: string) => void, read: () => string) => {
    if (!navigator.clipboard?.writeText) {
      say(
        tt(
          lang,
          "Copying is not possible in this browser; please use the contact form.",
          "Kopieren geht in diesem Browser nicht; bitte nutze das Kontaktformular.",
        ),
      );
      return;
    }
    navigator.clipboard.writeText(read()).then(
      () => say(tt(lang, "Copied.", "Kopiert.")),
      () => say(tt(lang, "Copying did not work; please use the contact form.", "Kopieren hat nicht geklappt; bitte nutze das Kontaktformular.")),
    );
  };

  return (
    <>
      <p className="vh">
        <T
          en="The address is drawn so that address harvesters cannot read it, and a screen reader cannot either. The button below copies it to the clipboard; or write to me with the contact form."
          de="Die Anschrift ist so gezeichnet, dass Adresssammler sie nicht lesen können, und deshalb auch kein Screenreader. Der Knopf darunter kopiert sie in die Zwischenablage; oder schreib mir über das Kontaktformular."
        />
      </p>
      <div className="imp-address scr-block" aria-hidden="true" ref={address}>
        <div className="imp-line">
          <Scrambled glyphs={drawn.name} />
        </div>
        <div className="imp-line">
          <Scrambled glyphs={drawn.street} />
        </div>
        <div className="imp-line">
          <Scrambled glyphs={drawn.postcode} /> <Scrambled glyphs={drawn.city} />
        </div>
        <div className="imp-line">
          <Scrambled glyphs={lang === "de" ? drawn.country_de : drawn.country_en} />
        </div>
      </div>
      <div className="imp-actions">
        <button
          ref={first}
          type="button"
          className="button button-ghost"
          onClick={() => copy(setAddressSaid, () => (address.current ? readLines(address.current) : ""))}
        >
          <T en="Copy address" de="Anschrift kopieren" />
        </button>
      </div>
      <output className="imp-status">{addressSaid}</output>

      <h2>
        <T en="Contact" de="Kontakt" />
      </h2>
      <dl className="imp-dl">
        <dt>
          <T en="E-mail" de="E-Mail" />
        </dt>
        <dd>
          <span className="scr-block" aria-hidden="true" ref={mail}>
            <Scrambled glyphs={drawn.email} />
          </span>
          <span className="imp-actions">
            <button
              type="button"
              className="button button-ghost"
              onClick={() => {
                window.location.href = `mailto:${run(mail.current)}`;
              }}
            >
              <T en="Write an e-mail" de="E-Mail schreiben" />
            </button>
            <button type="button" className="button button-ghost" onClick={() => copy(setContactSaid, () => run(mail.current))}>
              <T en="Copy e-mail address" de="E-Mail-Adresse kopieren" />
            </button>
          </span>
        </dd>
        <dt>
          <T en="Telephone" de="Telefon" />
        </dt>
        <dd>
          <span className="scr-block" aria-hidden="true" ref={phone}>
            <Scrambled glyphs={drawn.phone} />
          </span>
          <span className="imp-actions">
            <button
              type="button"
              className="button button-ghost"
              onClick={() => {
                window.location.href = `tel:${run(phone.current).replace(/[^+\d]/g, "")}`;
              }}
            >
              <T en="Call" de="Anrufen" />
            </button>
            <button type="button" className="button button-ghost" onClick={() => copy(setContactSaid, () => run(phone.current))}>
              <T en="Copy number" de="Nummer kopieren" />
            </button>
          </span>
        </dd>
      </dl>
      <output className="imp-status">{contactSaid}</output>
    </>
  );
}

export function ImpressumBox({ solver }: { solver?: Solver }) {
  const lang = useLang();
  const [state, setState] = useState<State>("idle");
  const [progress, setProgress] = useState(0);
  const [fields, setFields] = useState<Fields | null>(null);
  const drawn = useMemo(() => (fields ? draw(fields) : null), [fields]);
  const capable = useMemo(() => ready(), []);

  const show = () => {
    if (state === "working") return;
    setState("working");
    setProgress(0);
    fetchBlob(impressumEndpoint(), setProgress, solver)
      .then(decode)
      .then((f) => {
        setFields(f);
        setState("shown");
      })
      .catch(() => setState("failed"));
  };

  return (
    <div className="imp-box glass">
      {state !== "shown" && (
        <p className="imp-why">
          <T
            en="To keep address harvesters away, the details load only when you ask for them; your browser works out a small puzzle for it, which takes a moment."
            de="Damit Adresssammler leer ausgehen, werden die Angaben erst auf deinen Wunsch geladen; dein Browser löst dafür ein kleines Rätsel, das einen Moment dauert."
          />
        </p>
      )}
      {capable && (state === "idle" || state === "working" || state === "failed") && (
        <button type="button" className="button" onClick={show} aria-busy={state === "working"} aria-describedby="imp-show-status">
          <T en="Show the details" de="Impressum anzeigen" />
        </button>
      )}
      {state === "working" && <progress className="imp-progress" max={1} value={progress} aria-label={tt(lang, "Puzzle", "Rätsel")} />}
      <output className="imp-status" id="imp-show-status">
        {state === "working" &&
          tt(lang, "One moment, your browser is working out a small puzzle …", "Einen Moment, dein Browser löst ein kleines Rätsel …")}
      </output>
      {(state === "failed" || !capable) && (
        <p className="imp-failed">
          <T
            en={
              <>
                The details could not be loaded. Please reload the page or use the{" "}
                <ContactLink>contact form</ContactLink>.
              </>
            }
            de={
              <>
                Die Angaben konnten nicht geladen werden. Bitte lade die Seite neu oder nutze das{" "}
                <ContactLink>Kontaktformular</ContactLink>.
              </>
            }
          />
        </p>
      )}
      {/* the honeypot: hidden from people, disallowed in robots.txt; an address that follows it gets invented details */}
      <a className="hp" href="/site/impressum/kontakt" tabIndex={-1} aria-hidden="true" rel="nofollow">
        Kontakt
      </a>
      {drawn && <Details drawn={drawn} />}
    </div>
  );
}

export default function Impressum() {
  useTitle("Legal notice – Baylee", "Impressum – Baylee");
  useNoIndex();
  return (
    <header className="page-head legal">
      <div className="section-inner">
        <p className="eyebrow">
          <T en="Legal notice" de="Impressum" />
        </p>
        <h1>
          <T en="Legal notice" de="Impressum" />
        </h1>
        <h2>
          <T en="Information pursuant to § 5 DDG" de="Angaben gemäß § 5 DDG" />
        </h2>
        <p>
          <T
            en="This website is a private, non-commercial project of a private individual. Baylee is free and open source; there is no advertising and nothing to buy."
            de="Diese Website ist ein privates, nicht kommerzielles Projekt einer Privatperson. Baylee ist kostenlos und quelloffen; es gibt keine Werbung und nichts zu kaufen."
          />
        </p>
        <ImpressumBox />
        <p>
          <T
            en={
              <>
                You can also write to me with the <ContactLink>contact form</ContactLink>.
              </>
            }
            de={
              <>
                Du erreichst mich außerdem über das <ContactLink>Kontaktformular</ContactLink>.
              </>
            }
          />
        </p>
        <h2>
          <T en="Dispute resolution" de="Streitbeilegung" />
        </h2>
        <p>
          <T
            en="This is a free, non-commercial offer of a private individual. I am neither obliged nor willing to take part in dispute resolution proceedings before a consumer arbitration board."
            de="Dies ist ein kostenloses, nicht kommerzielles Angebot einer Privatperson. Ich bin weder verpflichtet noch bereit, an Streitbeilegungsverfahren vor einer Verbraucherschlichtungsstelle teilzunehmen."
          />
        </p>
        <p className="small">
          <T en="How this site handles your data: " de="Wie diese Seite mit deinen Daten umgeht: " />
          <Link to={PATHS.privacy}>
            <T en="the privacy notice" de="die Datenschutzerklärung" />
          </Link>
          .{" "}
          <T en="What the client, the gateway and the feedback service keep about you: " de="Was Client, Gateway und Feedback-Dienst über dich speichern: " />
          <a href="https://github.com/AceVik/baylee/blob/main/docs/privacy.md" rel="noopener">
            docs/privacy.md
          </a>
          .
        </p>
      </div>
    </header>
  );
}
