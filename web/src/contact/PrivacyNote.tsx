// What the form keeps, beside the button, as the privacy notice says it.

import { T } from "../i18n";
import { Link, PATHS } from "../router";

export function PrivacyNote() {
  return (
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
  );
}
