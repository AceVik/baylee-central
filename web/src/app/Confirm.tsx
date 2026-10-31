// A second step for a change that cannot be taken back (docs/website-design.md
// §"Components", dialogs): it names what it acts on and asks in the words of
// its button, never "OK".

import { useId, type ReactNode } from "react";

import { BusyButton } from "../account/Busy";
import { Modal } from "../components/Modal";
import { T } from "../i18n";
import type { Key } from "../i18n";

export function Confirm({
  title,
  children,
  action,
  danger = true,
  busy = false,
  onConfirm,
  onCancel,
}: {
  title: ReactNode;
  children: ReactNode;
  action: Key;
  danger?: boolean;
  busy?: boolean;
  onConfirm(): void;
  onCancel(): void;
}) {
  const heading = useId();
  return (
    <Modal className="app-dialog" labelledBy={heading} onClose={onCancel}>
      <h2 id={heading} className="app-dialog-title">
        {title}
      </h2>
      <div className="app-dialog-body">{children}</div>
      <div className="app-dialog-actions">
        <button type="button" className="button button-ghost" onClick={onCancel}>
          <T id="app.cancel" />
        </button>
        <BusyButton busy={busy} className={danger ? "button button-danger solid" : "button"} onClick={onConfirm}>
          <T id={action} />
        </BusyButton>
      </div>
    </Modal>
  );
}
