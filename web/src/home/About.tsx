import { T } from "../i18n";
import { Icon } from "../components/icons";
import { Section } from "../components/Section";
import { FEATURES } from "./features";

export function About() {
  return (
    <Section id="about" eyebrow={<T id="about.eyebrow" />} title={<T id="about.title" />}>
      <div className="two-col reveal">
        <div>
          <h3>
            <T id="about.engineTitle" />
          </h3>
          <p>
            <T id="about.engine" />
          </p>
          <p>
            <T id="about.deterministic" />
          </p>
        </div>
        <div>
          <h3>
            <T id="about.tableTitle" />
          </h3>
          <p>
            <T id="about.table" />
          </p>
          <p>
            <T id="about.keys" />
          </p>
        </div>
      </div>
      <ul className="features">
        {FEATURES.map((f) => (
          <li key={f.icon} className="glass reveal">
            <span className="feat-icon">
              <Icon name={f.icon} />
            </span>
            <h3>
              <T id={f.title} />
            </h3>
            <p>
              <T id={f.text} />
            </p>
          </li>
        ))}
      </ul>
    </Section>
  );
}
