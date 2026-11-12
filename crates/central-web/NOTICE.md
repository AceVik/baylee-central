# Notices: what the website ships besides its own code

Every file the site serves comes from this repository or is built from it;
nothing is loaded from another origin. The built site serves the full texts
as `/notices.txt` (written by `cargo xtask site`), which the footer links.

| What | Where | Licence | Notes |
| --- | --- | --- | --- |
| Inter (variable 100–900) | `assets/fonts/Inter-*.woff2` | SIL OFL 1.1, `assets/fonts/OFL-Inter.txt` | © 2016 The Inter Project Authors; the Latin, Latin Extended, Cyrillic and Cyrillic Extended subsets of `@fontsource-variable/inter` 5.3.0, unchanged |
| Literata (variable 200–900, roman and italic) | `assets/fonts/Literata-*.woff2` | SIL OFL 1.1, `assets/fonts/OFL-Literata.txt` | © 2017 The Literata Project Authors; the same subsets of `@fontsource-variable/literata` 5.3.0, unchanged |
| Lucide icons (`lucide-static` 1.60.0) | `src/site/icons.rs`, drawn inline | ISC, `assets/LICENSE-lucide.txt` (parts derived from Feather, MIT) | © Lucide Icons and Contributors |
| The marks of Windows, Apple and Linux (Font Awesome Free 7.3.1, brands) | `src/solid.rs`, drawn inline | CC BY 4.0, `assets/LICENSE-fontawesome.txt` | © Fonticons, Inc.; the paths unchanged |
| The world painting, the Baylee logo, the cloud portrait, the icon | `assets/img/world-*`, `logo-*`, `cloud-*`, `icon*` | the project's own | owner-approved; provenance in `web/assets-sunlight.md` (format and size conversions only) |
| Baylee's photographs | `assets/img/memorial/` | the owner's | family-approved, exact copies of the existing derivatives |
| The sharing image | `assets/img/og.jpg` | the project's own | a crop of the world painting |
| The islands' crates (Leptos and its dependencies, `central-api`) | in `islands.wasm` and `admin-islands.wasm` | MIT, Apache-2.0, BSL-1.0, CC0-1.0, Zlib and the AGPL-3.0-only of `central-api`, each listed with its text in `/notices.txt` | owner-approved graph: `docs/dependency-proposal/leptos.md` |

Not in this repository and never served from it: Wizards of the Coast's
material. The client's screenshots, which show Scryfall's card images
(© Wizards of the Coast), are copied in at deployment from a Baylee checkout
(`BAYLEE_SRC`) and credited beside them.
