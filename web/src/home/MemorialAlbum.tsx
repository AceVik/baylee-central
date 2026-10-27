import { useEffect, useRef, useState } from "react";

import { Modal } from "../components/Modal";
import { T, t, useLang, type Key } from "../i18n";
import { MemorialLight } from "./MemorialLight";
import { PhotoViewer, CAPTIONS } from "./PhotoViewer";
import "../styles/memorial-book.css";

const FOCAL = ["50% 57%", "50% 60%", "47% 53%", "48% 50%", "50% 56%", "45% 50%", "61% 54%", "43% 64%", "50% 58%", "50% 54%"];

export const CHAPTERS: readonly [Key, Key][] = Array.from({ length: 8 }, (_, i) =>
  [`baylee.chapter${i + 1}Title`, `baylee.chapter${i + 1}Body`] as [Key, Key]);


export function MemorialAlbum({ photo, chapter, onPhoto, onChapter, onClose }: {
  photo: number; chapter: number; onPhoto: (photo: number) => void;
  onChapter: (chapter: number) => void; onClose: () => void;
}) {
  const lang = useLang();
  const article = useRef<HTMLElement>(null);
  const [viewing, setViewing] = useState(false);
  const [leaf, setLeaf] = useState<{ chapter: number; scroll: number; direction: string; id: number } | null>(null);
  const serial = useRef(0);
  useEffect(() => {
    if (!leaf) return;
    const timeout = setTimeout(() => setLeaf(null), 760);
    return () => clearTimeout(timeout);
  }, [leaf]);
  const selected = CHAPTERS[chapter] ?? ["baylee.chapter1Title", "baylee.chapter1Body"] as const;
  const caption = CAPTIONS[photo] ?? "baylee.photo1";
  const turn = (to: number) => {
    const next = Math.max(0, Math.min(CHAPTERS.length - 1, to));
    if (next === chapter) return;
    if (!matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setLeaf({ chapter, scroll: article.current?.scrollTop ?? 0, direction: next > chapter ? "forward" : "backward", id: ++serial.current });
    }
    onChapter(next);
    article.current?.scrollTo({ top: 0, behavior: "instant" });
  };
  const step = (by: number) => onPhoto((photo + by + CAPTIONS.length) % CAPTIONS.length);
  return (
    <Modal className="baylee-album" labelledBy="baylee-title" onClose={onClose} onKey={(event) => {
      if (event.ctrlKey || event.metaKey) return;
      if ((event.key === "Home" || event.key === "End") && event.target instanceof HTMLElement && event.target.getAttribute("role") === "tab") {
        event.preventDefault();
        const next = event.key === "Home" ? 0 : CHAPTERS.length - 1;
        turn(next);
        document.getElementById(`baylee-chapter-${next}`)?.focus();
      }
      if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
        event.preventDefault();
        if (event.altKey || (event.target instanceof HTMLElement && event.target.getAttribute("role") === "tab")) {
          const next = Math.max(0, Math.min(CHAPTERS.length - 1, chapter + (event.key === "ArrowRight" ? 1 : -1)));
          turn(next);
          if (event.target instanceof HTMLElement && event.target.getAttribute("role") === "tab") document.getElementById(`baylee-chapter-${next}`)?.focus();
        }
        else step(event.key === "ArrowRight" ? 1 : -1);
      }
    }}>
      <header className="album-masthead">
        <MemorialLight />
        <img className="album-cloud" src="/img/baylee-cloud.webp" width={1200} height={800} alt="" aria-hidden="true" />
        <div className="album-heading">
          <p className="eyebrow"><T id="baylee.eyebrow" /></p>
          <h2 id="baylee-title">Baylee</h2>
          <p className="album-love"><T id="baylee.storyTitle" /></p>
          <p className="album-dates"><T id="baylee.together" /></p>
        </div>
        <button type="button" className="round-btn album-dismiss" onClick={onClose} aria-label={t(lang, "baylee.close")}><span aria-hidden="true">×</span></button>
      </header>
      <div className="album-layout">
        <figure className="album-photo">
          <button className="album-photo-open" type="button" onClick={() => setViewing(true)} aria-label={t(lang, "baylee.photoOpen")} aria-haspopup="dialog">
            <span className="photo-preview"><img key={photo} src={`/img/memorial/baylee-${photo + 1}.jpg`} alt={t(lang, caption)} width={1400} height={1400} style={{ objectPosition: FOCAL[photo] }} /></span>
            <span className="photo-open-hint">{t(lang, "baylee.photoOpen")} ↗</span>
          </button>
          <div className="album-photo-caption">
            <figcaption aria-live="polite"><T id={caption} /></figcaption>
            <div className="album-controls">
              <button type="button" className="round-btn" onClick={() => step(-1)} aria-label={t(lang, "baylee.previous")}>←</button>
              <span aria-hidden="true">{photo + 1} / {CAPTIONS.length}</span>
              <button type="button" className="round-btn" onClick={() => step(1)} aria-label={t(lang, "baylee.next")}>→</button>
            </div>
            <p className="album-photo-note"><T id="baylee.photos" /></p>
          </div>
        </figure>
        <div className="album-reading">
          <div className="chapter-nav" role="tablist" aria-label={t(lang, "baylee.chapters")}>
            {CHAPTERS.map(([title], i) => <button key={title} id={`baylee-chapter-${i}`} role="tab" type="button" aria-selected={chapter === i} tabIndex={chapter === i ? 0 : -1} aria-controls="baylee-chapter-panel" aria-current={chapter === i ? "step" : undefined}
              aria-label={`${i + 1} · ${t(lang, title)}`} title={t(lang, title)} onClick={() => turn(i)}>{String(i + 1).padStart(2, "0")}</button>)}
          </div>
          <div className="story-leaves">
          <article className="story-page" id="baylee-chapter-panel" role="tabpanel" ref={article} tabIndex={0} aria-labelledby={`baylee-chapter-${chapter}`}>
            <div key={chapter} className="story-ink">
              <p className="eyebrow"><T id="baylee.chapters" /> · {chapter + 1} / {CHAPTERS.length}</p>
              <h3 id="chapter-title">{t(lang, selected[0])}</h3>
              {t(lang, selected[1]).split("\n\n").map((paragraph, i) => <p key={i}>{paragraph}</p>)}
              <div className="story-flower" aria-hidden="true">❧</div>
            </div>
          </article>
          {leaf && <div key={leaf.id} className={`story-turning-leaf ${leaf.direction}`} aria-hidden="true" onAnimationEnd={() => setLeaf(current => current?.id === leaf.id ? null : current)}>
            <div className="story-ink" style={{ transform: `translateY(${-leaf.scroll}px)` }}>
              <p className="eyebrow"><T id="baylee.chapters" /> · {leaf.chapter + 1} / {CHAPTERS.length}</p>
              <h3>{t(lang, (CHAPTERS[leaf.chapter]?.[0] ?? "baylee.chapter1Title"))}</h3>
              {t(lang, (CHAPTERS[leaf.chapter]?.[1] ?? "baylee.chapter1Body")).split("\n\n").map((paragraph, i) => <p key={i}>{paragraph}</p>)}
              <div className="story-flower">❧</div>
            </div>
          </div>}
          </div>
          <footer className="chapter-footer">
            <button className="button button-ghost" type="button" disabled={chapter === 0} onClick={() => turn(chapter - 1)} aria-label={t(lang, "baylee.chapterPrevious")}>
              ← <span><T id="baylee.chapterPrevious" /></span>
            </button>
            <span className="chapter-count" aria-live="polite">{chapter + 1} / {CHAPTERS.length}</span>
            <button className="button button-ghost" type="button" disabled={chapter === CHAPTERS.length - 1} onClick={() => turn(chapter + 1)} aria-label={t(lang, "baylee.chapterNext")}>
              <span><T id="baylee.chapterNext" /></span> →
            </button>
          </footer>
        </div>
      </div>
      {viewing && <PhotoViewer photo={photo} onPhoto={onPhoto} onClose={() => setViewing(false)} />}
    </Modal>
  );
}
