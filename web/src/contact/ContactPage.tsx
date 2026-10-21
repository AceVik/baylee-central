import { DISCORD, ISSUES } from "../config";
import { Icon } from "../components/icons";
import { T, useTitle, type Elems } from "../i18n";
import { Link, PATHS } from "../router";
import { ContactForm } from "./ContactForm";
import { Reports } from "./Reports";

const OPERATOR: Elems = { impressum: (words) => <Link to={PATHS.impressum}>{words}</Link> };

export default function Contact() {
  useTitle("title.contact");
  return (
    <>
      <header className="page-head">
        <div className="section-inner">
          <p className="eyebrow">
            <T id="contact.eyebrow" />
          </p>
          <h1>
            <T id="contact.title" />
          </h1>
          <div className="two-col">
            <div>
              <p>
                <T id="contact.intro" />
              </p>
              <p>
                <T id="contact.f8" />
              </p>
              <ul className="channels">
                <li>
                  <a className="glass" href={DISCORD} rel="noopener">
                    <Icon name="chat" />
                    <span>
                      Discord
                      <small>
                        <T id="contact.discord" />
                      </small>
                    </span>
                  </a>
                </li>
                <li>
                  <a className="glass" href={ISSUES} rel="noopener">
                    <Icon name="github" />
                    <span>
                      <T id="contact.issues" />
                      <small>
                        <T id="contact.issuesSub" />
                      </small>
                    </span>
                  </a>
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
        </div>
      </header>

      <Reports />
    </>
  );
}
