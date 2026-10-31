// A modal on the browser's own `<dialog>`: mounted while open, focus held
// inside, Escape or a click on the backdrop closes it, and closing gives the
// focus back to its opener, including when React removes the dialog first;
// when the opener is gone too, to the page's main content, never to <body>.
// The browser may close a dialog by itself (a second Escape in Chromium
// skips `cancel`): that asks `onClose` too, and a dialog its owner keeps
// open (a secret shown once) opens again.

import { useEffect, useRef, type ReactNode } from "react";

export function Modal({
  className,
  label,
  labelledBy,
  onClose,
  onKey,
  children,
}: {
  className: string;
  label?: string;
  labelledBy?: string;
  onClose: () => void;
  /** Keys pressed inside, for a dialog that steps (the screenshots). */
  onKey?: (e: KeyboardEvent) => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const handlers = useRef({ onClose, onKey });
  useEffect(() => {
    handlers.current = { onClose, onKey };
  });
  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    if (!dialog.open) dialog.showModal();
    let backdropDown = false;
    const down = (e: PointerEvent) => { backdropDown = e.target === dialog; };
    const click = (e: MouseEvent) => {
      if (backdropDown && e.target === dialog) handlers.current.onClose();
      backdropDown = false;
    };
    const key = (e: KeyboardEvent) => handlers.current.onKey?.(e);
    let leaving = false;
    let frame = 0;
    const closed = () => {
      if (leaving) return;
      handlers.current.onClose();
      frame = requestAnimationFrame(() => {
        if (!leaving && dialog.isConnected && !dialog.open) dialog.showModal();
      });
    };
    dialog.addEventListener("pointerdown", down);
    dialog.addEventListener("click", click);
    dialog.addEventListener("keydown", key);
    dialog.addEventListener("close", closed);
    return () => {
      leaving = true;
      cancelAnimationFrame(frame);
      dialog.removeEventListener("pointerdown", down);
      dialog.removeEventListener("click", click);
      dialog.removeEventListener("keydown", key);
      dialog.removeEventListener("close", closed);
      if (dialog.open) dialog.close();
      const back = previous?.isConnected ? previous : document.getElementById("main");
      back?.focus({ preventScroll: true });
    };
  }, []);
  return (
    <dialog
      ref={ref}
      className={className}
      aria-label={label}
      aria-labelledby={labelledBy}
      onCancel={(e) => {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }}
    >
      {children}
    </dialog>
  );
}
