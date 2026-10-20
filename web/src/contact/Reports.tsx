// How to report a bug or talk to the project: F8 in the client, Discord,
// GitHub issues, and what the client, the gateway and the feedback service
// keep.

import { DISCORD, GITHUB, ISSUES } from "../config";
import { Section } from "../components/Section";
import { T } from "../i18n";
import { Link, PATHS } from "../router";

export function Reports() {
  return (
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
  );
}
