# Website design guidelines

*30.10.2026.* How baylee.app looks, moves and is put together: the public
pages, the account area and the administration. It binds every change under
`web/`. The words are in `web/README.md` §"The words"; the security rules of
the pages are in [web accounts](web-accounts.md). Where this document and a
stylesheet disagree, the stylesheet is wrong.

## The mood

Baylee in sunlight by day, the blue hour by night.

- **Light scheme: sunlight.** Warm ivory ground, garden teal for structure,
  soft gold for the one thing to do. The owner-approved painting of Baylee
  among clouds, birds and blossoms leads the start page; nothing competes
  with it.
- **Dark scheme: the blue hour.** The client's lobby key (Baylee
  `docs/client-lobby.md`): a deep twilight blue ground, cool cornflower for
  information, and the same warm gold as the last light on the horizon for
  the one thing to do.
- **Calm.** One accent per view. Few borders, generous space, no badge or
  glow that does not answer a question.
- **Symmetric.** Page heads, section heads and dialogs are centred. Columns
  are equal, gutters are equal, a grid of cards fills its rows evenly. On a
  wide screen, content stands in the middle and never hugs a corner.

## Principles

1. **Read at a glance.** A visitor answers "what is this, what state is it
   in, what can I do" without hunting. State is shown in form *and* in
   words: a dot and "online", a pill and "revoked", never colour alone.
2. **One primary action per view.** It is the gold button. Everything else
   is a ghost button, a link or a quiet icon button.
3. **Feel fast.** Every pointer, key or touch gets an answer within a frame;
   transitions take 120–200 ms. Anything slower than 400 ms is a loading
   state, and it says so.
4. **Honest.** A button that is live succeeds. A list that failed says it
   failed and offers to try again; it never shows an empty list instead. A
   secret is shown once, in its own box, with what to do with it.
5. **Nothing from elsewhere.** No font, script, tracker or picture from
   another host, except the ones the CSP names (`web/scripts/csp.ts`).

## Colour

Every colour is a custom property in `web/src/styles/tokens.css`, defined
for the dark scheme in `:root`, for the light scheme under
`prefers-color-scheme: light` and again under `[data-theme="light"]`. The
header's colour switch goes from the device's scheme to light, dark and back
(`src/theme.ts`); the browser keeps the choice like the language (the
privacy notice's "language and display"), `public/boot.js` applies it before
the first paint, and `?theme=` overrides it for one visit. No other file names a raw colour, except
the artwork's own: the memorial's parchment (`memorial-book.css`,
`remembrance.css`) and the painted light over the world (`hero.css`,
`world.css`).

Tokens are named for what they mean, not for how they look:

| token | meaning | dark (blue hour) | light (sunlight) |
| --- | --- | --- | --- |
| `--bg` | the page ground | `#0f1b2d` | `#f5f2e9` |
| `--bg-alt` | every other section's ground | `#13223a` | `#e9eee8` |
| `--panel-solid` | a panel, card or field ground | `#172741` | `#fffdf5` |
| `--panel` | the same, translucent over the painting | `rgba(23 39 65 / 0.94)` | `rgba(255 253 246 / 0.96)` |
| `--panel-2` | a raised or hovered surface inside a panel | `#1f3253` | `#eceee5` |
| `--ink` | body text | `#eef1f8` | `#233e43` |
| `--ink-soft` | secondary text | `#cdd6ea` | `#415c5e` |
| `--ink-faint` | captions, labels, meta | `#a3b3d1` | `#536c6b` |
| `--title` | headings | `#fff7e4` | `#173c43` |
| `--gold` | the primary action, the selected choice | `#e3c07e` | `#235b66` |
| `--gold-ink` | text on `--gold` | `#1a1306` | `#fffaf0` |
| `--violet` | information: eyebrows, links in lists, the info accent | `#aebfff` | `#326b75` |
| `--link` | a link in prose | `#ebcd8f` | `#76511c` |
| `--focus` | the focus ring | `#ffe7a6` | `#235b66` |
| `--ok`, `--warn`, `--bad`, `--off` | states: fine, attention, failure or danger, inactive | | |
| `--edge`, `--edge-soft` | a border that matters, a border that only separates | | |

Rules:

- **Contrast**, WCAG 2.1 AA: text 4.5:1 on every surface it is drawn on,
  large text (≥ 1.2rem bold, ≥ 1.5rem) 3:1, borders that carry meaning and
  the focus ring 3:1. `web/test/styles/contrast.test.ts` measures the token
  file in both schemes and fails a pair that drops below.
- **State colours are not the accent.** `--ok`, `--warn` and `--bad` never
  stand in for the gold, and the gold never says "fine".
- **Danger** (revoke, lock, delete) is a ghost button whose text and border
  turn `--bad` on hover, and always asks a second time in a dialog.
- **Card pictures** are never filtered, faded, blended or tinted (Baylee's
  `docs/legal.md` §3; `test/styles/rules.test.ts`).

## Typography

| role | family | where |
| --- | --- | --- |
| Display and prose | Faustina (serif, variable 300–800, italic) | headings, paragraphs, the memorial |
| Interface | Alegreya Sans (400, 500, 700) | buttons, navigation, labels, tables, pills, forms |
| Codes | the system monospace | addresses, versions, tokens, keys, recovery codes |

Both fonts are SIL OFL 1.1 and served from this site (`public/fonts/`).

The scale (`--text-*`, rem at a root of 16–20 px by viewport):

| token | size | use |
| --- | --- | --- |
| `--text-xs` | 0.78rem | eyebrows (uppercase, tracked 0.16em), pills |
| `--text-sm` | 0.9rem | captions, meta, table cells |
| `--text-md` | 1rem | body, buttons, fields |
| `--text-lg` | 1.12–1.32rem | card titles (`h3`) |
| `--text-xl` | 1.7–2.6rem | section titles (`h2`) |
| `--text-2xl` | 2.2–4rem | page titles (`h1`) |

- Nothing smaller than 12 px on any screen.
- Prose uses old-style figures (`onum`); tables, counters, dates and times
  use tabular lining figures (`.num`, `font-variant-numeric: tabular-nums
  lining-nums`), so columns of numbers line up.
- Headings balance (`text-wrap: balance`), paragraphs avoid orphans
  (`text-wrap: pretty`). A line of prose stays under 70 characters
  (`--prose`).
- A name a person chose (a display name, a gateway's name) is drawn in
  `<bdi>`, never translated (`translate="no"` on handles), and cut with an
  ellipsis rather than wrapped in tight places.

## Space and layout

- **One step is 4 px.** The scale is `--space-1` (4 px) to `--space-12`
  (96 px): 4, 8, 12, 16, 24, 32, 40, 48, 64, 80, 96. Padding, gaps and
  margins take a step from it.
- **Widths.** `--prose` 46rem for reading, `--wrap` 72rem for a page,
  `--wrap-app` 80rem for the account area and the administration.
  `--gutter` 16 px at the screen's edge on a phone.
- **Breakpoints.** Three, no others: 600 px (phone to tablet), 860 px
  (one column to two, the bar to its menu) and 1100 px (the app's side
  navigation to a top row).
- **Grids** use `repeat(auto-fill, minmax(…, 1fr))`, so every column of a
  row is as wide as the others; a lone last card sits in the middle
  (`.grid-centred`).
- **Sections** alternate `--bg` and `--bg-alt`, each with an eyebrow, a
  centred title and an optional lead.
- **No sideways scroll** at 320 px. Wide tables scroll inside their own box.

## Shape and depth

- Radii: `--radius-sm` 8 px (fields, codes), `--radius-md` 12 px (pills,
  small panels), `--radius` 18 px (cards, panels, dialogs), and pills for
  buttons.
- Depth: `--shadow` for a resting panel, `--shadow-lift` for one under the
  pointer. Glass (`.glass`) only where a panel stands over the painting.

## Components

Each lives in one stylesheet named after it and one component under
`web/src/components/` when it has behaviour.

- **Buttons** (`buttons.css`): `.button` gold, primary, one per view;
  `.button-ghost` for every other action; `.button-danger` (ghost that
  turns `--bad`) for revoke, lock and delete; `.round-btn` for an icon
  alone, always with a hidden label; `.chip-btn` for a filter or a choice
  among few. 44 px high at least, on every screen.
- **Fields** (`forms.css`): label above, hint below, error below the hint in
  `--bad` with an icon and the words; `aria-describedby` ties both to the
  field. A field's border is 3:1.
- **Panels and cards** (`.glass`, `.card`): 18 px radius, one soft border,
  `--shadow`. A card that is a link lifts under the pointer.
- **Pills** (`.pill`): a state in words, coloured by the state token,
  `--text-xs`, never alone without the word.
- **Tables** (`tables.css`): for administration only. Header row in the
  interface face, tabular figures, row hover, the first column the thing's
  name, the last column its actions. On a phone a table becomes a list of
  cards (`.table-cards`).
- **The app frame** (`app.css`): the account area and the administration
  share one shell: a side navigation on the left at 1100 px and wider, a
  row of tabs above the content below it. The current item is marked with
  `aria-current="page"` and the gold rule.
- **Dialogs** (`components/Modal.tsx`): the native `<dialog>`, centred,
  focus inside, Escape and the close button close it, focus returns to the
  opener. A dangerous change names the thing it acts on and asks for the
  change in words on its button ("Revoke token"), never "OK".
- **The secret box** (`.secret`): a token, key or code shown once. Mono,
  selectable, a copy button, the warning "shown only now" above it, and the
  dialog does not close on a click outside until the person says they kept
  it. A secret is never put in a URL, the page's storage or a log.
- **States of a list**: loading is a skeleton of the list's shape
  (`.skeleton`), empty is a sentence and the next step, failure is the
  reason and "Try again". The three never look alike.
- **Live words** (`<output>` or `role="status"`): what an action did ("Address
  copied", "Token revoked") is said in a live region next to it.

## Motion

Motion explains a change; it never decorates a still page. The client's
rule holds here too (Baylee `.claude/skills/game-animation`, `ambience.rs`):
a control settles in about 120 ms, which is under the 150 ms where it starts
to feel slow and over the 60 ms where it reads as a snap.

| token | value | use |
| --- | --- | --- |
| `--dur-fast` | 120 ms | hover, press, colour |
| `--dur` | 180 ms | a menu, a popover, a page's cross-fade, a dialog |
| `--dur-slow` | 320 ms | a section coming into view |
| `--ease` | `cubic-bezier(0.22, 1, 0.36, 1)` | arriving (ease-out) |
| `--ease-in` | `cubic-bezier(0.4, 0, 1, 1)` | leaving |
| `--stagger` | 45 ms | between the cards of a grid as they arrive, at most six steps |

- **Only `opacity` and `transform`** animate, and the colours of a control.
  Nothing animates layout (width, height, top, margin).
- **Hover** lifts a card by 2 px and deepens its shadow; a button lightens.
  **Press** scales a button to 0.98. A row of text does not grow.
- **Arriving**: a section rises 14 px and fades in once as it is first read
  (`useReveal`); what is on screen at load never waits. The cards of a grid
  follow each other by `--stagger`.
- **Between pages** the main column cross-fades in `--dur` through the View
  Transitions API where the browser has it; elsewhere the page simply
  changes.
- **Ambient**: the online dot breathes slowly (2.4 s); the painting drifts
  with the scroll; the memorial's light is its own (limited to 20 draws a
  second). Nothing else moves by itself.
- **Reduced motion** (`prefers-reduced-motion: reduce`): every transition,
  animation, drift and count-up stops, and content is shown at once
  (`base.css` §"stillness", `motion.ts`). This is WCAG 2.3.3 and the same
  promise as the client's *reduce motion*.

## Words

The words are Fluent messages (`web/README.md` §"The words"). The reader is
*du* in German. A button says what it does, as a verb ("Create token",
"Revoke"), and a question asks in the words of the button that answers it.
An error says what happened and what to do next; it never blames, and it
never shows a server's diagnostic.

## Accessibility

WCAG 2.1 AA, mobile first:

- Landmarks (`header`, `nav`, `main`, `footer`), one `h1` per page, a skip
  link to `#main`, `document.title` per page.
- Every control works by keyboard, in reading order, with a visible focus
  ring (`--focus`, 3 px, offset 3 px). Menus close with Escape and return
  focus to their button.
- Touch targets are at least 44 × 44 px.
- An asynchronous result is announced (`aria-live` / `<output>`), and a list
  that loads says it is busy (`aria-busy`).
- Text scales to 200 % without loss; nothing is wider than 320 px forces.

## Areas

| area | paths | frame |
| --- | --- | --- |
| Public | `/`, `/download`, `/cards`, `/gateways`, `/contact`, legal pages | the bar, sections, the footer |
| Account | `/account`, `/account/security`, `/account/gateways` | the app frame |
| Administration | `/admin`, `/admin/users`, `/admin/users/<id>`, `/admin/gateways`, `/admin/audit`, `/admin/legacy` | the app frame, loaded only for an administrator |
| Sign-in | `/signin`, `/register` | a centred card beside Baylee's cloud |

The account area and the administration are loaded lazily and only after a
sign-in; a visitor's first page fetches neither.

## How it is held

- `test/styles/contrast.test.ts`: the token pairs above in both schemes.
- `test/styles/rules.test.ts`: no inline script or style in the shell, card
  pictures untouched, no raw colour outside `tokens.css`, a reduced-motion
  rule beside every animation.
- `e2e/`, under the production CSP at a phone's and a desktop's width:
  no sideways scroll on the main pages (`expectNoSideScroll`; the memorial
  at 320 px too), a gateway card's controls 44 px high with copying its
  address as the first action while gateway links are off
  (`VITE_DEEP_LINKS`, `web/README.md` §"Settings"), and the reduced-motion
  preference hiding nothing.
- `npm run screens`: every page at phone and desktop width in both schemes,
  for a person to look at before a change lands.
