// The send button says where the message is: idle, sending, sent, error. It
// is never `disabled` while sending, which would drop the keyboard's focus
// to the page; the form ignores a second press instead.

import { Icon } from "../components/icons";
import { T } from "../i18n";

export type SendState = "idle" | "sending" | "sent" | "error";

export function SendButton({ state }: { state: SendState }) {
  return (
    <button type="submit" className="button button-send" data-state={state} aria-disabled={state === "sending" ? true : undefined}>
      {state === "sending" ? (
        <>
          <svg className="button-icon i-spin" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <circle cx="12" cy="12" r="8.5" />
          </svg>
          <T id="form.sending" />
        </>
      ) : state === "sent" ? (
        <>
          <Icon name="check" className="button-icon" />
          <T id="send.sent" />
        </>
      ) : state === "error" ? (
        <>
          <Icon name="retry" className="button-icon" />
          <T id="send.retry" />
        </>
      ) : (
        <>
          <Icon name="send" className="button-icon" />
          <T id="send.send" />
        </>
      )}
    </button>
  );
}
