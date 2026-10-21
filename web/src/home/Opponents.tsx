import { GITHUB } from "../config";
import { T, t, useLang } from "../i18n";
import { Section } from "../components/Section";

export function Opponents() {
  const lang = useLang();
  return (
    <Section id="ai" alt eyebrow={<T id="ai.eyebrow" />} title={<T id="ai.title" />}>
      <div className="two-col reveal">
        <div>
          <h3>
            <T id="ai.houseTitle" />
          </h3>
          <p>
            <T id="ai.house" />
          </p>
        </div>
        <div>
          <h3>
            <T id="ai.modelTitle" />
          </h3>
          <p>
            <T id="ai.model" />
          </p>
          <p>
            <T id="ai.gateway" />
          </p>
          <p className="small">
            <a href={`${GITHUB}${t(lang, "link.readmeModel")}`} rel="noopener">
              <T id="ai.setup" />
            </a>
          </p>
        </div>
      </div>
    </Section>
  );
}
