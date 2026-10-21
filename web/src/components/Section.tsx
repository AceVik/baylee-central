import type { ReactNode } from "react";

import { T } from "../i18n";

export function SkipLink() {
  return (
    <a className="skip" href="#main">
      <T id="app.skip" />
    </a>
  );
}

/** A section with its eyebrow and heading; `alt` gives it the other background. */
export function Section({
  id,
  eyebrow,
  title,
  alt,
  children,
}: {
  id?: string;
  eyebrow: ReactNode;
  title: ReactNode;
  alt?: boolean;
  children: ReactNode;
}) {
  const heading = id === undefined ? undefined : `${id}-title`;
  return (
    <section className={`section${alt === true ? " section-alt" : ""}`} id={id} aria-labelledby={heading}>
      <div className="section-inner">
        <p className="eyebrow">{eyebrow}</p>
        <h2 id={heading}>{title}</h2>
        {children}
      </div>
    </section>
  );
}
