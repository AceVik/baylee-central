// The start page's script: Baylee's album (#album and #photo-viewer),
// screenshots, and counting stats.
interface PhotoPrint {
  readonly sheet: HTMLDivElement;
  readonly shadow: HTMLSpanElement;
  readonly image: HTMLImageElement;
  index: number;
  large: boolean;
}

interface TurnParts {
  readonly top: PhotoPrint;
  readonly base: PhotoPrint;
  readonly incoming: PhotoPrint;
  readonly forward: boolean;
  readonly travel: number;
}

interface SweptTurn {
  readonly parts: TurnParts;
  readonly laid: number;
  readonly duration: number;
}

type PrintPartKey = "top" | "shadow" | "base";

interface TurnPose {
  readonly top: Keyframe;
  readonly shadow: Keyframe;
  readonly base: Keyframe;
}

interface BookFoldState {
  opening: boolean;
  readonly runs: Animation[];
}

const ANGLES = Array.from({ length: 13 }, (_, at) => at * 15);
const LIT = 0.89 ** 2;
const SWING = "cubic-bezier(0.42, 0.05, 0.2, 1)";
const SWING_BACK = "cubic-bezier(0.36, 0.1, 0.24, 1)";
const BOOK_OPEN: Keyframe[] = [
  { offset: 0, opacity: 0, transform: "translate3d(0, 18px, 0) scale(0.9)", easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
  { offset: 0.16, opacity: 1 },
  { offset: 0.34, transform: "translate3d(0, 0, 0) scale(0.94)", easing: "ease-in-out" },
  { offset: 1, opacity: 1, transform: "translate3d(0, 0, 0) scale(1)" },
];
const BOOK_SHUT: Keyframe[] = [
  { offset: 0, opacity: 1, transform: "translate3d(0, 0, 0) scale(1)", easing: "ease-in-out" },
  { offset: 0.76, transform: "translate3d(0, 0, 0) scale(0.94)", easing: "cubic-bezier(0.4, 0, 1, 1)" },
  { offset: 0.8, opacity: 1, easing: "ease-in" },
  { offset: 1, opacity: 0, transform: "translate3d(0, 14px, 0) scale(0.88)" },
];
const SETTLE: Keyframe[] = [
  { offset: 0, transform: "rotateY(0deg)", easing: "cubic-bezier(0.3, 0, 0.6, 1)" },
  { offset: 0.38, transform: "rotateY(-2.2deg)", easing: "cubic-bezier(0.5, 0, 0.9, 0.6)" },
  { offset: 0.66, transform: "rotateY(0deg)", easing: "cubic-bezier(0.1, 0.4, 0.5, 1)" },
  { offset: 0.82, transform: "rotateY(-0.45deg)", easing: "cubic-bezier(0.5, 0, 0.9, 0.6)" },
  { offset: 1, transform: "rotateY(0deg)" },
];

function still(): boolean {
  return matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function copyOf(section: HTMLElement, lift: number): HTMLElement {
  const ink = section.cloneNode(true) as HTMLElement;
  ink.removeAttribute("id");
  ink.removeAttribute("role");
  ink.removeAttribute("aria-labelledby");
  ink.hidden = false;
  ink.style.transform = `translateY(${-lift}px)`;
  return ink;
}

function sheetOf(section: HTMLElement, lift: number, width: number, gutter: number): HTMLDivElement {
  const sheet = document.createElement("div");
  sheet.className = "leaf-sheet";
  sheet.style.width = `${width}px`;
  if (gutter > 0) sheet.style.borderInlineEnd = `${gutter}px solid transparent`;
  sheet.append(copyOf(section, lift));
  return sheet;
}

function stroke(area: HTMLElement | null, act: (by: number) => void): void {
  if (!area) return;
  let from: { readonly id: number; readonly x: number; readonly y: number } | null = null;
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
}

function unseen(node: Element, hidden: boolean): void {
  if (hidden) node.setAttribute("aria-hidden", "true");
  else node.removeAttribute("aria-hidden");
}

function pose(laid: number, travel: number): TurnPose {
  const off = 1 - laid;
  return {
    top: { transform: `translate3d(${(off * travel).toFixed(1)}px, 0, 0) rotate(${(off * 3).toFixed(2)}deg) scale(${(1 + off * 0.04).toFixed(4)})` },
    shadow: { opacity: (0.35 + 0.65 * (1 - laid ** 3)).toFixed(3) },
    base: { transform: `translate3d(${(-laid * 7).toFixed(2)}%, 0, 0) scale(${(1 - laid * 0.08).toFixed(4)})`, opacity: (1 - laid).toFixed(3) },
  };
}

function place(parts: TurnParts, laid: number): void {
  const at = pose(laid, parts.travel);
  parts.top.sheet.style.transform = String(at.top.transform ?? "");
  parts.top.shadow.style.opacity = String(at.shadow.opacity ?? "");
  parts.base.sheet.style.transform = String(at.base.transform ?? "");
  parts.base.sheet.style.opacity = String(at.base.opacity ?? "");
}

function piece<K extends keyof HTMLElementTagNameMap>(name: string, parent: Element, tag?: K): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag ?? ("div" as K));
  node.className = name;
  parent.append(node);
  return node;
}

function rad(angle: number): number {
  return (angle * Math.PI) / 180;
}

function facing(angle: number): number {
  return 0.45 * Math.sin(rad(angle)) + 0.89 * Math.cos(rad(angle));
}

function round(value: number): number {
  return Number(value.toFixed(3));
}

function ended(runs: readonly Animation[]): Promise<boolean> {
  return Promise.all(runs.map((run) => run.finished)).then(
    () => true,
    () => false,
  );
}

function page(): void {
  const album = document.getElementById("album");
  const viewer = document.getElementById("photo-viewer");

  if (album instanceof HTMLDialogElement && viewer instanceof HTMLDialogElement) {
    const tabs = [...album.querySelectorAll<HTMLElement>('[role="tab"]')];
    const chapters = tabs.map((tab) => document.getElementById(tab.getAttribute("aria-controls") ?? ""));
    const storyPage = album.querySelector<HTMLElement>(".story-page");
    const leaves = album.querySelector<HTMLElement>(".story-leaves");
    const photos = [...album.querySelectorAll<HTMLImageElement>(".photo-preview img")];
    const caption = album.querySelector<HTMLElement>("[data-caption]");
    const counts = document.querySelectorAll<HTMLElement>("[data-photo-count]");
    const chapterCount = album.querySelector<HTMLElement>("[data-chapter-count]");
    const previousChapter = album.querySelector<HTMLButtonElement>('[data-chapter-step="-1"]');
    const nextChapter = album.querySelector<HTMLButtonElement>('[data-chapter-step="1"]');
    const fullCaption = viewer.querySelector<HTMLElement>("[data-viewer-caption]");
    let chapter = 0;
    let photo = 0;

    const showChapter = (to: number): void => {
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

    const STRIPS = 10;
    const turnLeaf = (from: number, to: number): void => {
      const leaving = chapters[from];
      const entering = chapters[to];
      if (!leaving || !entering || !leaves || !storyPage || still()) return;
      for (const old of leaves.querySelectorAll("[data-turning]")) old.remove();
      const forward = to > from;
      const width = leaves.clientWidth;
      const gutter = storyPage.offsetWidth - storyPage.clientWidth;
      const scrolled = storyPage.scrollTop;
      const parts: HTMLDivElement[] = [];
      if (!forward) {
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
      let hinge: HTMLElement = leaf;
      let first: HTMLDivElement | null = null;
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
      first?.addEventListener("animationend", (event) => {
        if (event.target === first) for (const part of parts) part.remove();
      });
    };

    const turn = (to: number, focusTab = false): void => {
      const next = Math.max(0, Math.min(chapters.length - 1, to));
      if (next === chapter) return;
      turnLeaf(chapter, next);
      chapter = next;
      showChapter(chapter);
      storyPage?.scrollTo({ top: 0, behavior: "instant" });
      if (focusTab) tabs[chapter]?.focus();
    };

    tabs.forEach((tab, at) => tab.addEventListener("click", () => turn(at)));
    previousChapter?.addEventListener("click", () => turn(chapter - 1));
    nextChapter?.addEventListener("click", () => turn(chapter + 1));

    const showPhoto = (to: number, forward: boolean, swept: SweptTurn | null = null): void => {
      const before = photos[photo];
      photo = (to + photos.length) % photos.length;
      const next = photos[photo];
      const layPhotos = before !== next && before?.hidden === false && !still();
      for (const image of photos) {
        const under = layPhotos && image === before;
        image.classList.toggle("under", under);
        image.hidden = image !== next && !under;
      }
      if (next) {
        next.classList.toggle("back", !forward);
        next.style.animation = "none";
        void next.offsetWidth;
        next.style.animation = "";
      }
      const words = next?.alt ?? "";
      if (caption) caption.textContent = words;
      if (fullCaption) fullCaption.textContent = words;
      for (const count of counts) count.textContent = `${photo + 1} / ${photos.length}`;
      if (viewer.open) turnFull(forward, swept);
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
    const step = (by: number): void => showPhoto(photo + by, by > 0);
    for (const button of document.querySelectorAll<HTMLElement>("[data-photo-step]")) {
      button.addEventListener("click", () => step(Number(button.getAttribute("data-photo-step"))));
    }

    stroke(leaves, (by) => turn(chapter + by));
    stroke(album.querySelector<HTMLElement>(".photo-preview"), (by) => step(by));

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

    const viewport = viewer.querySelector<HTMLElement>("[data-viewport]");
    const level = viewer.querySelector<HTMLElement>("[data-zoom-level]");
    const zoomOut = viewer.querySelector<HTMLButtonElement>('[data-zoom="-1"]');
    const zoomIn = viewer.querySelector<HTMLButtonElement>('[data-zoom="1"]');
    let view = { zoom: 1, x: 0, y: 0 };
    let drag: { readonly id: number; readonly x: number; readonly y: number; readonly origin: { zoom: number; x: number; y: number } } | null = null;

    const prints: PhotoPrint[] = [];
    let ring = 0;
    const near = (by: number): PhotoPrint => prints[(((ring + by) % 3) + 3) % 3];
    let full = viewer.querySelector("[data-viewer-image]");
    if (viewport instanceof HTMLElement && full instanceof HTMLImageElement) {
      for (let at = 0; at < 3; at += 1) {
        const image = at === 0 ? full : document.createElement("img");
        if (at > 0) {
          image.alt = "";
          image.decoding = "async";
          image.draggable = false;
        }
        const sheet = document.createElement("div");
        sheet.className = "photo-print";
        sheet.hidden = at > 0;
        const shadow = document.createElement("span");
        shadow.className = "photo-print-shadow";
        sheet.append(shadow, image);
        viewport.append(sheet);
        prints.push({ sheet, shadow, image, index: -1, large: false });
        image.addEventListener("load", () => {
          if (image === full) draw(view);
        });
      }
      viewport.dataset.prints = "";
    }
    const wrap = (index: number): number => ((index % photos.length) + photos.length) % photos.length;
    const load = (print: PhotoPrint | undefined, index: number, large: boolean): void => {
      const at = wrap(index);
      const source = photos[at];
      if (!print || !source) return;
      if (print.index === at && (print.large || !large)) return;
      print.index = at;
      print.large = large;
      const ratio = Number(source.getAttribute("width")) / Number(source.getAttribute("height"));
      print.sheet.style.setProperty("--ratio", String(Number.isFinite(ratio) && ratio > 0 ? ratio : 1));
      print.image.src = large ? (source.dataset.full ?? source.src) : source.src;
      print.image.alt = source.alt;
      if (!large) print.image.decode().catch(() => {});
    };

    const clamp = (next: { zoom: number; x: number; y: number }): { zoom: number; x: number; y: number } => {
      if (!(viewport instanceof HTMLElement) || !(full instanceof HTMLImageElement) || !full.naturalWidth) {
        return { zoom: next.zoom, x: 0, y: 0 };
      }
      const fitRatio = Math.min(viewport.clientWidth / full.naturalWidth, viewport.clientHeight / full.naturalHeight);
      const x = Math.max(0, (full.naturalWidth * fitRatio * next.zoom - viewport.clientWidth) / 2);
      const y = Math.max(0, (full.naturalHeight * fitRatio * next.zoom - viewport.clientHeight) / 2);
      return { zoom: next.zoom, x: Math.max(-x, Math.min(x, next.x)), y: Math.max(-y, Math.min(y, next.y)) };
    };
    const draw = (next: { zoom: number; x: number; y: number }): void => {
      view = clamp(next);
      if (full instanceof HTMLElement) full.style.transform = `translate(${view.x}px, ${view.y}px) scale(${view.zoom})`;
      if (viewport instanceof HTMLElement) viewport.toggleAttribute("data-zoomed", view.zoom > 1);
      if (level) level.textContent = `${Math.round(view.zoom * 100)} %`;
      if (zoomOut) zoomOut.disabled = view.zoom <= 1;
      if (zoomIn) zoomIn.disabled = view.zoom >= 4;
    };
    const zoom = (by: number): void => draw({ ...view, zoom: Math.max(1, Math.min(4, view.zoom + by)) });
    const fit = (): void => draw({ zoom: 1, x: 0, y: 0 });

    const own = (): void => {
      if (prints.length === 0) return;
      full = near(0).image;
      for (const print of prints) print.image.toggleAttribute("data-viewer-image", print === near(0));
    };
    let turning: { readonly runs: Animation[]; readonly parts: TurnParts } | null = null;
    let swipe: {
      readonly id: number;
      readonly x: number;
      readonly y: number;
      forward: boolean | null;
      parts: TurnParts | null;
      laid: number;
      readonly trail: { readonly x: number; readonly t: number }[];
    } | null = null;
    const rest = (): void => {
      if (turning) for (const run of turning.runs) run.cancel();
      turning = null;
      viewport?.removeAttribute("data-turning");
      for (const print of prints) {
        const isShown = print === near(0);
        print.sheet.hidden = !isShown;
        print.sheet.removeAttribute("data-top");
        unseen(print.sheet, !isShown);
        for (const part of [print.sheet, print.shadow]) {
          part.style.removeProperty("transform");
          part.style.removeProperty("opacity");
        }
        if (!isShown) print.image.style.removeProperty("transform");
      }
      own();
    };
    const lay = (): void => {
      rest();
      if (prints.length === 0) return;
      load(near(0), photo, true);
      load(near(1), photo + 1, false);
      load(near(-1), photo - 1, false);
    };
    const showFull = (): void => {
      lay();
      fit();
    };

    const TURN = 600;
    const begin = (forward: boolean, index: number): TurnParts => {
      const incoming = forward ? near(1) : near(-1);
      load(incoming, index, incoming.large);
      const top = forward ? incoming : near(0);
      const base = forward ? near(0) : incoming;
      const wide = viewport?.clientWidth ?? 0;
      const ratio = Number(top.sheet.style.getPropertyValue("--ratio")) || 1;
      const travel = (wide + Math.min(wide, (viewport?.clientHeight ?? 0) * ratio)) / 2 + 64;
      for (const print of [top, base]) {
        print.sheet.hidden = false;
        unseen(print.sheet, print !== incoming);
      }
      top.sheet.toggleAttribute("data-top", true);
      viewport?.toggleAttribute("data-turning", true);
      const parts: TurnParts = { top, base, incoming, forward, travel };
      place(parts, forward ? 0 : 1);
      return parts;
    };
    const glide = (parts: TurnParts, from: number, to: number, duration: number, easing: string, done: () => void): void => {
      const frames = (part: PrintPartKey): Keyframe[] => [0, 0.25, 0.5, 0.75, 1].map((s) => pose(from + (to - from) * s, parts.travel)[part]);
      const timing: KeyframeAnimationOptions = { duration, easing, fill: "both" };
      const runs = [
        parts.top.sheet.animate(frames("top"), timing),
        parts.top.shadow.animate(frames("shadow"), timing),
        parts.base.sheet.animate(frames("base"), timing),
      ];
      const run = { runs, parts };
      turning = run;
      Promise.all(runs.map((one) => one.finished)).then(
        () => {
          if (turning === run) done();
        },
        () => {},
      );
    };
    const turnFull = (forward: boolean, swept: SweptTurn | null): void => {
      if (prints.length === 0) return;
      if (swept) {
        const { parts, laid, duration } = swept;
        ring = prints.indexOf(parts.incoming);
        own();
        fit();
        glide(parts, laid, parts.forward ? 1 : 0, duration, "cubic-bezier(0.2, 0.7, 0.3, 1)", lay);
        return;
      }
      swipe = null;
      rest();
      if (near(0).index === photo) {
        lay();
        return;
      }
      if (still()) {
        const incoming = forward ? near(1) : near(-1);
        if (incoming.index === photo) ring = prints.indexOf(incoming);
        lay();
        fit();
        return;
      }
      const parts = begin(forward, photo);
      ring = prints.indexOf(parts.incoming);
      own();
      fit();
      const run = { runs: [] as Animation[], parts };
      turning = run;
      const image = parts.incoming.image;
      const drawn = image.complete ? Promise.resolve() : Promise.race([image.decode().catch(() => {}), new Promise<void>((done) => setTimeout(done, 250))]);
      void drawn.then(() => {
        if (turning !== run) return;
        glide(parts, forward ? 0 : 1, forward ? 1 : 0, TURN, "cubic-bezier(0.32, 0.72, 0, 1)", lay);
      });
    };

    viewer.addEventListener("toggle", (event) => {
      swipe = null;
      if ((event as ToggleEvent).newState === "open") showFull();
      else lay();
    });
    album.querySelector(".album-photo-open")?.addEventListener("click", () => requestAnimationFrame(showFull));
    for (const button of viewer.querySelectorAll<HTMLElement>("[data-zoom]")) {
      button.addEventListener("click", () => {
        const by = Number(button.getAttribute("data-zoom"));
        if (by === 0) fit();
        else zoom(by * 0.5);
      });
    }
    let sweep = { sum: 0, last: -Infinity, spent: false };
    viewport?.addEventListener(
      "wheel",
      (event) => {
        event.preventDefault();
        if (Math.abs(event.deltaX) <= Math.abs(event.deltaY)) {
          zoom(event.deltaY < 0 ? 0.5 : -0.5);
          return;
        }
        if (view.zoom > 1) {
          draw({ ...view, x: view.x - event.deltaX });
          return;
        }
        if (event.timeStamp - sweep.last > 220) sweep = { sum: 0, last: 0, spent: false };
        sweep.last = event.timeStamp;
        sweep.sum += event.deltaX;
        if (!sweep.spent && Math.abs(sweep.sum) > 60) {
          sweep.spent = true;
          step(sweep.sum > 0 ? 1 : -1);
        }
      },
      { passive: false },
    );
    const capture = (event: PointerEvent): void => {
      try {
        viewport?.setPointerCapture(event.pointerId);
      } catch {
        // a pointer that is already gone has nothing to capture
      }
    };
    viewport?.addEventListener("pointerdown", (event) => {
      if (event.button !== 0 || view.zoom <= 1 || drag) return;
      event.preventDefault();
      capture(event);
      viewport.toggleAttribute("data-dragging", true);
      drag = { id: event.pointerId, x: event.clientX, y: event.clientY, origin: view };
    });
    viewport?.addEventListener("pointermove", (event) => {
      if (!drag || drag.id !== event.pointerId) return;
      draw({ ...drag.origin, x: drag.origin.x + event.clientX - drag.x, y: drag.origin.y + event.clientY - drag.y });
    });
    const release = (): void => {
      drag = null;
      viewport?.removeAttribute("data-dragging");
    };
    viewport?.addEventListener("pointerup", release);
    viewport?.addEventListener("pointercancel", release);
    viewport?.addEventListener("lostpointercapture", release);
    if (viewport) new ResizeObserver(() => draw(view)).observe(viewport);

    const NUDGE = 8;
    viewport?.addEventListener("pointerdown", (event) => {
      if (event.button !== 0 || !event.isPrimary || view.zoom > 1 || drag || prints.length === 0) return;
      lay();
      capture(event);
      swipe = { id: event.pointerId, x: event.clientX, y: event.clientY, forward: null, parts: null, laid: 0, trail: [{ x: event.clientX, t: event.timeStamp }] };
    });
    viewport?.addEventListener("pointermove", (event) => {
      if (!swipe || swipe.id !== event.pointerId) return;
      const across = event.clientX - swipe.x;
      const down = event.clientY - swipe.y;
      swipe.trail.push({ x: event.clientX, t: event.timeStamp });
      while (swipe.trail.length > 2 && event.timeStamp - swipe.trail[0].t > 100) swipe.trail.shift();
      if (swipe.forward === null) {
        if (Math.abs(across) < NUDGE) return;
        if (Math.abs(down) > Math.abs(across)) {
          swipe = null;
          return;
        }
      }
      const forward = across < 0;
      if (still()) {
        swipe.forward = forward;
        return;
      }
      if (forward !== swipe.forward || !swipe.parts) {
        if (swipe.parts) rest();
        swipe.forward = forward;
        swipe.parts = begin(forward, photo + (forward ? 1 : -1));
      }
      const reach = Math.min(1, Math.abs(across) / swipe.parts.travel);
      swipe.laid = forward ? reach : 1 - reach;
      place(swipe.parts, swipe.laid);
    });
    const letGo = (event: PointerEvent, kept: boolean): void => {
      if (!swipe || swipe.id !== event.pointerId) return;
      const currentStroke = swipe;
      swipe = null;
      if (currentStroke.forward === null) return;
      const across = event.clientX - currentStroke.x;
      const first = currentStroke.trail[0];
      const speed = first && event.timeStamp > first.t ? (event.clientX - first.x) / (event.timeStamp - first.t) : 0;
      const flick = Math.abs(speed) > 0.45;
      const onward = kept && (flick ? Math.sign(speed) === Math.sign(across) : Math.abs(across) > 48);
      if (!currentStroke.parts) {
        if (onward) step(currentStroke.forward ? 1 : -1);
        return;
      }
      const reach = Math.abs(across) / currentStroke.parts.travel;
      const goes = kept && (flick ? Math.sign(speed) === Math.sign(across) : reach > 1 / 3);
      const to = currentStroke.forward === goes ? 1 : 0;
      const way = Math.abs(to - currentStroke.laid) * currentStroke.parts.travel;
      const duration = Math.round(Math.max(160, Math.min(440, (way / Math.max(Math.abs(speed), 0.8)) * 1.7)));
      if (goes) {
        showPhoto(photo + (currentStroke.forward ? 1 : -1), currentStroke.forward, { parts: currentStroke.parts, laid: currentStroke.laid, duration });
      } else {
        glide(currentStroke.parts, currentStroke.laid, to, duration, "cubic-bezier(0.2, 0.7, 0.3, 1)", lay);
      }
    };
    viewport?.addEventListener("pointerup", (event) => letGo(event, true));
    viewport?.addEventListener("pointercancel", (event) => letGo(event, false));

    viewer.addEventListener("keydown", (event) => {
      if (event.ctrlKey || event.metaKey) return;
      const keys: Record<string, () => void> = {
        ArrowRight: () => step(1),
        ArrowLeft: () => step(-1),
        "+": () => zoom(0.5),
        "=": () => zoom(0.5),
        "-": () => zoom(-0.5),
        "0": fit,
      };
      const act = Object.hasOwn(keys, event.key) ? keys[event.key] : undefined;
      if (!act) return;
      event.preventDefault();
      event.stopPropagation();
      act();
    });

    const openers = [...document.querySelectorAll('button[commandfor="album"]')];
    const logo = document.querySelector(".hero-logo");
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

    const swing: readonly (readonly [HTMLElement, (angle: number) => Keyframe])[] = [
      [cover, (angle) => ({ transform: `rotateY(${-angle}deg)` })],
      [frontDim, (angle) => ({ opacity: angle > 90 ? 0.55 : round(0.7 * Math.max(0, 1 - Math.max(0, facing(angle)) ** 2 / LIT)) })],
      [glint, (angle) => ({ opacity: angle < 60 ? round(Math.min(1, Math.max(0, facing(angle) ** 2 - LIT) / 0.2)) : 0, transform: `translateX(${Math.min(angle, 60) - 30}%)` })],
      [back, (angle) => ({ opacity: angle < 160 ? 1 : round((180 - angle) / 20) })],
      [backDim, (angle) => ({ opacity: round(0.65 * (1 - Math.min(1, Math.max(0, -facing(angle)) / 0.89))) })],
      [
        shade,
        (angle) => {
          const reach = Math.max(0, Math.min(1, Math.cos(rad(angle)) + 0.506 * Math.sin(rad(angle))));
          return { opacity: reach > 0 ? round(0.85 * Math.min(1, reach * 1.6) * (1 - angle / 200)) : 0, transform: `scaleX(${Math.max(0.001, round(reach))})` };
        },
      ],
    ];

    let folding: BookFoldState | null = null;
    const unfold = (): void => {
      const runs = folding?.runs ?? [];
      folding = null;
      for (const run of runs) run.cancel();
      delete album.dataset.folding;
    };
    const fade = (isOpening: boolean): Animation | null => {
      try {
        return album.animate(
          { opacity: isOpening ? [0, 1] : [1, 0] },
          isOpening ? { duration: 420, easing: "ease-out", fill: "both", pseudoElement: "::backdrop" } : { duration: 220, delay: 640, easing: "ease-in", fill: "both", pseudoElement: "::backdrop" },
        );
      } catch {
        return null;
      }
    };
    const fold = (isOpening: boolean): Promise<boolean> => {
      album.dataset.folding = isOpening ? "open" : "shut";
      const timing: KeyframeAnimationOptions = { duration: isOpening ? 900 : 650, delay: isOpening ? 160 : 0, easing: isOpening ? SWING : SWING_BACK, fill: "both" };
      const frames = (poseFn: (angle: number) => Keyframe): Keyframe[] => {
        const all = ANGLES.map((angle) => ({ offset: angle / 180, ...poseFn(angle) }));
        return isOpening ? all : all.toReversed().map((frame) => ({ ...frame, offset: round(1 - (frame.offset ?? 0)) }));
      };
      const veil = isOpening ? null : fade(false);
      const runs = [
        album.animate(isOpening ? BOOK_OPEN : BOOK_SHUT, { duration: isOpening ? 1060 : 860, fill: "both" }),
        ...swing.map(([node, poseFn]) => node.animate(frames(poseFn), timing)),
        ...(isOpening ? pages.map((p) => p.animate(SETTLE, { duration: 560, delay: 560, fill: "both" })) : []),
        ...(veil ? [veil] : []),
      ];
      folding = { opening: isOpening, runs };
      return ended(runs);
    };
    const cloth = (): void => {
      if (!(logo instanceof HTMLImageElement)) return;
      const picture = logo.currentSrc || logo.src;
      if (picture && mark.src !== picture) mark.src = picture;
    };
    const opening = (): void => {
      if (album.open || folding || still()) return;
      cloth();
      const run = fold(true);
      const mine = folding as BookFoldState | null;
      requestAnimationFrame(() => {
        const veil = folding === mine && mine?.opening && album.open ? fade(true) : null;
        if (veil && mine) mine.runs.push(veil);
      });
      void run.then((done) => {
        if (done && folding === mine && mine?.opening) unfold();
      });
    };
    const shut = (): void => {
      if (!album.open) return;
      if (still()) {
        album.close();
        return;
      }
      if (folding && !folding.opening) return;
      let run: Promise<boolean>;
      if (folding) {
        folding.opening = false;
        album.dataset.folding = "shut";
        for (const one of folding.runs) one.reverse();
        run = ended(folding.runs);
      } else {
        run = fold(false);
      }
      void run.then((done) => {
        if (done && album.open) album.close();
      });
    };
    for (const opener of openers) {
      opener.addEventListener("click", opening);
      const ready = (): void => {
        cloth();
        if (mark.src) mark.decode().catch(() => {});
      };
      for (const kind of ["pointerenter", "focus", "touchstart"] as const) opener.addEventListener(kind, ready, { once: true, passive: true });
    }
    album.addEventListener("beforetoggle", (event) => {
      if ((event as ToggleEvent).newState === "open") opening();
    });
    album.addEventListener("close", unfold);
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

    showChapter(0);
    showPhoto(0, true);
    album.dataset.ready = "";
  }

  const MOVING = "(prefers-reduced-motion: no-preference)";
  const animate = (picture: Element, srcset: string, sizes: string): void => {
    let source = picture.querySelector<HTMLSourceElement>("source[data-moving-source]");
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

  const draw = (button: HTMLElement): void => {
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

  const cards = [...document.querySelectorAll<HTMLElement>("button.shot-open[data-stills]")];
  if (typeof IntersectionObserver === "function") {
    const near = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          near.unobserve(entry.target);
          if (entry.target instanceof HTMLElement) draw(entry.target);
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
    const shots = [...document.querySelectorAll<HTMLElement>(".shot-open")];
    const picture = shotViewer.querySelector("[data-shot-picture]");
    const image = shotViewer.querySelector("[data-shot-image]");
    const words = shotViewer.querySelector("[data-shot-caption]");
    let at = 0;
    const show = (to: number): void => {
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

  const counters = [...document.querySelectorAll<HTMLElement>("dt[data-count]")];
  if (counters.length > 0 && typeof IntersectionObserver === "function" && matchMedia(MOVING).matches) {
    const write = new Intl.NumberFormat(document.documentElement.lang || "en");
    const run = (node: HTMLElement): void => {
      const end = Number(node.dataset.count);
      const finalText = node.textContent ?? "";
      if (!Number.isFinite(end) || end <= 0) {
        return;
      }
      const began = performance.now();
      const span = 1100;
      const step = (now: number): void => {
        const part = Math.min(1, (now - began) / span);
        const eased = 1 - (1 - part) ** 3;
        node.textContent = part < 1 ? write.format(Math.round(end * eased)) : finalText;
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
            if (entry.target instanceof HTMLElement) run(entry.target);
          }
        }
      },
      { threshold: 0.6 },
    );
    for (const node of counters) {
      seen.observe(node);
    }
  }
}

page();
document.addEventListener("baylee:page", page);
