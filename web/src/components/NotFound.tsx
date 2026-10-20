import { T, useTitle } from "../i18n";
import { Link } from "../router";

export function NotFound() {
  useTitle("Not found – Baylee", "Nicht gefunden – Baylee");
  return (
    <header className="page-head">
      <div className="section-inner">
        <p className="eyebrow">404</p>
        <h1>
          <T en="There is no such page" de="Diese Seite gibt es nicht" />
        </h1>
        <p>
          <Link to="/">
            <T en="Back to the start page" de="Zurück zur Startseite" />
          </Link>
        </p>
      </div>
    </header>
  );
}
