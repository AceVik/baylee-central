// The provider's details, drawn scrambled (./Scrambled.tsx), with buttons
// that copy them, write an e-mail or call: the characters are read back only
// in those click handlers, and no href ever holds an address.

import { useEffect, useRef, useState } from "react";

import { T, tt, useLang } from "../i18n";
import type { Fields } from "./blob";
import { Scrambled, readLines, readRun, scatter, type Glyph } from "./Scrambled";

/** The scrambled run inside `el`, read back for a click handler. */
function run(el: HTMLElement | null): string {
  return el?.firstElementChild ? readRun(el.firstElementChild) : "";
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

export function draw(f: Fields): Drawn {
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

export function Details({ drawn }: { drawn: Drawn }) {
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
