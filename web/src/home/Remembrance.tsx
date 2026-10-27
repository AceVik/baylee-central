import { useState } from "react";

import { MemorialAlbum } from "./MemorialAlbum";
import { T, t, useLang } from "../i18n";
import "../styles/remembrance.css";


export function Remembrance() {
  const lang = useLang();
  const [open, setOpen] = useState(false);
  const [photo, setPhoto] = useState(0);
  const [chapter, setChapter] = useState(0);
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
      {open && <MemorialAlbum photo={photo} chapter={chapter} onPhoto={setPhoto} onChapter={setChapter} onClose={() => setOpen(false)} />}
    </div>
  );
}
