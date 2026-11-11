// Baylee's album (the start page's #album and #photo-viewer): the server
// writes the whole book; this turns it into one chapter and one photograph
// at a time, turns the page between chapters, and opens a photograph full
// size to zoom into and move about. Plain DOM, no markup strings: nothing
// here needs a Trusted Types policy. Without it the book still reads,
// every chapter after the other.

const album = document.getElementById("album");
const viewer = document.getElementById("photo-viewer");

if (album instanceof HTMLDialogElement && viewer instanceof HTMLDialogElement) {
  const still = () => matchMedia("(prefers-reduced-motion: reduce)").matches;
  const tabs = [...album.querySelectorAll('[role="tab"]')];
  const chapters = tabs.map((tab) => document.getElementById(tab.getAttribute("aria-controls") ?? ""));
  const page = album.querySelector(".story-page");
  const leaves = album.querySelector(".story-leaves");
  const photos = [...album.querySelectorAll(".photo-preview img")];
  const caption = album.querySelector("[data-caption]");
  const counts = document.querySelectorAll("[data-photo-count]");
  const chapterCount = album.querySelector("[data-chapter-count]");
  const previousChapter = album.querySelector('[data-chapter-step="-1"]');
  const nextChapter = album.querySelector('[data-chapter-step="1"]');
  let chapter = 0;
  let photo = 0;

  // ---- chapters ------------------------------------------------------------

  const showChapter = (to) => {
    chapters.forEach((section, at) => {
      if (section) section.hidden = at !== to;
    });
    tabs.forEach((tab, at) => {
      tab.setAttribute("aria-selected", String(at === to));
      tab.tabIndex = at === to ? 0 : -1;
    });
    if (chapterCount) chapterCount.textContent = `${to + 1} / ${chapters.length}`;
    if (previousChapter) previousChapter.disabled = to === 0;
    if (nextChapter) nextChapter.disabled = to === chapters.length - 1;
  };

  // A copy of the chapter being left folds away over the new one.
  const turnLeaf = (from, forward) => {
    const leaving = chapters[from];
    if (!leaving || !leaves || !page || still()) return;
    const leaf = document.createElement("div");
    leaf.className = `story-turning-leaf ${forward ? "forward" : "backward"}`;
    leaf.setAttribute("aria-hidden", "true");
    const ink = leaving.cloneNode(true);
    ink.removeAttribute("id");
    ink.removeAttribute("role");
    ink.removeAttribute("aria-labelledby");
    ink.hidden = false;
    ink.style.transform = `translateY(${-page.scrollTop}px)`;
    leaf.append(ink);
    leaves.append(leaf);
    leaf.addEventListener("animationend", () => leaf.remove(), { once: true });
  };

  const turn = (to, focusTab = false) => {
    const next = Math.max(0, Math.min(chapters.length - 1, to));
    if (next === chapter) return;
    turnLeaf(chapter, next > chapter);
    chapter = next;
    showChapter(chapter);
    page?.scrollTo({ top: 0, behavior: "instant" });
    if (focusTab) tabs[chapter]?.focus();
  };

  tabs.forEach((tab, at) => tab.addEventListener("click", () => turn(at)));
  previousChapter?.addEventListener("click", () => turn(chapter - 1));
  nextChapter?.addEventListener("click", () => turn(chapter + 1));

  // ---- photographs ------------------------------------------------------

  const showPhoto = (to, forward) => {
    photo = (to + photos.length) % photos.length;
    photos.forEach((image, at) => {
      image.hidden = at !== photo;
      if (at === photo) {
        image.classList.toggle("back", !forward);
        // the slide runs again for each photograph
        image.style.animation = "none";
        void image.offsetWidth;
        image.style.animation = "";
      }
    });
    const words = photos[photo]?.alt ?? "";
    if (caption) caption.textContent = words;
    for (const count of counts) count.textContent = `${photo + 1} / ${photos.length}`;
    if (viewer.open) showFull();
  };
  const step = (by) => showPhoto(photo + by, by > 0);
  for (const button of document.querySelectorAll("[data-photo-step]")) {
    button.addEventListener("click", () => step(Number(button.getAttribute("data-photo-step"))));
  }

  // ---- keys: arrows turn photographs, or chapters on the tabs or with Alt --

  album.addEventListener("keydown", (event) => {
    if (event.ctrlKey || event.metaKey || viewer.open) return;
    const onTab = event.target instanceof Element && event.target.getAttribute("role") === "tab";
    if (onTab && (event.key === "Home" || event.key === "End")) {
      event.preventDefault();
      turn(event.key === "Home" ? 0 : chapters.length - 1, true);
    } else if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      const by = event.key === "ArrowRight" ? 1 : -1;
      if (onTab || event.altKey) turn(chapter + by, onTab);
      else step(by);
    }
  });

  // ---- the full photograph ------------------------------------------------

  const viewport = viewer.querySelector("[data-viewport]");
  const full = viewer.querySelector("[data-viewer-image]");
  const fullCaption = viewer.querySelector("[data-viewer-caption]");
  const level = viewer.querySelector("[data-zoom-level]");
  const zoomOut = viewer.querySelector('[data-zoom="-1"]');
  const zoomIn = viewer.querySelector('[data-zoom="1"]');
  let view = { zoom: 1, x: 0, y: 0 };
  let drag = null;

  // the photograph may move only as far as it reaches past the frame
  const clamp = (next) => {
    if (!(viewport instanceof HTMLElement) || !(full instanceof HTMLImageElement) || !full.naturalWidth) {
      return { zoom: next.zoom, x: 0, y: 0 };
    }
    const fit = Math.min(viewport.clientWidth / full.naturalWidth, viewport.clientHeight / full.naturalHeight);
    const x = Math.max(0, (full.naturalWidth * fit * next.zoom - viewport.clientWidth) / 2);
    const y = Math.max(0, (full.naturalHeight * fit * next.zoom - viewport.clientHeight) / 2);
    return { zoom: next.zoom, x: Math.max(-x, Math.min(x, next.x)), y: Math.max(-y, Math.min(y, next.y)) };
  };
  const draw = (next) => {
    view = clamp(next);
    if (full instanceof HTMLElement) full.style.transform = `translate(${view.x}px, ${view.y}px) scale(${view.zoom})`;
    if (viewport instanceof HTMLElement) viewport.toggleAttribute("data-zoomed", view.zoom > 1);
    if (level) level.textContent = `${Math.round(view.zoom * 100)} %`;
    if (zoomOut) zoomOut.disabled = view.zoom <= 1;
    if (zoomIn) zoomIn.disabled = view.zoom >= 4;
  };
  const zoom = (by) => draw({ ...view, zoom: Math.max(1, Math.min(4, view.zoom + by)) });
  const fit = () => draw({ zoom: 1, x: 0, y: 0 });
  const showFull = () => {
    const source = photos[photo];
    if (!(full instanceof HTMLImageElement) || !source) return;
    full.src = source.dataset.full ?? source.src;
    full.alt = source.alt;
    if (fullCaption) fullCaption.textContent = source.alt;
    fit();
  };

  viewer.addEventListener("toggle", (event) => {
    if (event.newState === "open") showFull();
  });
  // where a dialog fires no toggle event, the frame that opens it says so
  album.querySelector(".album-photo-open")?.addEventListener("click", () => requestAnimationFrame(showFull));
  full?.addEventListener("load", () => draw(view));
  for (const button of viewer.querySelectorAll("[data-zoom]")) {
    button.addEventListener("click", () => {
      const by = Number(button.getAttribute("data-zoom"));
      if (by === 0) fit();
      else zoom(by * 0.5);
    });
  }
  viewport?.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      zoom(event.deltaY < 0 ? 0.5 : -0.5);
    },
    { passive: false },
  );
  viewport?.addEventListener("pointerdown", (event) => {
    if (event.button !== 0 || view.zoom <= 1 || drag) return;
    event.preventDefault();
    viewport.setPointerCapture(event.pointerId);
    viewport.toggleAttribute("data-dragging", true);
    drag = { id: event.pointerId, x: event.clientX, y: event.clientY, origin: view };
  });
  viewport?.addEventListener("pointermove", (event) => {
    if (!drag || drag.id !== event.pointerId) return;
    draw({ ...drag.origin, x: drag.origin.x + event.clientX - drag.x, y: drag.origin.y + event.clientY - drag.y });
  });
  const release = () => {
    drag = null;
    viewport?.removeAttribute("data-dragging");
  };
  viewport?.addEventListener("pointerup", release);
  viewport?.addEventListener("pointercancel", release);
  viewport?.addEventListener("lostpointercapture", release);
  if (viewport) new ResizeObserver(() => draw(view)).observe(viewport);

  viewer.addEventListener("keydown", (event) => {
    if (event.ctrlKey || event.metaKey) return;
    const keys = { ArrowRight: () => step(1), ArrowLeft: () => step(-1), "+": () => zoom(0.5), "=": () => zoom(0.5), "-": () => zoom(-0.5), 0: fit };
    const act = keys[event.key];
    if (!act) return;
    event.preventDefault();
    event.stopPropagation();
    act();
  });

  // ---- ready: one chapter, one photograph -----------------------------------

  showChapter(0);
  showPhoto(0, true);
  album.dataset.ready = "";
}
