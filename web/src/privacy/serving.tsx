// The notice's first half: who is responsible, and what serving the site
// sends where (the server, the browser's own storage, GitHub, Scryfall,
// Central).

import { Link, PATHS } from "../router";
import type { Part } from "./part";

export const SERVING: Part[] = [
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
];
