# Privacy, training and website requirements

Planning record, 2026-10-19. This is not a legal compliance certification.

## Data classification

| Class | Examples | Handling |
| --- | --- | --- |
| Client-only secrets | Password, master key, recovery code | Never sent to central; documented legacy migration exception for password |
| Private ciphertext | Decks, replays, settings, history | Account/device authorization, retention and deletion |
| Personal metadata | Account, contact edges, devices, IP/security logs | Purpose and legal basis, restricted access and bounded retention |
| Training candidates | Full rule events, decks, decisions, RNG, hidden states | Treat as personal until anonymity is established |
| Approved training data | Validated minimized copies | Separate storage/workers and dataset lifecycle |
| Public information | Approved listings and explicit public shares | Separate publication decision and rights review |

Contact routing exposes relationship metadata; do not call it zero-knowledge.
The game host needs selected deck/game state to execute play even if saved decks
are encrypted centrally. A training export is separate from the private replay.

## Training pipeline

1. Determine export eligibility under the approved legal basis and user choices.
2. Produce an allowlisted rule-level candidate after the game, with engine/rules
   version, all state needed for replay, decisions and reproducible randomness.
3. Omit chat/free text, names, account/device IDs, IPs, absolute timestamps and
   personal replay identifiers. Use per-game seat numbers, not stable player IDs.
4. Validate in bounded quarantine, independently of authentication infrastructure.
   Store each acting player's actual information set separately from omniscient
   truth so training cannot accidentally teach hidden-information cheating.
5. Evaluate re-identification, including linkage to public streams and replay
   sequences. Removing names or hashing IDs is not proof of anonymity.
6. Only admit eligible validated records to versioned datasets and track dataset
   and model lineage. No training job reads private account vaults.

Do not promise both perfect anonymity and indefinite per-user dataset tracking.
Before anonymization, authorization/withdrawal needs restricted traceability.
After verified irreversible anonymization, account-specific retrieval may no
longer be possible. Define this boundary, retention periods and honest UI wording
before release. Logs, upload timing, object names and credentials are part of
the linkage analysis; do not retain hidden mappings and claim anonymity.

Full unique trajectories may remain linkable. If necessary, reduce information
or process them under a separately justified legal basis; do not degrade training
silently or relabel pseudonymous data. Human replay ingestion stays disabled until
the legal and technical gates pass. Synthetic/self-play datasets can support QA
and early training without repurposing real-user replays.

## Accepted preference interaction

- Place the persistent setting under Settings > Privacy > AI training, clearly
  discoverable with a concise explanation; do not deliberately hide it.
- Default-on is conditional on demonstrated anonymity and a reviewed lawful
  basis for all earlier personal processing. A preselected control is never
  recorded as GDPR consent. Pending that review, human automatic intake is off.
- Explicit opt-out persists across devices and blocks automatic contribution.
- Each result screen may offer an unchecked, non-modal action: "Share only this
  game for AI training". No recurring interrupting consent popup.
- A one-game grant does not turn the global setting back on. Permanent re-enabling
  is a separate deliberate action. No retroactive blanket upload of old replays.
- Under the initial consent-based route, all human participants must authorize
  export; one player's action cannot authorize another player's personal data.
- Enforce preferences and one-game grants at the intake boundary, not only the
  UI. Specify races, offline clients and queued uploads: revocation prevents
  not-yet-authorized submissions; already processed data follows the applicable
  legal lifecycle, never a false promise of automatic model unlearning.

## Legal and operational release checklist

Name the actual controller and contact; map roles of independent gateways and
processors rather than assuming they are all processors. Document purposes,
legal bases, records of processing, retention, deletion/export/access procedures,
processor agreements, international transfers and breach response. Assess whether
a DPIA is required, including minors and training risks. EU hosting alone does
not establish GDPR compliance. Obtain qualified review before human-data intake.

Define treatment of erasure/withdrawal across raw candidates, datasets, backups
and models; retraining may be necessary. Record model lineage without secretly
preserving personal links in data declared anonymous. Set actual retention values
before launch, including logs, quarantine and backups; no indefinite default.

Website includes privacy information, required operator/legal notices, accessible
controls and unofficial-project notices. Public fan content is accessible without
email registration. Do not assume fan-content permission grants AI training or
dataset redistribution rights. Review Wizards policy, Scryfall terms and asset
licenses separately; ship no protected assets in this preparation repository.

## Primary references

- [GDPR](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A32016R0679),
  especially recital 26 and Articles 5–7, 12–22, 25, 28, 32–35 and 44 onward.
- [EDPB lawful processing](https://www.edpb.europa.eu/sme/be-compliant/process-personal-data-lawfully_en):
  "For consent to be unambiguous, there should be a clear affirmative action"
  (checked 2026-10-19). Pre-ticked boxes are not valid consent.
- [EDPB Opinion 28/2024 overview](https://www.edpb.europa.eu/news/edpb-opinion-on-ai-models-gdpr-principles-support-responsible-ai_en):
  model anonymity and legitimate interests require case-specific assessment.
- [Wizards Fan Content Policy](https://company.wizards.com/en/legal/fancontentpolicy):
  "You can’t require payments, surveys, downloads, subscriptions, or email registration to access your Fan Content;"
  (checked 2026-10-19). No blanket AI-training permission inferred.
