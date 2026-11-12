// The start page's script. Baylee's album (#album and #photo-viewer): the
// server writes the whole book; this turns it into one chapter and one
// photograph at a time, turns the page between chapters, and opens a
// photograph full size to zoom into and move about. The screenshots: each
// is drawn, and starts moving, once it comes near, and opens large
// (#shot-viewer), where the arrow keys step. Plain DOM, no
// markup strings: nothing here needs a Trusted Types policy. Without it
// the book still reads, every chapter after the other.

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

  // The page that turns is a sheet of paper, not a board: a copy of the
  // chapter, cut into strips that are each hinged on the one before, so the
  // leaf bends as its edge lifts and lies flat again where it lands. Going
  // on, the chapter being left lifts away from the next; going back, the
  // chapter come back to is laid down over the one being left.
  const STRIPS = 10;
  const copyOf = (section, lift) => {
    const ink = section.cloneNode(true);
    ink.removeAttribute("id");
    ink.removeAttribute("role");
    ink.removeAttribute("aria-labelledby");
    ink.hidden = false;
    ink.style.transform = `translateY(${-lift}px)`;
    return ink;
  };
  const sheetOf = (section, lift, width, gutter) => {
    const sheet = document.createElement("div");
    sheet.className = "leaf-sheet";
    sheet.style.width = `${width}px`;
    // the page's own text ends where its scrollbar begins
    if (gutter > 0) sheet.style.borderInlineEnd = `${gutter}px solid transparent`;
    sheet.append(copyOf(section, lift));
    return sheet;
  };
  const turnLeaf = (from, to) => {
    const leaving = chapters[from];
    const entering = chapters[to];
    if (!leaving || !entering || !leaves || !page || still()) return;
    // a turn that had not ended makes way for this one
    for (const old of leaves.querySelectorAll("[data-turning]")) old.remove();
    const forward = to > from;
    const width = leaves.clientWidth;
    const gutter = page.offsetWidth - page.clientWidth;
    const scrolled = page.scrollTop;
    const parts = [];
    if (!forward) {
      // what the leaf comes down on: the chapter being left, as it stood
      const under = document.createElement("div");
      under.className = "story-under";
      under.append(sheetOf(leaving, scrolled, width, gutter));
      parts.push(under);
    }
    const shade = document.createElement("div");
    shade.className = "story-turn-shade";
    parts.push(shade);
    const leaf = document.createElement("div");
    leaf.className = `story-turning-leaf ${forward ? "forward" : "backward"}`;
    let hinge = leaf;
    let first = null;
    for (let at = 0; at < STRIPS; at += 1) {
      const strip = document.createElement("div");
      strip.className = "leaf-strip";
      if (at === 0) strip.style.width = `${width / STRIPS}px`;
      const face = document.createElement("div");
      face.className = "leaf-face";
      const sheet = sheetOf(forward ? leaving : entering, forward ? scrolled : 0, width, gutter);
      sheet.style.left = `${(-at * width) / STRIPS}px`;
      face.append(sheet);
      strip.append(face);
      hinge.append(strip);
      hinge = strip;
      first ??= strip;
    }
    parts.push(leaf);
    for (const part of parts) {
      part.dataset.turning = "";
      part.setAttribute("aria-hidden", "true");
    }
    leaves.append(...parts);
    // every strip's animation ends at once; the first one's is the leaf's
    first?.addEventListener("animationend", (event) => {
      if (event.target === first) for (const part of parts) part.remove();
    });
  };

  const turn = (to, focusTab = false) => {
    const next = Math.max(0, Math.min(chapters.length - 1, to));
    if (next === chapter) return;
    turnLeaf(chapter, next);
    chapter = next;
    showChapter(chapter);
    page?.scrollTo({ top: 0, behavior: "instant" });
    if (focusTab) tabs[chapter]?.focus();
  };

  tabs.forEach((tab, at) => tab.addEventListener("click", () => turn(at)));
  previousChapter?.addEventListener("click", () => turn(chapter - 1));
  nextChapter?.addEventListener("click", () => turn(chapter + 1));

  // ---- photographs ------------------------------------------------------

  // The photograph asked for is laid on the one before it, which goes once
  // the new one has settled.
  const showPhoto = (to, forward) => {
    const before = photos[photo];
    photo = (to + photos.length) % photos.length;
    const next = photos[photo];
    const lay = before !== next && before?.hidden === false && !still();
    for (const image of photos) {
      const under = lay && image === before;
      image.classList.toggle("under", under);
      image.hidden = image !== next && !under;
    }
    if (next) {
      next.classList.toggle("back", !forward);
      // the movement runs again for each photograph
      next.style.animation = "none";
      void next.offsetWidth;
      next.style.animation = "";
    }
    const words = next?.alt ?? "";
    if (caption) caption.textContent = words;
    for (const count of counts) count.textContent = `${photo + 1} / ${photos.length}`;
    if (viewer.open) showFull();
  };
  for (const image of photos) {
    image.addEventListener("animationend", () => {
      for (const other of photos) {
        if (!other.classList.contains("under")) continue;
        other.hidden = true;
        other.classList.remove("under");
      }
    });
  }
  const step = (by) => showPhoto(photo + by, by > 0);
  for (const button of document.querySelectorAll("[data-photo-step]")) {
    button.addEventListener("click", () => step(Number(button.getAttribute("data-photo-step"))));
  }

  // ---- a finger: a stroke sideways turns the page, or the photograph --------

  // The page scrolls up and down under a finger as ever (touch-action:
  // pan-y); a stroke that is mostly sideways is the turn. A mouse selects
  // text with the same movement, so this is for touch and pen only.
  const stroke = (area, act) => {
    if (!area) return;
    let from = null;
    area.addEventListener("pointerdown", (event) => {
      from = event.pointerType === "mouse" ? null : { id: event.pointerId, x: event.clientX, y: event.clientY };
    });
    area.addEventListener("pointerup", (event) => {
      if (from?.id !== event.pointerId) return;
      const across = event.clientX - from.x;
      const down = event.clientY - from.y;
      from = null;
      if (Math.abs(across) >= 48 && Math.abs(across) > 2 * Math.abs(down)) act(across < 0 ? 1 : -1);
    });
    area.addEventListener("pointercancel", () => {
      from = null;
    });
  };
  stroke(leaves, (by) => turn(chapter + by));
  stroke(album.querySelector(".photo-preview"), (by) => step(by));

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

  // ---- the book opens and shuts -------------------------------------------------

  // The book is laid before the reader shut, and its cover swings up on the
  // spine towards the reader and over to the left while the reader leans
  // in. The light comes from over the reader's left shoulder: the cloth
  // catches it as it lifts and darkens as it turns away, the endpaper is
  // dark as it comes round and brightens as it comes down, and the cover's
  // shade on the first page shrinks towards the spine and is gone. The
  // first page lifts a little with the cover and settles. Shutting runs the
  // other way, and the book is taken away. These are Web Animations so that
  // light and shade follow the cover's angle under one easing over the
  // whole swing (a CSS animation eases each keyframe on its own); only
  // transform and opacity change. With less motion asked for the book is
  // simply open or shut, and without the script it opens as a dialog.
  const openers = [...document.querySelectorAll('button[commandfor="album"]')];
  const logo = document.querySelector(".hero-logo");
  const piece = (name, parent, tag = "div") => {
    const node = document.createElement(tag);
    node.className = name;
    parent.append(node);
    return node;
  };
  const cover = piece("album-cover", album);
  const front = piece("album-cover-face album-cover-front", cover);
  const mark = piece("album-cover-mark", front, "img");
  mark.alt = "";
  piece("album-cover-line", front, "p").textContent = album.querySelector(".album-love")?.textContent ?? "";
  const glint = piece("album-cover-glint", front);
  const frontDim = piece("album-cover-dim", front);
  const back = piece("album-cover-face album-cover-back", cover);
  const backDim = piece("album-cover-dim", back);
  const shade = piece("album-cover-shade", album);
  for (const part of [cover, shade]) part.setAttribute("aria-hidden", "true");
  const pages = [...album.querySelectorAll(":scope > .album-masthead, :scope > .album-layout")];

  // the cover's angle, 0 shut to 180 lying open, in steps the light follows
  const ANGLES = Array.from({ length: 13 }, (_, at) => at * 15);
  const rad = (angle) => (angle * Math.PI) / 180;
  // how squarely the cloth faces the light (the endpaper faces the other way)
  const facing = (angle) => 0.45 * Math.sin(rad(angle)) + 0.89 * Math.cos(rad(angle));
  const LIT = 0.89 ** 2;
  const round = (value) => Number(value.toFixed(3));
  // Opening, the cover is lifted gently, swings over fast and comes down
  // softly; shutting, it leaves the table at once (off the book, where
  // little of it is seen) and is laid down on the pages softly.
  const SWING = "cubic-bezier(0.42, 0.05, 0.2, 1)";
  const SWING_BACK = "cubic-bezier(0.36, 0.1, 0.24, 1)";
  const swing = [
    [cover, (angle) => ({ transform: `rotateY(${-angle}deg)` })],
    [frontDim, (angle) => ({ opacity: angle > 90 ? 0.55 : round(0.7 * Math.max(0, 1 - Math.max(0, facing(angle)) ** 2 / LIT)) })],
    [glint, (angle) => ({ opacity: angle < 60 ? round(Math.min(1, Math.max(0, facing(angle) ** 2 - LIT) / 0.2)) : 0, transform: `translateX(${Math.min(angle, 60) - 30}%)` })],
    [back, (angle) => ({ opacity: angle < 160 ? 1 : round((180 - angle) / 20) })],
    [backDim, (angle) => ({ opacity: round(0.65 * (1 - Math.min(1, Math.max(0, -facing(angle)) / 0.89))) })],
    [
      shade,
      (angle) => {
        // where the shadow of the cover's edge falls on the page
        const reach = Math.max(0, Math.min(1, Math.cos(rad(angle)) + 0.506 * Math.sin(rad(angle))));
        return { opacity: reach > 0 ? round(0.85 * Math.min(1, reach * 1.6) * (1 - angle / 200)) : 0, transform: `scaleX(${Math.max(0.001, round(reach))})` };
      },
    ],
  ];
  // the book is set down before the reader (it is there at once, it does
  // not fade in under its cover) and the reader leans in as it opens; shut,
  // the reader sits back and it is taken away
  const BOOK_OPEN = [
    { offset: 0, opacity: 0, transform: "translate3d(0, 18px, 0) scale(0.9)", easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
    { offset: 0.16, opacity: 1 },
    { offset: 0.34, transform: "translate3d(0, 0, 0) scale(0.94)", easing: "ease-in-out" },
    { offset: 1, opacity: 1, transform: "translate3d(0, 0, 0) scale(1)" },
  ];
  const BOOK_SHUT = [
    { offset: 0, opacity: 1, transform: "translate3d(0, 0, 0) scale(1)", easing: "ease-in-out" },
    { offset: 0.76, transform: "translate3d(0, 0, 0) scale(0.94)", easing: "cubic-bezier(0.4, 0, 1, 1)" },
    { offset: 0.8, opacity: 1, easing: "ease-in" },
    { offset: 1, opacity: 0, transform: "translate3d(0, 14px, 0) scale(0.88)" },
  ];
  // lifted with the cover, falling back, a last small bounce
  const SETTLE = [
    { offset: 0, transform: "rotateY(0deg)", easing: "cubic-bezier(0.3, 0, 0.6, 1)" },
    { offset: 0.38, transform: "rotateY(-2.2deg)", easing: "cubic-bezier(0.5, 0, 0.9, 0.6)" },
    { offset: 0.66, transform: "rotateY(0deg)", easing: "cubic-bezier(0.1, 0.4, 0.5, 1)" },
    { offset: 0.82, transform: "rotateY(-0.45deg)", easing: "cubic-bezier(0.5, 0, 0.9, 0.6)" },
    { offset: 1, transform: "rotateY(0deg)" },
  ];

  let folding = null;
  const unfold = () => {
    const runs = folding?.runs ?? [];
    folding = null;
    for (const run of runs) run.cancel();
    delete album.dataset.folding;
  };
  const ended = (runs) => Promise.all(runs.map((run) => run.finished)).then(
    () => true,
    () => false,
  );
  const fold = (opening) => {
    album.dataset.folding = opening ? "open" : "shut";
    const timing = { duration: opening ? 900 : 650, delay: opening ? 160 : 0, easing: opening ? SWING : SWING_BACK, fill: "both" };
    const frames = (pose) => {
      const all = ANGLES.map((angle) => ({ offset: angle / 180, ...pose(angle) }));
      return opening ? all : all.reverse().map((frame) => ({ ...frame, offset: round(1 - frame.offset) }));
    };
    const runs = [
      album.animate(opening ? BOOK_OPEN : BOOK_SHUT, { duration: opening ? 1060 : 860, fill: "both" }),
      ...swing.map(([node, pose]) => node.animate(frames(pose), timing)),
      ...(opening ? pages.map((page) => page.animate(SETTLE, { duration: 560, delay: 560, fill: "both" })) : []),
    ];
    folding = { opening, runs };
    return ended(runs);
  };
  const cloth = () => {
    if (!(logo instanceof HTMLImageElement)) return;
    const picture = logo.currentSrc || logo.src;
    if (picture && mark.src !== picture) mark.src = picture;
  };
  const opening = () => {
    if (album.open || folding || still()) return;
    cloth();
    const run = fold(true);
    const mine = folding;
    run.then((done) => {
      // a book shut while it opened is closed by `shut`, not laid open here
      if (done && folding === mine && mine.opening) unfold();
    });
  };
  const shut = () => {
    if (!album.open) return;
    if (still()) {
      album.close();
      return;
    }
    if (folding && !folding.opening) return;
    let run;
    if (folding) {
      // still opening: the same movement goes back from where it is
      folding.opening = false;
      album.dataset.folding = "shut";
      for (const one of folding.runs) one.reverse();
      run = ended(folding.runs);
    } else {
      run = fold(false);
    }
    run.then((done) => {
      if (done && album.open) album.close();
    });
  };
  // A press on the logo (by pointer or key) comes before the dialog opens,
  // so the first frame is already the shut book; `beforetoggle` catches any
  // other way it is opened.
  for (const opener of openers) {
    opener.addEventListener("click", opening);
    // the book's first photograph and the cover's picture are made ready
    // once a reader comes near the logo, not while they wait for the book
    const ready = () => {
      photos[0]?.setAttribute("loading", "eager");
      cloth();
      if (mark.src) mark.decode().catch(() => {});
    };
    for (const kind of ["pointerenter", "focus", "touchstart"]) opener.addEventListener(kind, ready, { once: true, passive: true });
  }
  album.addEventListener("beforetoggle", (event) => {
    if (event.newState === "open") opening();
  });
  album.addEventListener("close", unfold);
  // the close button and Escape shut the book; an Escape that may not be
  // held (pressed again at once) shuts it at once
  album.addEventListener("cancel", (event) => {
    if (!event.cancelable || still()) return;
    event.preventDefault();
    shut();
  });
  album.querySelector(".album-masthead form[method='dialog']")?.addEventListener("submit", (event) => {
    if (still()) return;
    event.preventDefault();
    shut();
  });

  // ---- ready: one chapter, one photograph -----------------------------------

  showChapter(0);
  showPhoto(0, true);
  album.dataset.ready = "";
}

// ---- the screenshots: drawn as they come near, and each one large -----------

// A recording (an animated AVIF) for those who did not ask for less motion,
// put before a picture's still.
const MOVING = "(prefers-reduced-motion: no-preference)";
const animate = (picture, srcset, sizes) => {
  let source = picture.querySelector("source[data-moving-source]");
  if (!source) {
    source = document.createElement("source");
    source.dataset.movingSource = "";
    source.media = MOVING;
    source.type = "image/avif";
    picture.prepend(source);
  }
  if (sizes) source.sizes = sizes;
  source.srcset = srcset;
};

// A card's picture, made once the card is within a little of the window
// (the server sent it only for browsers without script).
const draw = (button) => {
  if (button.querySelector("picture")) return;
  const picture = document.createElement("picture");
  const image = document.createElement("img");
  image.width = 1200;
  image.height = 703;
  image.alt = button.dataset.alt ?? "";
  image.decoding = "async";
  image.sizes = button.dataset.sizes ?? "";
  image.srcset = button.dataset.stills ?? "";
  image.src = button.dataset.still ?? "";
  picture.append(image);
  animate(picture, button.dataset.movings ?? "", button.dataset.sizes ?? "");
  button.prepend(picture);
};

const cards = [...document.querySelectorAll("button.shot-open[data-stills]")];
if (typeof IntersectionObserver === "function") {
  const near = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        near.unobserve(entry.target);
        draw(entry.target);
      }
    },
    { rootMargin: "300px 0px" },
  );
  for (const card of cards) near.observe(card);
} else {
  for (const card of cards) draw(card);
}

const shotViewer = document.getElementById("shot-viewer");

if (shotViewer instanceof HTMLDialogElement) {
  const shots = [...document.querySelectorAll(".shot-open")];
  const picture = shotViewer.querySelector("[data-shot-picture]");
  const image = shotViewer.querySelector("[data-shot-image]");
  const words = shotViewer.querySelector("[data-shot-caption]");
  let at = 0;
  const show = (to) => {
    at = (to + shots.length) % shots.length;
    const shot = shots[at];
    if (!shot) return;
    if (picture) animate(picture, shot.dataset.moving ?? "", "");
    if (image instanceof HTMLImageElement) {
      image.src = shot.dataset.still ?? "";
      image.alt = shot.dataset.alt ?? "";
    }
    const caption = shot.closest("figure")?.querySelector("figcaption");
    if (words && caption) words.replaceChildren(...[...caption.childNodes].map((node) => node.cloneNode(true)));
  };
  shots.forEach((shot, index) => shot.addEventListener("click", () => show(index)));
  for (const button of shotViewer.querySelectorAll("[data-shot-step]")) {
    button.addEventListener("click", () => show(at + Number(button.getAttribute("data-shot-step"))));
  }
  shotViewer.addEventListener("keydown", (event) => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
    event.preventDefault();
    show(at + (event.key === "ArrowRight" ? 1 : -1));
  });
}

// ---- the card numbers count up as they come into view -----------------------
// Once each, in the page's own number format, and only where motion is
// welcome: the number the server wrote is already the right one, so without
// the script, or with reduced motion, nothing changes.
const counters = [...document.querySelectorAll("dt[data-count]")];
if (counters.length > 0 && typeof IntersectionObserver === "function" && matchMedia(MOVING).matches) {
  const write = new Intl.NumberFormat(document.documentElement.lang || "en");
  const run = (node) => {
    const end = Number(node.dataset.count);
    const final = node.textContent;
    if (!Number.isFinite(end) || end <= 0) {
      return;
    }
    const began = performance.now();
    const span = 1100;
    const step = (now) => {
      const part = Math.min(1, (now - began) / span);
      // fast at first, settling at the end
      const eased = 1 - (1 - part) ** 3;
      node.textContent = part < 1 ? write.format(Math.round(end * eased)) : final;
      if (part < 1) {
        requestAnimationFrame(step);
      }
    };
    requestAnimationFrame(step);
  };
  const seen = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          seen.unobserve(entry.target);
          run(entry.target);
        }
      }
    },
    { threshold: 0.6 },
  );
  for (const node of counters) {
    seen.observe(node);
  }
}
