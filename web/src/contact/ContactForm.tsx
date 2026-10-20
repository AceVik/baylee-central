// The contact form. It posts with fetch to the feedback service of Baylee's
// repository (`POST /site/contact`, docs/feedback.md §"The landing page's
// contact form"): the name and address only if given, the message, the
// page's language and an empty honeypot. Nothing is kept in the browser.

import { useEffect, useRef, useState, type FormEvent } from "react";

import { contactEndpoint } from "../config";
import { T, num, tt, useLang } from "../i18n";
import { reducedMotion } from "../motion";
import { PrivacyNote } from "./PrivacyNote";
import { SendButton, type SendState } from "./SendButton";
import { ThankYou } from "./ThankYou";

const MAX = 5000;

type Problem = "message" | "email" | null;

export function ContactForm({ endpoint }: { endpoint?: string }) {
  const lang = useLang();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [message, setMessage] = useState("");
  const [website, setWebsite] = useState("");
  const [state, setState] = useState<SendState>("idle");
  const [status, setStatus] = useState<{ bad: boolean; en: string; de: string } | null>(null);
  const [problem, setProblem] = useState<Problem>(null);
  const [done, setDone] = useState(false);
  const messageRef = useRef<HTMLTextAreaElement>(null);
  const emailRef = useRef<HTMLInputElement>(null);
  const timer = useRef(0);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  const say = (bad: boolean, en: string, de: string) => setStatus({ bad, en, de });
  const unreachable = () =>
    say(
      true,
      "The form could not reach us just now. Please try again later, or write on Discord or GitHub.",
      "Das Formular hat uns gerade nicht erreicht. Bitte versuch es später noch einmal, oder schreib auf Discord oder GitHub.",
    );

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    // never `disabled` while sending, which would drop the keyboard's focus to the page; a second press is ignored
    if (state === "sending") return;
    const text = message.trim();
    const mail = email.trim();
    if (!text) {
      setProblem("message");
      say(true, "Please write a message first.", "Bitte schreib zuerst eine Nachricht.");
      messageRef.current?.focus();
      return;
    }
    if (mail && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(mail)) {
      setProblem("email");
      say(
        true,
        "That e-mail address does not look right. Leave it empty if you want no answer.",
        "Diese E-Mail-Adresse sieht nicht richtig aus. Lass sie leer, wenn du keine Antwort willst.",
      );
      emailRef.current?.focus();
      return;
    }
    setProblem(null);
    setState("sending");
    say(false, "Sending…", "Wird gesendet …");
    fetch(endpoint ?? contactEndpoint(), {
      method: "POST",
      credentials: "omit",
      referrerPolicy: "no-referrer",
      mode: "cors",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: name.trim() || null, email: mail || null, message: text, language: lang, website }),
    }).then(
      (r) => {
        if (r.status === 201) {
          setState("sent");
          say(false, "Sent.", "Gesendet.");
          // a moment for the button's tick, then the thank-you
          timer.current = window.setTimeout(
            () => {
              setName("");
              setEmail("");
              setMessage("");
              setWebsite("");
              setStatus(null);
              setState("idle");
              setDone(true);
            },
            reducedMotion() ? 0 : 650,
          );
          return;
        }
        setState("error");
        if (r.status === 429)
          say(
            true,
            "Too many messages from here in the last hour. Please try again later.",
            "Zu viele Nachrichten von hier in der letzten Stunde. Bitte versuch es später noch einmal.",
          );
        else if (r.status === 400 || r.status === 413)
          say(
            true,
            "The message was not accepted. Is it shorter than 5,000 characters, and the address a real one?",
            "Die Nachricht wurde nicht angenommen. Ist sie kürzer als 5.000 Zeichen und die Adresse eine echte?",
          );
        else unreachable();
      },
      () => {
        setState("error");
        unreachable();
      },
    );
  };

  if (done) {
    return (
      <ThankYou
        onAgain={() => {
          setDone(false);
          window.requestAnimationFrame(() => messageRef.current?.focus());
        }}
      />
    );
  }

  const count = `${num(lang, message.length)} / ${num(lang, MAX)}`;
  return (
    <form className="contact-form" noValidate onSubmit={submit}>
      <div className="field">
        <label htmlFor="c-name">
          <T en="Name" de="Name" />{" "}
          <span className="opt">
            <T en="optional" de="freiwillig" />
          </span>
        </label>
        <input id="c-name" type="text" maxLength={100} autoComplete="name" value={name} onChange={(e) => setName(e.target.value)} />
      </div>
      <div className="field">
        <label htmlFor="c-email">
          <T en="E-mail" de="E-Mail" />{" "}
          <span className="opt">
            <T en="optional, only if you want an answer" de="freiwillig, nur wenn du eine Antwort willst" />
          </span>
        </label>
        <input
          ref={emailRef}
          id="c-email"
          type="email"
          maxLength={254}
          autoComplete="email"
          inputMode="email"
          spellCheck={false}
          value={email}
          aria-invalid={problem === "email" ? true : undefined}
          aria-describedby={problem === "email" ? "c-status" : undefined}
          onChange={(e) => {
            setEmail(e.target.value);
            if (problem === "email") setProblem(null);
          }}
        />
      </div>
      <div className="field">
        <label htmlFor="c-message">
          <T en="Message" de="Nachricht" />
        </label>
        <textarea
          ref={messageRef}
          id="c-message"
          maxLength={MAX}
          required
          rows={6}
          value={message}
          aria-invalid={problem === "message" ? true : undefined}
          aria-describedby={problem === "message" ? "c-status" : "c-count"}
          onChange={(e) => {
            setMessage(e.target.value);
            if (problem === "message") setProblem(null);
          }}
        />
        <div className="field-foot">
          <span className={`count${message.length > MAX * 0.9 ? " near" : ""}`} id="c-count">
            {count}
          </span>
        </div>
      </div>
      <div className="trap" aria-hidden="true">
        <label htmlFor="c-website">Website</label>
        <input id="c-website" type="text" tabIndex={-1} autoComplete="off" value={website} onChange={(e) => setWebsite(e.target.value)} />
      </div>
      <PrivacyNote />
      <div className="form-row">
        <SendButton state={state} />
        <output className={`form-status${status?.bad === true ? " bad" : ""}`} id="c-status">
          {status ? tt(lang, status.en, status.de) : ""}
        </output>
      </div>
    </form>
  );
}
