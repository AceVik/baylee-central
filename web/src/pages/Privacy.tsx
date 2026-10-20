// The website's own privacy notice (Art. 13 GDPR), drawn from the old
// site's (Baylee's web/site/datenschutz.html, written from the code on
// 13.10.2026) and from what this app does. A draft for the owner's lawyer to
// check before it is relied on (Baylee's docs/legal.md §4). It must change
// with the site: a new host in the CSP, a cookie, an access log or a new form
// means a new sentence here (README.md §"Privacy"). Never put the operator's
// private details here: the Impressum names the controller.

import type { ReactNode } from "react";

import { FEEDBACK_ORIGIN, GITHUB } from "../config";
import { T, useLang, useTitle } from "../i18n";
import { Link, PATHS } from "../router";

interface Part {
  id: string;
  title: [string, string];
  body: [ReactNode, ReactNode];
  basis?: [string, string];
}

const feedbackHost = new URL(FEEDBACK_ORIGIN).host;

const PARTS: Part[] = [
  {
    id: "controller",
    title: ["Who is responsible", "Verantwortlich"],
    body: [
      <>
        The controller is the private individual named in the <Link to={PATHS.impressum}>legal notice</Link>. You can reach
        me there or with the <Link to={PATHS.contact}>contact form</Link>.
      </>,
      <>
        Verantwortlich ist die im <Link to={PATHS.impressum}>Impressum</Link> genannte Privatperson. Du erreichst mich dort
        oder über das <Link to={PATHS.contact}>Kontaktformular</Link>.
      </>,
    ],
  },
  {
    id: "hosting",
    title: ["Hosting and server", "Hosting und Server"],
    body: [
      <>
        The website runs on a server rented from netcup GmbH in Germany. When you open a page, your browser sends the server
        what every request carries: your IP address, the address asked for, the time and what your browser says about
        itself. The server needs this to answer. The web server keeps <b>no access log</b> for this website, so these
        requests are not written down. The server&apos;s own operating messages (its system journal) can, in rare error
        cases, contain technical connection data such as an IP address; they are kept only as long as the journal keeps
        them.
      </>,
      <>
        Die Website läuft auf einem bei der netcup GmbH gemieteten Server in Deutschland. Wenn du eine Seite öffnest, schickt
        dein Browser dem Server, was jede Anfrage mitbringt: deine IP-Adresse, die angefragte Adresse, die Uhrzeit und was
        dein Browser über sich sagt. Der Server braucht das, um zu antworten. Der Webserver führt für diese Website{" "}
        <b>kein Zugriffsprotokoll</b>; diese Anfragen werden also nicht aufgeschrieben. Die Betriebsmeldungen des Servers
        (sein Systemjournal) können in seltenen Fehlerfällen technische Verbindungsdaten wie eine IP-Adresse enthalten; sie
        bleiben nur so lange, wie das Journal sie aufbewahrt.
      </>,
    ],
    basis: [
      "Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in delivering the website safely.",
      "Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse an einer sicheren Auslieferung der Website.",
    ],
  },
  {
    id: "storage",
    title: ["No cookies, no tracking", "Keine Cookies, kein Tracking"],
    body: [
      <>
        This website sets no cookies and uses no analytics, no tracking and no advertising. The fonts are served from this
        server; no font service is asked. Your browser keeps two things on your device, which never leave it: the language
        you chose (until you delete it) and, for ten minutes in this tab, GitHub&apos;s answers about the latest release, so
        the page does not ask again on every visit.
      </>,
      <>
        Diese Website setzt keine Cookies und nutzt keine Analyse, kein Tracking und keine Werbung. Die Schriften kommen von
        diesem Server; kein Schriftendienst wird gefragt. Dein Browser speichert zwei Dinge auf deinem Gerät, die es nie
        verlassen: die gewählte Sprache (bis du sie löschst) und, zehn Minuten lang in diesem Tab, GitHubs Antworten zur
        neuesten Version, damit die Seite nicht bei jedem Besuch neu fragt.
      </>,
    ],
    basis: [
      "Legal basis for this storage: § 25(2) no. 2 TDDDG; it is strictly necessary for what you asked the page to do.",
      "Rechtsgrundlage für diese Speicherung: § 25 Abs. 2 Nr. 2 TDDDG; sie ist für das, was du von der Seite willst, unbedingt erforderlich.",
    ],
  },
  {
    id: "github",
    title: ["GitHub: releases and project status", "GitHub: Versionen und Projektstand"],
    body: [
      <>
        To show the latest release, its files and the project&apos;s state, your browser asks GitHub&apos;s public API (
        <code>api.github.com</code>) when the start page or the download page loads, and a download comes straight from
        GitHub. Your browser then sends your IP address and the request to GitHub, Inc. in the USA. See{" "}
        <a href="https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement" rel="noopener">
          GitHub&apos;s privacy statement
        </a>
        .
      </>,
      <>
        Um die neueste Version, ihre Dateien und den Stand des Projekts zu zeigen, fragt dein Browser beim Laden der
        Startseite oder der Download-Seite GitHubs öffentliche API (<code>api.github.com</code>), und ein Download kommt
        direkt von GitHub. Dein Browser schickt dabei deine IP-Adresse und die Anfrage an GitHub, Inc. in den USA. Siehe{" "}
        <a href="https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement" rel="noopener">
          GitHubs Datenschutzerklärung
        </a>
        .
      </>,
    ],
    basis: [
      "Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in showing current downloads without hosting them myself.",
      "Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, aktuelle Downloads zu zeigen, ohne sie selbst zu hosten.",
    ],
  },
  {
    id: "scryfall",
    title: ["Card images from Scryfall", "Kartenbilder von Scryfall"],
    body: [
      <>
        Card images are not stored here. Only when you open a set in the card status, or show a card large, your browser
        loads its images directly from Scryfall&apos;s image server (<code>cards.scryfall.io</code>) and sends your IP
        address and the request to Scryfall, LLC in the USA. No referrer is sent. See{" "}
        <a href="https://scryfall.com/docs/privacy" rel="noopener">
          Scryfall&apos;s privacy policy
        </a>
        .
      </>,
      <>
        Kartenbilder liegen nicht hier. Erst wenn du im Kartenstatus ein Set öffnest oder eine Karte groß zeigst, lädt dein
        Browser ihre Bilder direkt von Scryfalls Bildserver (<code>cards.scryfall.io</code>) und schickt dabei deine
        IP-Adresse und die Anfrage an Scryfall, LLC in den USA. Ein Referrer wird nicht mitgeschickt. Siehe{" "}
        <a href="https://scryfall.com/docs/privacy" rel="noopener">
          Scryfalls Datenschutzerklärung
        </a>
        .
      </>,
    ],
    basis: [
      "Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in showing the cards the status speaks of.",
      "Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, die Karten zu zeigen, von denen der Status spricht.",
    ],
  },
  {
    id: "central",
    title: ["The gateway directory", "Das Gateway-Verzeichnis"],
    body: [
      <>
        When you open the <Link to={PATHS.gateways}>gateway directory</Link>, your browser asks Baylee Central, the
        project&apos;s account service, for the public list of gateways, through this website&apos;s own address. The
        request carries no cookie, no account and no identifier of yours; Central answers everyone the same list. Nothing
        else on this website talks to Central.
      </>,
      <>
        Wenn du das <Link to={PATHS.gateways}>Gateway-Verzeichnis</Link> öffnest, fragt dein Browser Baylee Central, den
        Kontodienst des Projekts, über die eigene Adresse dieser Website nach der öffentlichen Liste der Gateways. Die
        Anfrage trägt kein Cookie, kein Konto und keine Kennung von dir; Central antwortet allen mit derselben Liste. Sonst
        spricht nichts auf dieser Website mit Central.
      </>,
    ],
    basis: [
      "Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in showing where Baylee can be played online.",
      "Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, zu zeigen, wo man Baylee online spielen kann.",
    ],
  },
  {
    id: "contact",
    title: ["The contact form", "Das Kontaktformular"],
    body: [
      <>
        What you send goes to Baylee&apos;s feedback service (<code>{feedbackHost}</code>) and is stored there: your
        message, your name and e-mail address only if you give them, the page&apos;s language, the time and whether I have
        dealt with it. Your IP address is not stored; for an hour it is counted in memory only, to stop a flood of messages.
        Only I read the messages; nothing is passed on. A message is deleted 90 days after I have dealt with it and a year
        after it came at the latest, or earlier when you ask.
      </>,
      <>
        Was du sendest, geht an Baylees Feedback-Dienst (<code>{feedbackHost}</code>) und wird dort gespeichert: deine
        Nachricht, Name und E-Mail-Adresse nur, wenn du sie angibst, die Sprache der Seite, die Uhrzeit und ob ich sie
        erledigt habe. Deine IP-Adresse wird nicht gespeichert; eine Stunde lang wird sie nur im Arbeitsspeicher gezählt, um
        eine Flut von Nachrichten zu verhindern. Nur ich lese die Nachrichten; nichts wird weitergegeben. Eine Nachricht wird
        90 Tage, nachdem ich sie erledigt habe, gelöscht, spätestens ein Jahr nach ihrem Eingang, oder früher, wenn du darum
        bittest.
      </>,
    ],
    basis: [
      "Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in answering messages; where your message aims at a contract with me, Art. 6(1)(b) GDPR.",
      "Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, Nachrichten zu beantworten; zielt deine Nachricht auf einen Vertrag mit mir, Art. 6 Abs. 1 lit. b DSGVO.",
    ],
  },
  {
    id: "impressum",
    title: ["How the legal notice is protected", "Wie das Impressum geschützt ist"],
    body: [
      <>
        So that address harvesters do not collect my details, the legal notice shows them after your browser has solved a
        small computing puzzle. For that, the feedback service keeps in memory only, never on disk and never in a log: how
        many puzzles your IP address asked for (for an hour; an IPv6 address by its first half), whether it followed a link
        hidden from people (for 24 hours), and which puzzles were solved (five minutes, without an address). Three yes/no
        answers about your browser set the puzzle&apos;s difficulty and are not kept. Nobody is blocked: at worst the puzzle
        takes longer.
      </>,
      <>
        Damit Adresssammler meine Angaben nicht einsammeln, zeigt das Impressum sie, nachdem dein Browser ein kleines
        Rechenrätsel gelöst hat. Dafür hält der Feedback-Dienst nur im Arbeitsspeicher, nie auf der Festplatte und nie in
        einem Protokoll: wie viele Rätsel deine IP-Adresse angefragt hat (eine Stunde lang; eine IPv6-Adresse nach ihrer
        ersten Hälfte), ob sie einem für Menschen versteckten Link gefolgt ist (24 Stunden), und welche Rätsel gelöst wurden
        (fünf Minuten, ohne Adresse). Drei Ja/Nein-Antworten über deinen Browser bestimmen die Schwierigkeit und werden nicht
        aufbewahrt. Niemand wird gesperrt: Im ungünstigsten Fall dauert das Rätsel länger.
      </>,
    ],
    basis: [
      "Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in protecting my details from misuse.",
      "Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, meine Angaben vor Missbrauch zu schützen.",
    ],
  },
  {
    id: "links",
    title: ["Links to other sites", "Links zu anderen Seiten"],
    body: [
      <>
        Discord, GitHub, Scryfall, Wizards of the Coast and the websites of listed gateways are linked, not embedded:
        nothing is sent to them until you follow a link, and then their own privacy terms apply.
      </>,
      <>
        Discord, GitHub, Scryfall, Wizards of the Coast und die Websites gelisteter Gateways sind verlinkt, nicht
        eingebettet: An sie geht nichts, bis du einem Link folgst; dann gelten deren eigene Datenschutzbestimmungen.
      </>,
    ],
  },
  {
    id: "rights",
    title: ["Your rights", "Deine Rechte"],
    body: [
      <>
        <p>
          You have the right to access (Art. 15 GDPR), rectification (Art. 16), erasure (Art. 17), restriction of processing
          (Art. 18) and data portability (Art. 20), and you may object at any time to processing based on Art. 6(1)(f) (Art.
          21). Write to me through the contact form or the address in the legal notice.
        </p>
        <p>
          You may also complain to a data protection supervisory authority (Art. 77 GDPR), for example the one where you
          live, or the one responsible for me: the{" "}
          <a href="https://www.ldi.nrw.de/" rel="noopener">
            Landesbeauftragte für Datenschutz und Informationsfreiheit Nordrhein-Westfalen (LDI NRW)
          </a>
          .
        </p>
        <p>
          There is no automated decision-making within the meaning of Art. 22 GDPR: the puzzle&apos;s difficulty adapts by
          itself, but it decides nothing about you.
        </p>
      </>,
      <>
        <p>
          Du hast das Recht auf Auskunft (Art. 15 DSGVO), Berichtigung (Art. 16), Löschung (Art. 17), Einschränkung der
          Verarbeitung (Art. 18) und Datenübertragbarkeit (Art. 20), und du kannst einer Verarbeitung nach Art. 6 Abs. 1 lit.
          f jederzeit widersprechen (Art. 21). Schreib mir dafür über das Kontaktformular oder an die Anschrift im Impressum.
        </p>
        <p>
          Außerdem kannst du dich bei einer Datenschutz-Aufsichtsbehörde beschweren (Art. 77 DSGVO), etwa bei der an deinem
          Wohnort oder bei der für mich zuständigen: der{" "}
          <a href="https://www.ldi.nrw.de/" rel="noopener">
            Landesbeauftragten für Datenschutz und Informationsfreiheit Nordrhein-Westfalen (LDI NRW)
          </a>
          .
        </p>
        <p>
          Eine automatisierte Entscheidungsfindung im Sinne von Art. 22 DSGVO findet nicht statt: Die Schwierigkeit des
          Rätsels passt sich selbst an, entscheidet aber nichts über dich.
        </p>
      </>,
    ],
  },
];

export default function Privacy() {
  const lang = useLang();
  const de = lang === "de";
  useTitle("Privacy – Baylee", "Datenschutz – Baylee");
  return (
    <header className="page-head legal">
      <div className="section-inner">
        <p className="eyebrow">
          <T en="Privacy" de="Datenschutz" />
        </p>
        <h1>
          <T en="Privacy notice for this website" de="Datenschutzerklärung dieser Website" />
        </h1>
        <p className="section-lead">
          <T
            en={
              <>
                This page says what happens with your data when you visit this website. The game itself (the client and the
                online gateway) keeps its own records, listed in full in{" "}
                <a href={`${GITHUB}/blob/main/docs/privacy.md`} rel="noopener">
                  docs/privacy.md
                </a>
                .
              </>
            }
            de={
              <>
                Diese Seite sagt, was mit deinen Daten geschieht, wenn du diese Website besuchst. Das Spiel selbst (Client und
                Online-Gateway) führt eigene Aufzeichnungen; die vollständige Liste steht in{" "}
                <a href={`${GITHUB}/blob/main/docs/privacy.md`} rel="noopener">
                  docs/privacy.md
                </a>
                .
              </>
            }
          />
        </p>
        <nav className="toc" aria-label={de ? "Inhalt" : "Contents"}>
          <ol>
            {PARTS.map((p) => (
              <li key={p.id}>
                <a href={`#${p.id}`}>{de ? p.title[1] : p.title[0]}</a>
              </li>
            ))}
          </ol>
        </nav>
        {PARTS.map((p) => (
          <section key={p.id} id={p.id} className="legal-part" aria-labelledby={`${p.id}-title`}>
            <h2 id={`${p.id}-title`}>{de ? p.title[1] : p.title[0]}</h2>
            {p.id === "rights" ? de ? p.body[1] : p.body[0] : <p>{de ? p.body[1] : p.body[0]}</p>}
            {p.basis && <p className="small">{de ? p.basis[1] : p.basis[0]}</p>}
          </section>
        ))}
        <p className="small legal-date">
          <T en="As of 20 October 2026." de="Stand: 20. Oktober 2026." />
        </p>
      </div>
    </header>
  );
}
