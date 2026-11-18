# Website design guidelines

*Status, 12.11.2026: the React app these rules were written for was removed from `web/`. The paths marked as removed below are gone.*

*30.10.2026.* How baylee.app looks, moves and is put together: the public
pages, the account area and the administration. It binds every change under
`crates/central-web/`. The words were in the old `web/README.md` §"The words" (removed 12.11.2026); the security rules of
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
   another host, except the ones the CSP names (`web/scripts/csp.ts`, removed 12.11.2026).

## Colour

Every colour is a custom property in `web/src/styles/tokens.css` (removed 12.11.2026), defined
for the dark scheme in `:root`, for the light scheme under
`prefers-color-scheme: light` and again under `[data-theme="light"]`. The
header's colour switch went from the device's scheme to light, dark and back
(`src/theme.ts`); on the server-rendered site (`crates/central-web/js/boot.js`)
one press shows the scheme that is not on screen, and a choice the device
would make anyway is not kept, so the page follows the device again (owner,
12.11.2026: the first press on a light device had changed nothing). The
browser keeps the choice like the language (the
privacy notice's "language and display"), `public/boot.js` applies it before
the first paint, and `?theme=` overrides it for one visit. No other file names a raw colour, except
the artwork's own: the memorial's parchment (`memorial-book.css`,
`remembrance.css`) and the painted light over the world (`hero.css`,
`world.css`).

Tokens are named for what they mean, not for how they look:

| token | meaning | dark (blue hour) | light (sunlight) |
| --- | --- | --- | --- |
| `--bg` | the page ground | `#0f1b2d` | `#ebe6d6` |
| `--bg-alt` | every other section's ground | `#13223a` | `#dce6e0` |
| `--panel-solid` | a panel, card or field ground | `#172741` | `#fffffd` |
| `--panel` | the same, translucent over the painting | `rgba(23 39 65 / 0.94)` | `rgba(255 255 253 / 0.98)` |
| `--panel-2` | a raised or hovered surface inside a panel | `#1f3253` | `#e2e8dc` |
| `--ink` | body text | `#eef1f8` | `#14282d` |
| `--ink-soft` | secondary text | `#cdd6ea` | `#2e464a` |
| `--ink-faint` | captions, labels, meta | `#a3b3d1` | `#465d60` |
| `--title` | headings | `#fff7e4` | `#0e3037` |
| `--gold` | the primary action, the selected choice | `#e3c07e` | `#1b5562` |
| `--gold-ink` | text on `--gold` | `#1a1306` | `#fffaf0` |
| `--violet` | information: eyebrows, links in lists, the info accent | `#aebfff` | `#2a626d` |
| `--link` | a link in prose | `#ebcd8f` | `#6b4712` |
| `--focus` | the focus ring | `#ffe7a6` | `#1b5562` |
| `--ok`, `--warn`, `--bad`, `--off` | states: fine, attention, failure or danger, inactive | | |
| `--edge`, `--edge-soft` | a border that matters, a border that only separates | | |
| `--chart-reports`, `--chart-crashes` | the two series of the feedback section's 30-day chart, nowhere else | `#6f8ae8`, `#e0665a` | `#1b7fae`, `#b8402f` |

Rules:

- **Contrast**, WCAG 2.1 AA: text 4.5:1 on every surface it is drawn on,
  large text (≥ 1.2rem bold, ≥ 1.5rem) 3:1, borders that carry meaning and
  the focus ring 3:1. `web/test/styles/contrast.test.ts` (removed 12.11.2026) measures the token
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
| Display and prose | Literata (serif, variable, italic) | headings, paragraphs, the memorial |
| Interface | Inter (sans, variable) | buttons, navigation, labels, tables, pills, forms |
| Codes | the system monospace | addresses, versions, tokens, keys, recovery codes |

Both fonts are SIL OFL 1.1 and served from this site
(`crates/central-web/assets/fonts/`, each beside its licence text; the
Latin, extended Latin and Cyrillic subsets of `@fontsource-variable` 5.3.0,
chosen by `unicode-range`, so a page loads only the scripts it shows).
Japanese, Korean and Chinese are drawn with the reader's own system fonts
(`:root:lang(…)` in `base.css` names them per language): a web font for
them weighs megabytes, and the system's is the one its readers know. A
weight a font lacks is not synthesised.

The scale (`--text-*`, rem at a root of 16–20 px by viewport):

| token | size | use |
| --- | --- | --- |
| `--text-xs` | 0.78rem | eyebrows (uppercase, tracked 0.16em), pills |
| `--text-sm` | 0.9rem | captions, meta, table cells |
| `--text-md` | 1rem | body, buttons, fields |
| `--text-lg` | 1.12–1.32rem | card titles (`h3`) |
| `--text-xl` | 1.7–2.6rem | section titles (`h2`) |
| `--text-2xl` | 1.6–4rem | page titles (`h1`; the low end is a 320 px phone, where a long German title still stands whole) |

- Nothing smaller than 12 px on any screen.
- Prose uses old-style figures (`onum`); tables, counters, dates and times
  use tabular lining figures (`.num`, `font-variant-numeric: tabular-nums
  lining-nums`), so columns of numbers line up.
- Headings and leads balance (`text-wrap: balance`), paragraphs avoid
  orphans (`text-wrap: pretty`). A line of prose stays under 70 characters
  (`--prose`). A heading breaks inside a word only when the word alone is
  wider than the screen (`overflow-wrap: break-word`).
- What belongs together stays together: the words mark it (`<nb>…</nb>` in
  a message becomes `.keep`, one unbreakable piece, as each sentence of the
  start page's title is), and a compound that begins with a figure is
  written with non-breaking hyphens (`64‑Bit`, `4‑Spieler‑Runde`).
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
  (`.grid-centred`). The gateways' cards (`.gateways`) are one, two or
  three in a row by the breakpoints, on a grid with twice as many columns,
  so a last row that is not full stands in the middle too; the cards of a
  row share their lines (subgrid), so their figures and buttons stand
  level.
- **Sections** alternate `--bg` and `--bg-alt`, each with an eyebrow, a
  centred title and an optional lead.
- **No sideways scroll** at 320 px. Wide tables scroll inside their own box.

## Shape and depth

- Radii: `--radius-sm` 8 px (fields, codes), `--radius-md` 12 px (pills,
  small panels), `--radius` 18 px (cards, panels, dialogs), and pills for
  buttons.
- Depth: `--shadow` for a resting panel, `--shadow-lift` for one under the
  pointer (a deeper shade, the panel itself stays where it is),
  `--shadow-menu` for what floats over the page (a menu, a dialog's card):
  a sheen at its top edge, a hairline, and four shades from close to far,
  the farthest in the accent's colour. In the light scheme every shade is
  the scheme's own ink. Glass (`.glass`) only where a panel stands over the
  painting.

## Components

Each lives in one stylesheet named after it and one component under
`web/src/components/` (removed 12.11.2026) when it has behaviour.

- **Buttons** (`buttons.css`): `.button` gold, primary, one per view (one
  exception: a list whose rows each offer the same step gives that step
  the gold in every row: the gateways awaiting approval, and the gateway
  directory's "Open in Baylee");
  `.button-ghost` for every other action; `.button-danger` (ghost that
  turns `--bad`) for revoke, lock and delete; `.round-btn` for an icon
  alone, always with a hidden label; `.chip-btn` for a filter or a choice
  among few. 44 px high at least, on every screen. A button whose action
  takes a moment stays focusable while it works (`BusyButton`:
  `aria-disabled` and a spinner, never `disabled`), and a row's buttons are
  described by the row's name.
- **Fields** (`forms.css`): label above, hint below, error below the hint in
  `--bad` with an icon and the words; `aria-describedby` ties both to the
  field, and `aria-invalid` marks it while the problem stands. A field's
  border is 3:1. A password field (`PasswordInput`) has a button that shows
  what was typed (pressed state, spellcheck off) and says when Caps Lock is
  on.
- **Facts** (`.kv`): label and value side by side, the labels one column as
  wide as the longest, no rule under each; one column on a phone.
- **Copying** (`app/Copy.tsx`): a value someone pastes elsewhere (an
  address, a sign-in name, an id) has a quiet round copy button beside it,
  named in words; it shows a check for a moment and the live words say it,
  also the second time. An address (`Address`) breaks after `//` and before
  a dot, never inside a name, and is never cut with an ellipsis: its end (a
  port) is often what tells two apart.
- **Tiles** (`.stats`, the administration's first view): one count each, its
  label as an eyebrow, a sentence, and a link to where the work is; a count
  that asks for attention (gateways awaiting approval) in the state colour
  and in words, nowhere a badge without them.
- **Finding** (`data-find`): a page with a list long enough to search has one
  search field, which Ctrl+K (Cmd+K on a Mac) reaches from anywhere on the
  page (`aria-keyshortcuts`); Escape empties it. A search that asks Central
  follows the field after a short pause, the newest answer winning, and
  stands in the address (`?q=`).
- **Panels and cards** (`.glass`, `.card`): 18 px radius, one soft border,
  `--shadow`. A card that is a link deepens its shade under the pointer.
- **Pills** (`.pill`): a state in words, coloured by the state token,
  `--text-xs`, never alone without the word.
- **Tables** (`tables.css`): for administration only. Header row in the
  interface face, tabular figures, row hover, the first column the thing's
  name (in the heading's ink, underlined under the pointer), the last column
  its actions. A table stands in its own scroll box (`TableBox`), which the
  keyboard reaches and a screen reader names; on a phone each row stays one
  line and the box scrolls. (A list of cards on a phone was the first plan;
  restyling table rows drops their table semantics in Safari, so it waits
  for explicit roles.)
- **The app frame** (`app.css`): the account area and the administration
  share one shell: a side navigation on the left at 1100 px and wider, a
  row of tabs above the content below it, one line that scrolls sideways
  with the current tab in sight. The current item is marked with
  `aria-current="page"` and the gold rule. Its head is start-aligned, beside
  the navigation, unlike the centred page heads of the public pages.
  The account's navigation is in two groups, the account (overview,
  profile, security, privacy and data, notifications) and playing (chat,
  which holds the contacts, and gateways and tokens); the group names show
  beside the side navigation and not above the row of tabs. The server's
  page never links to the administration: an administrator's links are
  drawn by the island once Central said so.
- **The account's overview** (`.me`, `.play-card`, `.tiles`): who is
  signed in first (own picture or the name's first letter, display name
  with its tag, the sign-in name, when the session ends), then what needs
  attention, then for an administrator the administration's pages, then
  the game (the start page's download button for the reader's machine and
  the gateway list), then every part of the area as a tile that is one
  link: its sign over its name and one line on what is there, the sign
  beside the words on a phone.
- **The header's account link** (`.nav-account`, `boot.js`): signed out
  it says "Sign in"; signed in it shows the own picture, the display name
  and the tag where a page noted them (`docs/web-accounts.md`), otherwise
  "Your account". The name is cut with an ellipsis before the bar breaks,
  the tag never; a screen reader hears "Your account:" before the name.
- **The strip** (`.strip` in `header.css`, `pages/home.rs`): Baylee in
  six numbers under the bar of the start page, in the owner's order
  (18.11.2026): players, running games and open tables as the online
  gateways report them, the gateways online of all that are listed,
  Central's accounts, and the visits. A cell is a value, a glyph and a
  short label; a number nobody reported is no cell. A phone stands the
  cells three to a line, the value over its label; from 600 px they share
  one line evenly with a hairline between neighbours, and from 1100 px a
  cell is one line, glyph, value, label. The cards' numbers are not here;
  they have their section.
- **Dialogs** (`components/Modal.tsx`): the native `<dialog>`, centred,
  focus inside, Escape and the close button close it, focus returns to the
  opener (or to the main content when the opener is gone). A dialog the
  browser closes by itself while its owner keeps it (a secret shown once)
  opens again. A dangerous change names the thing it acts on and asks for the
  change in words on its button ("Revoke token"), never "OK".
- **The secret box** (`.secret`): a token, key or code shown once. Mono,
  selectable, a copy button, the warning "shown only now" above it, and the
  dialog does not close on a click outside until the person says they kept
  it. A secret is never put in a URL, the page's storage or a log.
- **States of a list**: loading is a skeleton of the list's shape
  (`.skeleton`) under `aria-busy`, empty is a sentence and the next step,
  failure is the reason and "Try again" (`Failed`). The three never look
  alike. A list says how many it shows; "More" focuses the first new row and
  takes no second press while it loads.
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
- **Hover** deepens a card's shadow and lights its edge; a button lightens.
  Nothing moves or changes size as a whole under the pointer (owner,
  11.11.2026). **Press** sinks a button into its own shade. A row of text
  does not grow.
- **Arriving**: a section rises 14 px and fades in once as it is first read
  (`useReveal`); what is on screen at load never waits. The cards of a grid
  follow each other by `--stagger`.
- **Between pages** the main column cross-fades in `--dur` through the View
  Transitions API where the browser has it; elsewhere the page simply
  changes.
- **Ambient**: the online dot breathes slowly (2.4 s); the painting drifts
  with the scroll; the hero's light and Baylee's cloud move slowly, and only
  while the hero is on screen and no dialog covers it. The hero's clouds
  (`js/clouds.js`) drift on a wind that comes in slow gusts and gather into
  their banks: cumulus, small puffs in front of them and thin streaks high
  behind, light in both colour schemes (sunlit by day, as bright under the
  blue hour's veil as the painted ones) with a soft shade of their own and a
  warm rim towards the sun. Their wisps turn over slowly in a calm sky, each
  finer layer drifting against the coarser. The pointer stirs the air they
  ride in, a small fluid on the GPU (stable fluids with vorticity
  confinement on a 112-cell-high grid, the clouds on a finer one): it drags
  the air along its path, a swift stroke more of it than a slow one and
  never faster than itself; stirred vapour thins, rises a little and is
  drawn out like smoke, and gathers again as the air forgets the stir, with
  nothing pulled back or popping in; a press sends a gust outwards. The air
  moves alike at every refresh rate: at most a hundred steps a second while
  stirred (a 120 Hz screen shows every second frame), thirty while calm,
  when the solve rests altogether. Thinner behind the words, fuller towards
  the edges, and under the veil that keeps the words readable; a still
  picture under reduced motion and where WebGL 2 renders in software, the
  painting alone without WebGL 2. The memorial's light
  is its own (limited to 20 draws a second, its GPU context given back when
  the album closes). Nothing else moves by itself.
- **One state giving way to another** (owner's second round, 11.11.2026),
  through the same View Transitions API and only where it exists: a new
  colour scheme spreads in a circle from its switch (560 ms), and the
  gateway directory's cards glide to their new places when a chip, the sort
  or the layout is pressed (360 ms; up to 48 gateways, more change at
  once); typing in the search does not wait for it. A menu comes in with its rows one after the other (260 ms) and
  leaves as it came (150 ms).
- **Baylee's album**: a page turns as paper does (920 ms): the leaf is cut
  into ten strips, each hinged on the one before, so it bends while its
  edge lifts and lies flat where it lands, with its shade on the page
  below; a photograph is laid on the one before it (460 ms). A finger's
  sideways stroke turns either.
- **For those who look**: typing "baylee" outside a field lets paw prints
  cross the foot of the window once.
- **Reduced motion** (`prefers-reduced-motion: reduce`): every transition,
  animation, drift and count-up stops, and content is shown at once
  (`base.css` §"stillness", `motion.ts`). This is WCAG 2.3.3 and the same
  promise as the client's *reduce motion*.

## Words

The words are Fluent messages, one file per language in
`crates/central-web/locales/<code>/site.ftl`; the build refuses a language
that lacks a message, a variable, a tag or a plural form the English has.
English and German are written by hand; French, Italian, Spanish, Brazilian
Portuguese, Russian, Japanese, Korean and both Chinese scripts are machine
translations: every such page says so in its footer, and a legal page adds
that the German and the English text apply where its translation differs.
The reader is *du* in German. A button says what it does, as a verb ("Create token",
"Revoke"), and a question asks in the words of the button that answers it.
An error says what happened and what to do next; it never blames, and it
never shows a server's diagnostic.

## Accessibility

WCAG 2.1 AA, mobile first:

- Landmarks (`header`, `nav`, `main`, `footer`), one `h1` per page, a skip
  link to `#main`, `document.title` per page.
- Every control works by keyboard, in reading order, with a visible focus
  ring (`--focus`, 3 px, offset 3 px). Menus close with Escape and return
  focus to their button, and close when focus or a tap goes elsewhere.
- Focus never falls to the page: when the element that had it goes away (a
  button that did its job, a list read again) a page moves it to the words
  that say what happened or to the next row, and the app frame catches the
  rest at the main content.
- Windows' high contrast (`forced-colors`) keeps a focus ring in every
  field and outlines the current page and a pressed choice.
- The root size follows the browser's own text setting (16 to 20 px at
  its usual size), and the page keeps clear of a phone's notch
  (`safe-area-inset`).
- Touch targets are at least 44 × 44 px.
- An asynchronous result is announced (`aria-live` / `<output>`), and a list
  that loads says it is busy (`aria-busy`).
- Text scales to 200 % without loss; nothing is wider than 320 px forces.

## Areas

| area | paths | frame |
| --- | --- | --- |
| Public | `/`, `/download`, `/cards`, `/gateways`, `/contact`, `/donate`, legal pages | the bar, sections, the footer |
| Account | `/account`, `/account/chat`, `/account/contacts`, `/account/security`, `/account/gateways` | the app frame; the messenger on chat and contacts |
| Administration | `/admin` (the overview), `/admin/feedback`, `/admin/feedback/<id>`, `/admin/users`, `/admin/users/<id>`, `/admin/gateways`, `/admin/audit`, `/admin/legacy` | the app frame, loaded only for an administrator |
| Sign-in | `/signin`, `/register`, `/reset` | a dialog over the page that asked; at its own address, over the start page |

The account area and the administration are loaded lazily and only after a
sign-in; a visitor's first page fetches neither.

An address the site had and took away for good answers 410, in every
language, with a page that says so and leads to the start page
(`Page::REMOVED`, `Page::Gone`). `/challenges` is one since 18.11.2026:
Daily and Weekly tasks, the Tower, their rankings and the consent to be
listed in them are shown by the game client, and the website shows none of
it. A page whose content lives on elsewhere on the site is a permanent
redirect instead (`/cards` to `/#cards`).

The three forms (signing in, registering, a new password with a reset code)
are dialogs, never pages of their own. Every page outside the account area
carries them closed; a link to one opens it and puts the form's address in
the bar, closing it goes that step back, and Back closes it. Entered
directly (or reached from a reset mail, whose code rides in the fragment),
the address shows the start page with the dialog open, and closing it
leaves for the start page. Without script the address shows the start page
under a line that says the form needs script. The addresses stay out of the
index and the sitemap. Leaving a form for good (signed in, declined) takes
its entry's place in the history, so Back does not open it again.

A form can be followed by a step in the same dialog: the recovery code of a
new account (or of a reset), and Central's terms where the current version
is not accepted yet. A step is the dialog's whole matter (`.step`): the
dialog widens to 46rem, its own heading and line make way for the step's,
which is centred, and the step ends in `.step-actions`, the one way on as
wide as the step with its glyph, or two answers of equal width.

- **The code step** says first that the account exists and names it, the
  username in the title's ink and its tag in gold (`.who`). The code stands
  in a dashed gold frame and breaks at its hyphens into lines of equal
  length. The ways to keep it (`.keep-ways`, `islands/keep.rs`) are tiles
  of one size, each a glyph over its words: two in a row on a phone, all in
  one row from 600 px. Mailing it to oneself is the last of them and no more
  than a `mailto:` the person's own mail program opens; its warning (the
  code then lies in a mailbox) unfolds under the row when the tile is
  pressed or an address is typed (`.unfold`: it opens downward and fades
  in, stands at once under reduced motion, and is not there for a reader or
  the keyboard while shut), and the line under the tiles says that the mail
  program was asked.
- **The terms step** shows the text in a box that scrolls, with a line
  under it that fills as the reader goes down and turns to `--ok` at the
  end. Accepting waits for the end of the text, as it does in the game
  client: until then the button is `aria-disabled` (it keeps its place in
  the tab order and its sentence, `aria-describedby`), and a press makes
  that sentence speak up instead. A text that fits its box without
  scrolling counts as read. Declining is the quiet button of the two.

### The donation page

`/donate` asks for help with what Baylee costs each month, and it is the
one page that does. Baylee's `docs/legal.md` ("Donations") binds every
word of it: a donation buys nothing, no donor is named, Wizards is not put
beside a request for money, and the page says that a donation is a gift to
a private person and not tax-deductible.

- **The goal** (`.goal-card`) is the amount in the title face, the bar
  (`.give-meter`: the browser's `<progress>`, since the policy allows no
  inline width) and one line on what the figure counts. Only what a
  platform said is drawn: without a figure the bar is left out and a line
  says so, and where nobody gives yet the line invites the first instead of
  counting nobody. Beside it stands what a month costs, each amount a
  thinner bar against the goal (`.cost-bar`). The card's button leads down
  to the three places.
- **The three places** (`.way`) are cards of one build: the platform's
  name with a sign, one line on what kind of giving it is, its address as a
  QR code, the address as text, and the button. The code stands on a white
  tile in both schemes (a camera wants dark on light) and is a file made by
  Baylee's `cargo tools site qr`, not drawn by script; below 30rem it is
  left out, since a phone does not photograph its own screen. Every link
  to a platform opens a new tab with `noopener noreferrer`.
- **A donation button's light.** A button that leads to giving (`.give`,
  and the bar's `.nav-give`) has the shape, size and place of its
  neighbours and catches the eye by light alone: every six seconds a sheen
  crosses it and its heart glows. Nothing moves or changes size, and under
  reduced motion the button is still.
- **Where it is linked.** The bar's last entry before the account's, a
  heart before its word in a soft frame (the frame replaces the bar's
  underline); the footer's project column; and one card on the start page
  after the open-source section (`.give-band-card`): a heart, a sentence,
  the amount and the button. The site asks nowhere else.
- **`/funding.json`** says the same to a program, the game client first:
  the goal in euro cents, what is covered, the percentage and the number
  of supporters where a platform answered, which platforms the figure
  counts, the page's address and the three places. It names nobody.

### Going from page to page

Every page is a complete document at its own address, and that is what a
first visit, a search engine and a browser without script get. Between
pages of the public site `js/nav.js` fetches the next page (the same
request a navigation makes) and puts its part of the document in place of
the one on screen: the title, the head's words about that page, and the
body. The style sheet, the fonts, the scripts and the WebAssembly stay, the
bar does not blink, and Back and Forward bring a page back where it stood.
A link is fetched a moment before its press (the pointer over it, the
keyboard on it, a finger down), not at all where the device asks to save
data; a page that still takes longer than a blink shows a line running
under the bar. After a swap the keyboard is on `<main>` and a status line
says the page's title.

The document is loaded anew, exactly as before, when the link leaves the
language, enters or leaves the account area, when an island on screen was
taken over (its state ends with its document), and when the response is
not a page of the build on screen, not HTML, or failed. Structured data is
not carried over: it is for readers of the server's document.

The scripts of a page follow one contract: `baylee:leave` goes out before a
page is taken away (stop loops, let go of what hangs on the document),
`baylee:page` once the next one stands (start over on its nodes). A script
that binds only to a page's own nodes needs the second alone.

What makes this safe (`docs/security-assurance.md`, the three passes): the
response is parsed into a document that runs nothing, and its nodes are
put in place as nodes; a script that came with it does not run, a page's
own scripts are made here from their address and integrity value, and the
address must be one of the site's hashed scripts (`/assets/<name>.js`).
The Content Security Policy holds for what is put in place as for any
page: no inline script, no inline handler, no foreign origin. Trusted
Types allow one more policy for this, `baylee-nav`, which passes a string
to the parser for that one use and vouches for no other script address.
What the strip counts is a visit (`central-web-server`, `views.rs`): a document sent to a
navigation that did not start on one of the site's own pages and is not a reload. Going from page to page is never counted again, whether the page is put in place here or loads a document (another language, the account area).

### The gateway directory

Every gateway Central lists (`pages/info.rs`, the card in `pages/home.rs`,
`style/gateways.css`, `js/directory.js`), and its first four on the start
page in the same look. It shows what Central has and derives the rest from
it; nothing is made up, and a number nobody reported is never a zero.

- **In numbers.** Four equal tiles above the list: the gateways online of
  those listed ("5 / 7"), the players online, the games running and the
  open tables, added up from what the online gateways report. Four in a
  row, two and two on a phone; while no gateway reports, the first alone in
  the middle.
- **A gateway.** A card: a mark (the first letter of its name on one of the
  six avatar tints, chosen by its address), its name, its host under a lock
  (the address without `https://`; the copy button gives it whole), and
  whether it is online, as a pill with the word. Then the four counts it
  reports, each a glyph, a figure and its word; the share of its players
  in a game as a bar and a sentence (the bar is drawn in twentieths from
  `data-load`: a page carries no inline style); its version, with the pill
  "Current" when it is exactly the newest release's and no word when it is
  not (a gateway may run a newer build, and the suffixes do not order).
  One that reports nothing, or is offline, has a line in words in place of
  the figures. An offline gateway is flat, without the card's light and its
  mark's tint, and never dimmed: its words keep their contrast.
- **What to do.** "Open in Baylee" (gold, the same step on every card) is
  the client's link `baylee://gateway/add?origin=…`, offered only while
  the gateway is online; beside it two round buttons, copying the address
  and the gateway's website. A line under the list says what the link does
  and where the client is to be had.
- **The order.** Online before offline, then Central's own order, then the
  busier first; this is the order without script and the one called
  "Recommended". No gateway is called official (owner, 12.11.2026).
- **The bar.** One search field (name or address), the order, the layout,
  and under them the chips with their counts and how many are shown. The
  chips narrow together: "Online", "With open tables" and "Current
  version" pressed at once show what is all three; "All" lets go of them.
  From 860 px the bar stays in sight under the page's own. What is chosen
  stands in the address (`?q= &show= &sort= &view=`). Nothing found is a
  panel with the sentence and "Show all".
- **The list.** From 1100 px the layout can be a list: one panel, a
  gateway a row, the same parts in columns under a head that names them;
  the head's sortable columns are buttons that sort (pressed again: the
  recommended order), the one in use in gold. It is a list restyled, not a
  table, and a figure keeps its word for a screen reader. Below 1100 px
  the cards stay cards. The start page shows its four as such rows from
  1100 px, without the sorting, and as shorter cards below (three on a
  phone).

### The messenger

The chat and the contacts (`src/chat/`, `styles/chat.css`, [chat](central-chat.md))
are one window, as in Baylee's client and any messenger, not panels of
settings:

- **Two panes.** From 760 px the conversations and contacts stand beside the
  open conversation, the window as tall as the screen allows; below, one pane
  at a time with a way back, and the page's heading only for screen readers.
- **The side.** Who is signed in (an avatar of initials, six tints mixed from
  the palette, the same for the same person everywhere), a new group and a new
  contact as icons, and two tabs: *Chats* (group invitations, then every
  conversation by its last line, its time and a count of what arrived unread,
  newest first, with a search) and *Contacts* (adding by the exact handle,
  requests to answer, friends who open their conversation, requests sent, the
  blocked folded). Rare actions sit in a row's menu (⋮), never as a row of
  buttons; whatever cannot be taken back asks first.
- **The conversation.** A head with the avatar, the name and "End-to-end
  encrypted" (a group's members), the safety number behind a shield that turns
  to a warning when keys changed, a group's settings sliding in from the side;
  the messages as bubbles, the own on the right in `--gold`, by day with a
  divider, a run of one sender's lines closer together (a new sender, a new
  day or five quiet minutes start another), the time in each bubble's corner,
  and in a group the sender's face and name once per run; the composer grows
  with its text, Enter sends and Shift+Enter starts a line (a phone's keyboard
  keeps Enter for the line), the send button round.
- **Empty and waiting.** The list says it is loading until the contacts, the
  groups and the inbox's first read are in, and says that it failed, with "Try
  again", rather than look empty. An empty place says so in one line in the
  site's voice and offers the way on: no conversation, "Add a contact"; no
  contact, "Copy your handle"; no message yet, "Write the first message".
- **Locked.** A sign-in unlocks the chat; only after a reload the conversation
  pane asks for the password once, while the contacts and groups stay usable.

### The feedback section

Bug and crash reports (`src/admin/feedback/`, `styles/feedback.css`,
[reports](reports.md) §"The website's section") follow the components
above, with these of their own:

- **The list.** The views (inbox, open, mine, unassigned, done, all, trash)
  are a row of chips with their counts; the search and every filter stand
  in the address, the filters beyond the view as pills that take themselves
  out ("Clear filters" once there are two). A report's first line is the
  row's name, its labels, notes, record and issue under it; the columns
  sort with `aria-sort`. An unread report has a dot, the word "Unread" for a
  screen reader, and a heavier line. On a phone the first line wraps and the
  rest of the row scrolls in the table's box.
- **The drawer.** From 1100 px a report opens beside the list and the list
  keeps the columns that matter (the report, when it came, its status);
  below, a report opens on its own page, which carries the list's query, so
  the way back and `k`/`j` go through the same list.
- **What happened.** Every change says so in a notice at the foot of the
  screen (its words in a live region of their own, so its buttons are not
  read out), the trash's with "Undo" for ten seconds; a failure stays until
  it is dismissed. New reports found by the 30-second refresh are announced
  above the list and shown on a press: no row moves under the reader.
- **Keys.** `?` lists them. They act only when no field has the keyboard,
  and one switch, kept by the browser, turns every single key off (WCAG
  2.1.4); Ctrl+K and the buttons always work.
- **The 30 days.** Folded until asked for: a column a day, the crashes
  (`--chart-crashes`) stacked on the other reports (`--chart-reports`) with
  a gap of the surface between them, a legend in words with the totals, each
  day's numbers on hover and in a table. The two tokens are checked for
  colour vision against the panel in both schemes; in forced colours they
  become the system's text and highlight.
- **What the client sent.** The screenshot fills the column and opens large
  in a dialog; a card the text names is a chip whose picture shows under
  the pointer or the keyboard; the rest folds (the log, the system, the
  settings, the whole object as a tree drawn only as it is unfolded).

## Scrollbars, text smoothing and the download

- **One scrollbar everywhere** (`base.css`): the root sets `scrollbar-color`
  (it inherits) and `*` sets `scrollbar-width: thin`, so the page and every
  box that scrolls, listed or not, wears a thin bar in the page's ink on no
  track; Safari before `scrollbar-color` gets the same by `::-webkit-scrollbar`
  under `@supports not`. A component that needs its own bar sets its own.
  A new scrolling box needs no rule.
- **Text is antialiased** (`-webkit-font-smoothing`, `-moz-osx-font-smoothing`,
  `text-rendering`) on `:root`; form controls and placeholders say `inherit`.
- **A link in the download line carries no underline** (a text-shadow was
  drawing under the transparent one); hover and focus tint behind the words.
- **The download page** shows three system cards (the visitor's first, in
  gold), an architecture in each with its files, checksums and signatures,
  then, apart, the `x86-64-v2` archive for processors older than about
  2013-2015, with one sentence on who needs it; the steps of a first start per
  system; and what has no download yet (Android, iOS, browser). The menu of the
  download button shows the same variants, the older-processor archive a step
  in under its main build.

## How it is held

- `test/styles/contrast.test.ts`: the token pairs above in both schemes.
- `test/csp.test.ts`: the policy names GitHub only at Baylee's repository,
  takes no `data:` picture (`blob:` only for one the page made from bytes it
  checked: a report's screenshot) and switches off unused browser features.
- `test/styles/rules.test.ts`: no inline script or style in the shell, card
  pictures untouched, no raw colour outside `tokens.css`, a reduced-motion
  rule beside every animation.
- `e2e/`, under the production CSP at a phone's and a desktop's width:
  no sideways scroll on the main pages and on every page of the account
  area and the administration, signed in (`area.spec.ts`, against
  `e2e/area.ts`) (`expectNoSideScroll`; the memorial
  at 320 px too), a gateway card's controls 44 px high with copying its
  address as the first action while gateway links are off
  (`VITE_DEEP_LINKS`, the old `web/README.md` §"Settings", removed 12.11.2026), and the reduced-motion
  preference hiding nothing.
- `npm run screens`: every page at phone and desktop width in both schemes,
  the account area and the administration signed in as well, for a person
  to look at before a change lands.
