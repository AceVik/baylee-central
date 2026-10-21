import { T, type Elems } from "../i18n";
import { Icon } from "../components/icons";
import { Section } from "../components/Section";
import { Link, PATHS } from "../router";

const OWNER: Elems = {
  owner: (words) => (
    <a href="https://github.com/AceVik" rel="noopener">
      {words}
    </a>
  ),
};

export function Online() {
  return (
    <Section id="beta" eyebrow={<T id="beta.eyebrow" />} title={<T id="beta.title" />}>
      <div className="two-col reveal">
        <div>
          <p>
            <T id="beta.sanctuary" />
          </p>
          <p>
            <T id="beta.signup" />
          </p>
          <p>
            <T id="beta.keys" elems={OWNER} />
          </p>
        </div>
        <div className="link-card glass">
          <span className="feat-icon">
            <Icon name="server" />
          </span>
          <h3>
            <T id="beta.listTitle" />
          </h3>
          <p>
            <T id="beta.list" />
          </p>
          <p className="link-card-actions">
            <Link className="button" to={PATHS.gateways}>
              <T id="beta.open" />
            </Link>
            <Link className="button button-ghost" to={`${PATHS.gateways}#selfhost`}>
              <T id="beta.own" />
            </Link>
          </p>
        </div>
      </div>
    </Section>
  );
}
