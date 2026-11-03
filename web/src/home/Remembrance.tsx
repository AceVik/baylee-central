import { lazy, Suspense, useState } from "react";

import { Logo } from "../components/Logo";
import { t, useLang } from "../i18n";
import "../styles/remembrance.css";

// the album, its photographs' viewer and its light load when it is first opened
const MemorialAlbum = lazy(() => import("./MemorialAlbum").then((m) => ({ default: m.MemorialAlbum })));

export function Remembrance() {
  const lang = useLang();
  const [open, setOpen] = useState(false);
  const [photo, setPhoto] = useState(0);
  const [chapter, setChapter] = useState(0);
  return (
    <div className="remembrance">
      <button
        className="baylee-cloud hero-logo-btn"
        type="button"
        onClick={() => setOpen(true)}
        aria-label={t(lang, "baylee.open")}
        aria-haspopup="dialog"
      >
        <span className="cloud-halo" aria-hidden="true" />
        <Logo className="hero-logo" sizes="(max-width: 480px) 15rem, 23rem" alt="" priority />
      </button>
      {open && (
        <Suspense fallback={null}>
          <MemorialAlbum photo={photo} chapter={chapter} onPhoto={setPhoto} onChapter={setChapter} onClose={() => setOpen(false)} />
        </Suspense>
      )}
    </div>
  );
}
