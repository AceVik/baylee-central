# Legal texts of Baylee Central

**Status: the terms are in force at version 2026-10-22, the privacy policy at 2026-10-23** (display name and tag, 23.10.2026). The owner published the drafts on
21.10.2026 with registration on `baylee.app`, after only the statements the
deployed service did not yet bear out were corrected: deletion is on request
until it is in the settings, there is no password reset or e-mail yet, and the
recovery code wraps the key container. What the
[roadmap](../docs/roadmap-accounts-web.md) still adds (e-mail, 2FA, display
name, profile picture, bug reports in Central) is marked as to follow. The
owner's full review, and ideally a lawyer's, are still to come; each change is
a new version.

| file | what |
| --- | --- |
| `terms.de.md` | Nutzungsbedingungen (binding) |
| `terms.en.md` | terms of use (translation) |
| `privacy.de.md` | Datenschutzerklärung (binding) |
| `privacy.en.md` | privacy policy (translation) |

## Rules for these files

- German is binding; the English file is a translation and says so.
- Each file starts with `<!-- version: … -->` and `<!-- updated: … -->`.
  Both languages of one document carry the same version. The version `draft`
  means not in force, and it is the only thing that holds a text out of
  force (`docs/terms.md`): publishing replaces it by a real version in a
  reviewed commit. A new version of the terms needs every account's
  acceptance again (accept, later = signed out, or delete the account);
  nobody continues under an older one. The privacy policy is information,
  not a contract: a new version is shown, not accepted.
- The text uses only the Markdown subset the Baylee client renders: `#`/`##`
  headings, paragraphs, `**bold**`, `*italic*`, lists, rules and links. No
  tables, no HTML, no images.
- The links name the website's pages: `https://baylee.app/impressum`,
  `/agb` and `/datenschutz` (German), `/terms` and `/privacy` (English).
  The Impressum's data is the operator's and never in this repository.
- What a statement claims has to be true of the deployed service: retention
  times, recipients and legal bases are checked against the code and the
  server before a version is published (for example the 30-minute session
  limit in `central-migration`, the log retention on the host).

Each gateway has terms of its own, edited in its admin area. The Baylee
repository ships a default text and privacy notice any operator may adapt.
