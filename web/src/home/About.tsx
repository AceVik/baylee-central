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
  const de = lang === "de";

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
                {de ? "Offline & Online" : "Offline & Online"}
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
            <div
              className="ai-provider-badges"
              aria-label={de ? "Unterstützte Sprachmodelle" : "Supported language models"}
            >
              {LLM_PROVIDERS.map((p) => (
                <span key={p} className="ai-provider-chip">
                  {p}
                </span>
              ))}
            </div>
          </div>

          <div className="ai-card glass">
            <div className="ai-card-top">
              <span className="feat-icon">
                <Icon name="server" />
              </span>
              <span className="pill">
                {de ? "Für Gateway-Hosts" : "For Gateway Hosts"}
              </span>
            </div>
            <h3>
              {de
                ? "LLM-Spieler als Gateway-Host anbieten"
                : "Offer Hosted LLM Seats on Your Gateway"}
            </h3>
            <p>
              {de
                ? "Als Gateway-Betreiber kannst du vorkonfigurierte Sprachmodelle direkt auf deinem Server bereitstellen. Deine Gäste wählen am Tisch einfach eines deiner gehosteten Modelle als Mitspieler aus – ganz ohne eigenen API-Key."
                : "As a gateway operator, you can provide pre-configured language models directly on your server. Guests at a table simply pick one of your hosted models as an opponent—with no API key needed."}
            </p>
          </div>
        </div>

        <div className="ai-training-banner glass">
          <div className="ai-training-badge-col">
            <span className="pill ok ai-training-pill">
              <span className="dot ok live" aria-hidden="true" />
              {de ? "Aktuell im Training · Native Engine-KI" : "In Training · Native Engine AI"}
            </span>
            <h3>
              {de
                ? "Eigene Baylee AI – trainiert für echtes Spiel direkt auf der Engine"
                : "Native Baylee AI — trained to play directly on the engine like a real player"}
            </h3>
            <p>
              {de
                ? "Parallel zur Haus-KI und der externen LLM-Anbindung wird aktuell eine eigene Baylee AI direkt auf der deterministischen Regel-Engine trainiert. Sie lernt, komplexe Spielzustände, Prioritätsfenster, Kampfreihenfolgen und Deck-Synergien wie ein erfahrener menschlicher Spieler einzuschätzen – ganz ohne externe API-Latenzen."
                : "Alongside the House AI and external LLM bridge, a dedicated native Baylee AI is actively being trained directly on the deterministic rules engine. It learns to evaluate board states, priority windows, combat math, and deck synergies like an experienced human player—with zero external API latency."}
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
                {de ? "Gateway-Hosting & LLM-Setup" : "Gateway Hosting & LLM Setup"}
              </span>
            </Link>
          </div>
        </div>
      </div>
    </Section>
  );
}
