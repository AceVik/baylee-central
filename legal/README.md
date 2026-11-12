# Legal texts of Baylee Central

**Status: the terms and the privacy policy are at version 2026-11-11** (the
owner asked for the texts to be corrected on 11.11.2026: an administrator's
reset code, deleting the account in its settings on the website, the Fan
Content notice in its given words, the chat's and the notifications' legal
bases, and the push services as recipients outside the EEA; before that the
terms at 2026-10-22 and the privacy policy at 2026-11-02). Being a new
version of the terms, every account is asked to accept it again. What is
still the owner's to decide or confirm stands in
[the text review](../docs/proposals/website-text-review.md). The owner published the drafts on
21.10.2026 with registration on `baylee.app`, after only the statements the
deployed service did not yet bear out were corrected: deletion is on request
until it is in the settings, there is no password reset or e-mail yet, and the
recovery code wraps the key container. What the
[roadmap](../docs/roadmap-accounts-web.md) still adds (e-mail, 2FA, display
name, profile picture, bug reports in Central) is marked as to follow. The
owner's full review, and ideally a lawyer's, are still to come; each change is
a new version.

On 12.11.2026 the owner had e-mail built (an optional address, a reset
code by mail) and decided that version 2026-11-11, not yet published,
carries it: the terms' two sentences on e-mail and the privacy policy's
e-mail lines changed (`updated: 2026-11-12`), the version stayed. Nobody
outside local tests has accepted 2026-11-11 before the change. The same
day the privacy policy's e-mail line gained the notices to a confirmed
address (a reset finished, the password changed, an administrator's
reset code), which Central sends since then; the version stayed.

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
