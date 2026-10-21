import { T, useTitle } from "../i18n";
import { Link } from "../router";

export function NotFound() {
  useTitle("title.notFound");
  return (
    <header className="page-head">
      <div className="section-inner">
        <p className="eyebrow">404</p>
        <h1>
          <T id="notFound.title" />
        </h1>
        <p>
          <Link to="/">
            <T id="notFound.back" />
          </Link>
        </p>
      </div>
    </header>
  );
}
