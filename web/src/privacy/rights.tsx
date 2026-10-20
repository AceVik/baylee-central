// The notice's second half: the contact form, the legal notice's
// protection, links to other sites, and the visitor's rights.

import { FEEDBACK_ORIGIN } from "../config";
import type { Part } from "./part";

const feedbackHost = new URL(FEEDBACK_ORIGIN).host;

export const PERSONAL: Part[] = [
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
