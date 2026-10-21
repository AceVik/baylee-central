// The contact form. It posts with fetch to the feedback service of Baylee's
// repository (`POST /site/contact`, docs/feedback.md §"The landing page's
// contact form"): the name and address only if given, the message, the
// page's language and an empty honeypot. Nothing is kept in the browser.

import { useEffect, useRef, useState, type FormEvent } from "react";

import { contactEndpoint } from "../config";
import { T, num, t, useLang, type Args, type Key } from "../i18n";
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
  const [status, setStatus] = useState<{ bad: boolean; key: Key; args?: Args } | null>(null);
  const [problem, setProblem] = useState<Problem>(null);
  const [done, setDone] = useState(false);
  const messageRef = useRef<HTMLTextAreaElement>(null);
  const emailRef = useRef<HTMLInputElement>(null);
  const timer = useRef(0);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  const say = (bad: boolean, key: Key, args?: Args) => setStatus(args === undefined ? { bad, key } : { bad, key, args });
  const unreachable = () => say(true, "form.unreachable");

  const submit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    // never `disabled` while sending, which would drop the keyboard's focus to the page; a second press is ignored
    if (state === "sending") return;
    const text = message.trim();
    const mail = email.trim();
    if (!text) {
      setProblem("message");
      say(true, "form.empty");
      messageRef.current?.focus();
      return;
    }
    if (mail && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(mail)) {
      setProblem("email");
      say(true, "form.badEmail");
      emailRef.current?.focus();
      return;
    }
    setProblem(null);
    setState("sending");
    say(false, "form.sending");
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
          say(false, "form.sent");
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
        if (r.status === 429) say(true, "form.tooMany");
        else if (r.status === 400 || r.status === 413) say(true, "form.refused", { max: MAX });
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
          <T id="form.name" />{" "}
          <span className="opt">
            <T id="form.optional" />
          </span>
        </label>
        <input id="c-name" type="text" maxLength={100} autoComplete="name" value={name} onChange={(e) => setName(e.target.value)} />
      </div>
      <div className="field">
        <label htmlFor="c-email">
          <T id="form.email" />{" "}
          <span className="opt">
            <T id="form.emailOptional" />
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
          <T id="form.message" />
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
          {status ? t(lang, status.key, status.args) : ""}
        </output>
      </div>
    </form>
  );
}
