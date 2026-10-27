import { useCallback, useEffect, useRef, useState } from "react";
import { Modal } from "../components/Modal";
import { t, useLang, type Key } from "../i18n";

export const CAPTIONS: Key[] = ["baylee.photo1", "baylee.photo2", "baylee.photo3", "baylee.photo4", "baylee.photo5", "baylee.photo6", "baylee.photo7", "baylee.photo8", "baylee.photo9", "baylee.photo10"];
type View = { zoom: number; x: number; y: number };
const FIT: View = { zoom: 1, x: 0, y: 0 };

export function PhotoViewer({ photo, onPhoto, onClose }: { photo: number; onPhoto: (n: number) => void; onClose: () => void }) {
  const lang = useLang();
  const viewport = useRef<HTMLDivElement>(null);
  const image = useRef<HTMLImageElement>(null);
  const [view, setView] = useState<View>(FIT);
  const drag = useRef<{ id: number; x: number; y: number; origin: View } | null>(null);
  const clamp = useCallback((next: View): View => {
    const box = viewport.current, img = image.current;
    if (!box || !img?.naturalWidth) return { ...next, x: 0, y: 0 };
    const fit = Math.min(box.clientWidth / img.naturalWidth, box.clientHeight / img.naturalHeight);
    const x = Math.max(0, (img.naturalWidth * fit * next.zoom - box.clientWidth) / 2);
    const y = Math.max(0, (img.naturalHeight * fit * next.zoom - box.clientHeight) / 2);
    return { zoom: next.zoom, x: Math.max(-x, Math.min(x, next.x)), y: Math.max(-y, Math.min(y, next.y)) };
  }, []);
  const zoom = useCallback((by: number) => { drag.current = null; setView(current => clamp({ ...current, zoom: Math.max(1, Math.min(4, current.zoom + by)) })); }, [clamp]);
  const fit = () => { drag.current = null; setView(FIT); };
  const step = (by: number) => { drag.current = null; setView(FIT); onPhoto((photo + by + CAPTIONS.length) % CAPTIONS.length); };
  useEffect(() => {
    const box = viewport.current;
    if (!box) return;
    const wheel = (event: WheelEvent) => { event.preventDefault(); zoom(event.deltaY < 0 ? 0.5 : -0.5); };
    const resize = new ResizeObserver(() => { drag.current = null; setView(current => clamp(current)); });
    resize.observe(box);
    box.addEventListener("wheel", wheel, { passive: false });
    return () => { resize.disconnect(); box.removeEventListener("wheel", wheel); };
  }, [clamp, zoom]);
  return <Modal className="baylee-photo-viewer" label={t(lang, "baylee.photos")} onClose={onClose} onKey={event => {
    event.stopPropagation();
    if (event.ctrlKey || event.metaKey) return;
    if (["ArrowRight", "ArrowLeft", "+", "=", "-", "0"].includes(event.key)) event.preventDefault();
    if (event.key === "ArrowRight") step(1);
    if (event.key === "ArrowLeft") step(-1);
    if (event.key === "+" || event.key === "=") zoom(0.5);
    if (event.key === "-") zoom(-0.5);
    if (event.key === "0") fit();
  }}>
    <header className="photo-viewer-header">
      <p>{t(lang, CAPTIONS[photo] ?? "baylee.photo1")}</p>
      <button className="round-btn" type="button" onClick={onClose} aria-label={t(lang, "baylee.photoClose")}>×</button>
    </header>
    <div ref={viewport} className="photo-viewport" data-zoomed={view.zoom > 1} onPointerDown={event => {
      if (event.button !== 0 || view.zoom <= 1 || drag.current) return;
      event.preventDefault(); event.currentTarget.setPointerCapture(event.pointerId);
      drag.current = { id: event.pointerId, x: event.clientX, y: event.clientY, origin: view };
    }} onPointerMove={event => {
      const held = drag.current;
      if (!held || held.id !== event.pointerId) return;
      setView(clamp({ ...held.origin, x: held.origin.x + event.clientX - held.x, y: held.origin.y + event.clientY - held.y }));
    }} onPointerUp={() => { drag.current = null; }} onPointerCancel={() => { drag.current = null; }} onLostPointerCapture={() => { drag.current = null; }}>
      <img key={photo} ref={image} src={`/img/memorial/full/baylee-${photo + 1}.jpg`} alt={t(lang, CAPTIONS[photo] ?? "baylee.photo1")} draggable={false} onLoad={() => setView(current => clamp(current))} style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})` }} />
    </div>
    <footer className="photo-viewer-tools">
      <div className="photo-step-controls">
        <button type="button" className="round-btn" onClick={() => step(-1)} aria-label={t(lang, "baylee.previous")}>←</button>
        <span>{photo + 1} / {CAPTIONS.length}</span>
        <button type="button" className="round-btn" onClick={() => step(1)} aria-label={t(lang, "baylee.next")}>→</button>
      </div>
      <div className="photo-zoom-controls">
        <button type="button" className="round-btn" disabled={view.zoom === 1} onClick={() => zoom(-0.5)} aria-label={t(lang, "baylee.zoomOut")}>−</button>
        <output aria-live="polite">{Math.round(view.zoom * 100)} %</output>
        <button type="button" className="round-btn" disabled={view.zoom === 4} onClick={() => zoom(0.5)} aria-label={t(lang, "baylee.zoomIn")}>+</button>
        <button type="button" className="button button-ghost" onClick={fit}>{t(lang, "baylee.fit")}</button>
      </div>
    </footer>
  </Modal>;
}
