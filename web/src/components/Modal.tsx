// A modal on the browser's own `<dialog>`: mounted while open, focus held
// inside, Escape or a click on the backdrop closes it, and closing gives the
// focus back to what had it before (the browser does that on `close()`).

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
    if (!dialog.open) dialog.showModal();
    const click = (e: MouseEvent) => {
      if (e.target === dialog) handlers.current.onClose();
    };
    const key = (e: KeyboardEvent) => handlers.current.onKey?.(e);
    dialog.addEventListener("click", click);
    dialog.addEventListener("keydown", key);
    return () => {
      dialog.removeEventListener("click", click);
      dialog.removeEventListener("keydown", key);
      if (dialog.open) dialog.close();
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
        onClose();
      }}
    >
      {children}
    </dialog>
  );
}
