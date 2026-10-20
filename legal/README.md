# Legal texts of Baylee Central

**Status: drafts for the owner's review, not in force.** A lawyer's review is
recommended before publication. They describe the service as it will be at
public sign-up (milestones M1 to M7 of the
[roadmap](../docs/roadmap-accounts-web.md)): registration, recovery code,
optional e-mail, 2FA, encrypted backups, the gateway directory and bug
reports in Central. Until those exist, these texts are not served.

| file | what |
| --- | --- |
| `terms.de.md` | Nutzungsbedingungen (binding) |
| `terms.en.md` | terms of use (translation) |
| `privacy.de.md` | Datenschutzerklärung (binding) |
| `privacy.en.md` | privacy policy (translation) |

## Rules for these files

- German is binding; the English file is a translation and says so.
- Each file starts with `<!-- version: … -->` and `<!-- updated: … -->`.
  Both languages of one document carry the same version. A new version of
  the terms needs every account's acceptance again (accept, later = signed
  out, or delete the account); nobody continues under an older one. The
  privacy policy is information, not a contract: a new version is shown, not
  accepted. The version `2026-10-1` is a placeholder until publication.
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
