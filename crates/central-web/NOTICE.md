# Notices: what the website ships besides its own code

Every file the site serves comes from this repository or is built from it;
nothing is loaded from another origin. The built site serves the full texts
as `/notices.txt` (written by `cargo xtask site`), which the footer links.

| What | Where | Licence | Notes |
| --- | --- | --- | --- |
| Alegreya Sans (400, 500, 700) | `assets/fonts/AlegreyaSans-*.woff2` | SIL OFL 1.1, `assets/fonts/OFL-AlegreyaSans.txt` | © 2013 The Alegreya Sans Project Authors; the game's files, subset on 10.11.2026 to Latin-1 and the punctuation used (no Reserved Font Name) |
| Faustina (variable 300–800, roman and italic) | `assets/fonts/Faustina*.woff2` | SIL OFL 1.1, `assets/fonts/OFL-Faustina.txt` | © 2016 The Faustina Project Authors; subset as above |
| Lucide icons (`lucide-static` 1.60.0) | `src/site/icons.rs`, drawn inline | ISC, `assets/LICENSE-lucide.txt` (parts derived from Feather, MIT) | © Lucide Icons and Contributors |
| The world painting, the Baylee logo, the cloud portrait, the icon | `assets/img/world-*`, `logo-*`, `cloud-*`, `icon*` | the project's own | owner-approved; provenance in `web/assets-sunlight.md` (format and size conversions only) |
| Baylee's photographs | `assets/img/memorial/` | the owner's | family-approved, exact copies of the existing derivatives |
| The sharing image | `assets/img/og.jpg` | the project's own | a crop of the world painting |
| The islands' crates (Leptos and its dependencies, `central-api`) | in `islands.wasm` | MIT, Apache-2.0, BSL-1.0, CC0-1.0, Zlib and the AGPL-3.0-only of `central-api`, each listed with its text in `/notices.txt` | owner-approved graph: `docs/dependency-proposal/leptos.md` |

Not in this repository and never served from it: Wizards of the Coast's
material. The client's screenshots, which show Scryfall's card images
(© Wizards of the Coast), are copied in at deployment from a Baylee checkout
(`BAYLEE_SRC`) and credited beside them.
