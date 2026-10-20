import { T } from "../i18n";
import { Icon } from "../components/icons";
import { Section } from "../components/Section";
import { Link, PATHS } from "../router";

export function Online() {
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
