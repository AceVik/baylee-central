import { useState } from "react";

import { Modal } from "../components/Modal";
import { T, t, useLang, type Key } from "../i18n";
import "../styles/remembrance.css";

const CAPTIONS: Key[] = ["baylee.photo1", "baylee.photo2", "baylee.photo3", "baylee.photo4", "baylee.photo5"];

export function Remembrance() {
  const lang = useLang();
  const [open, setOpen] = useState(false);
  const [photo, setPhoto] = useState(0);
  const step = (by: number) => setPhoto((current) => (current + by + CAPTIONS.length) % CAPTIONS.length);
  return (
    <div className="remembrance">
      <button className="baylee-cloud" type="button" onClick={() => setOpen(true)} aria-label={t(lang, "baylee.open")} aria-haspopup="dialog">
        <span className="cloud-halo" aria-hidden="true" />
        <img src="/img/baylee-cloud.webp" width={1200} height={800} alt="" decoding="async" fetchPriority="high" />
        <span className="cloud-caption">
          <span className="cloud-dedication"><T id="baylee.dedication" /></span>
          <span className="cloud-invitation"><T id="baylee.invitation" /><span aria-hidden="true"> ↗</span></span>
        </span>
      </button>
      {open && (
        <Modal className="baylee-album" labelledBy="baylee-title" onClose={() => setOpen(false)} onKey={(event) => {
          if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
            event.preventDefault();
            step(event.key === "ArrowLeft" ? -1 : 1);
          }
        }}>
          <button type="button" className="round-btn album-dismiss" onClick={() => setOpen(false)} aria-label={t(lang, "baylee.close")}><span aria-hidden="true">×</span></button>
          <div className="album-layout">
            <figure className="album-photo">
              <img src={`/img/memorial/baylee-${photo + 1}.jpg`} alt={t(lang, CAPTIONS[photo] ?? "baylee.photo1")} width={1400} height={1400} />
              <figcaption aria-live="polite"><T id={CAPTIONS[photo] ?? "baylee.photo1"} /></figcaption>
              <div className="album-controls">
                <button type="button" className="round-btn" onClick={() => step(-1)} aria-label={t(lang, "baylee.previous")}>←</button>
                <span aria-hidden="true">{photo + 1} / {CAPTIONS.length}</span>
                <button type="button" className="round-btn" onClick={() => step(1)} aria-label={t(lang, "baylee.next")}>→</button>
              </div>
            </figure>
            <div className="album-story">
              <p className="eyebrow"><T id="baylee.eyebrow" /></p>
              <h2 id="baylee-title">Baylee</h2>
              <p className="album-love"><T id="baylee.dedication" /></p>
              <p><T id="baylee.story" /></p>
              <p><T id="baylee.memory" /></p>
              <p className="small"><T id="baylee.date" /></p>
              <button type="button" className="button button-ghost" onClick={() => setOpen(false)}><T id="baylee.close" /></button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
