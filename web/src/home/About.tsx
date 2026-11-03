import { GITHUB } from "../config";
import { T, t, useLang } from "../i18n";
import { Icon } from "../components/icons";
import { Section } from "../components/Section";
import { Link, PATHS } from "../router";
import { FEATURES } from "./features";

const LLM_PROVIDERS = [
  "Google Gemini",
  "OpenAI ChatGPT",
  "Anthropic Claude",
  "DeepSeek",
  "LM Studio",
] as const;

export function About() {
  const lang = useLang();

  return (
    <Section id="about" eyebrow={<T id="about.eyebrow" />} title={<T id="about.title" />}>
      <div className="two-col about-arch-grid reveal">
        <div className="about-arch-card glass">
          <div className="about-arch-head">
            <span className="feat-icon">
              <Icon name="shield" />
            </span>
            <h3>
              <T id="about.engineTitle" />
            </h3>
          </div>
          <p>
            <T id="about.engine" />
          </p>
          <p>
            <T id="about.deterministic" />
          </p>
        </div>
        <div className="about-arch-card glass">
          <div className="about-arch-head">
            <span className="feat-icon">
              <Icon name="grid" />
            </span>
            <h3>
              <T id="about.tableTitle" />
            </h3>
          </div>
          <p>
            <T id="about.table" />
          </p>
          <p>
            <T id="about.keys" />
          </p>
        </div>
      </div>

      <ul className="features stagger">
        {FEATURES.map((f) => (
          <li key={f.icon} className="glass reveal">
            <div className="feat-head">
              <span className="feat-icon">
                <Icon name={f.icon} />
              </span>
              <h3>
                <T id={f.title} />
              </h3>
            </div>
            <p>
              <T id={f.text} />
            </p>
          </li>
        ))}
      </ul>

      <div id="ai" className="ai-showcase reveal">
        <div className="ai-showcase-head">
          <p className="eyebrow">
            <T id="ai.eyebrow" />
          </p>
          <h2 className="ai-showcase-title">
            <T id="ai.title" />
          </h2>
        </div>

        <div className="ai-cards-grid">
          <div className="ai-card glass">
            <div className="ai-card-top">
              <span className="feat-icon">
                <Icon name="chip" />
              </span>
              <span className="pill">
                <T id="ai.housePill" />
              </span>
            </div>
            <h3>
              <T id="ai.houseTitle" />
            </h3>
            <p>
              <T id="ai.house" />
            </p>
          </div>

          <div className="ai-card glass">
            <div className="ai-card-top">
              <span className="feat-icon">
                <Icon name="globe" />
              </span>
              <span className="pill gold">Seat-Bridge · Bring Your Own Model</span>
            </div>
            <h3>
              <T id="ai.modelTitle" />
            </h3>
            <p>
              <T id="ai.model" />
            </p>
            <ul className="ai-provider-badges" aria-label={t(lang, "ai.providers")}>
              {LLM_PROVIDERS.map((p) => (
                <li key={p} className="ai-provider-chip">
                  {p}
                </li>
              ))}
            </ul>
          </div>

          <div className="ai-card glass">
            <div className="ai-card-top">
              <span className="feat-icon">
                <Icon name="server" />
              </span>
              <span className="pill">
                <T id="ai.hostsPill" />
              </span>
            </div>
            <h3>
              <T id="ai.hostsTitle" />
            </h3>
            <p>
              <T id="ai.hosts" />
            </p>
          </div>
        </div>

        <div className="ai-training-banner glass">
          <div className="ai-training-badge-col">
            <span className="pill ok ai-training-pill">
              <span className="dot ok live" aria-hidden="true" />
              <T id="ai.trainingPill" />
            </span>
            <h3>
              <T id="ai.trainingTitle" />
            </h3>
            <p>
              <T id="ai.training" />
            </p>
          </div>
          <div className="ai-training-actions">
            <a
              className="button button-ghost"
              href={`${GITHUB}${t(lang, "link.readmeModel")}`}
              target="_blank"
              rel="noopener"
            >
              <Icon name="external" />
              <span>
                <T id="ai.setup" />
              </span>
            </a>
            <Link className="button button-ghost" to={`${PATHS.gateways}#selfhost`}>
              <Icon name="server" />
              <span>
                <T id="ai.hostingLink" />
              </span>
            </Link>
          </div>
        </div>
      </div>
    </Section>
  );
}
