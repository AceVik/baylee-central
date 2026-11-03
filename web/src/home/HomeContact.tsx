import { DISCORD, ISSUES, WHATSAPP } from "../config";
import { Icon } from "../components/icons";
import { Section } from "../components/Section";
import { ContactForm } from "../contact/ContactForm";
import { T, useLang, type Elems } from "../i18n";
import { Link, PATHS } from "../router";
import "../styles/forms.css";

const OPERATOR: Elems = { impressum: (words) => <Link to={PATHS.impressum}>{words}</Link> };

export function HomeContact() {
  const lang = useLang();
  const de = lang === "de";

  return (
    <Section id="contact" alt eyebrow={<T id="contact.eyebrow" />} title={<T id="contact.title" />}>
      <div className="two-col reveal">
        <div>
          <p>
            <T id="contact.intro" />
          </p>
          <p>
            <T id="contact.f8" />
          </p>
          <ul className="channels home-channels">
            <li>
              <a className="glass" href={DISCORD} target="_blank" rel="noopener">
                <Icon name="discord" />
                <span>
                  Discord
                  <small>
                    <T id="contact.discord" />
                  </small>
                </span>
              </a>
            </li>
            <li>
              <a className="glass" href={WHATSAPP} target="_blank" rel="noopener">
                <Icon name="whatsapp" />
                <span>
                  WhatsApp
                  <small>
                    {de ? "Community-Gruppe & schnelle Runden" : "Community group & quick matches"}
                  </small>
                </span>
              </a>
            </li>
            <li>
              <a className="glass" href={ISSUES} target="_blank" rel="noopener">
                <Icon name="github" />
                <span>
                  <T id="contact.issues" />
                  <small>
                    <T id="contact.issuesSub" />
                  </small>
                </span>
              </a>
            </li>
            <li>
              <Link className="glass" to={PATHS.contact}>
                <Icon name="chat" />
                <span>
                  {de ? "Berichte & Datenschutz" : "Reports & Privacy"}
                  <small>
                    {de ? "Details zu F8-Berichten & Daten" : "Details on F8 reports & data"}
                  </small>
                </span>
              </Link>
            </li>
          </ul>
          <p className="small">
            <T id="contact.operator" elems={OPERATOR} />
          </p>
        </div>
        <div className="contact-card glass">
          <ContactForm />
        </div>
      </div>
    </Section>
  );
}
