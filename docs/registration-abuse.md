# Abuse protection for registration

*Proposal for the owner, 28.10.2026. Not built.* The owner made strong
captcha and spam protection for registration an open-beta blocker
(28.10.2026): privacy-friendly, no Google reCAPTCHA, the design first. A new
dependency or a third-party service needs the owner's approval; this
proposal needs neither.

## What exists

- A beta key per registration; each key has a number of uses, an expiry and
  can be revoked (`docs/web-accounts.md`).
- Per-address budgets on the browser listener, from the proxy's last
  `X-Forwarded-For` hop, IPv6 counted by its /64: `Register` 5 an hour,
  `Discover` and `LoginStart` 30 a minute each, `ResetStart` 10 an hour; in
  memory, at most 4096 addresses, the oldest window making room.
- Global budgets (32 unauthenticated requests a second) and the per-account
  sign-in budget (eight OPRF evaluations a minute).
- Registration is the browser listener's only; the native client cannot
  register yet (M6).

What is missing for an open beta: anything that costs a bot more than a
human, a cap on the whole, and a cap per key.

## Proposal

1. **A proof of work, issued and checked by Central** (ALTCHA-style, no
   service, no new dependency). `RegisterChallenge` (browser listener, its
   own per-address budget) answers `{ challenge, difficulty, expires_at }`:
   32 random bytes Central keeps in memory (bounded, one use, five
   minutes). The page finds a counter whose SHA-256 over challenge and
   counter has `difficulty` leading zero bits, in a Worker with the
   browser's own WebCrypto, and sends `challenge` and `counter` with
   `Register`. Central checks it before the beta key, spends it either way,
   and refuses a solution found less than three seconds after the
   challenge (the minimum fill time). The difficulty starts at about two
   seconds on a mid-range phone and rises automatically while registrations
   arrive faster than a set rate. Nothing about the person is processed:
   the challenge is random. The native client (M6) solves it in Rust.
   *Considered and set aside:* Friendly Captcha (an EU service, but a
   processor needing an agreement, and a third-party script on the page);
   the ALTCHA widget (a dependency for what is a few dozen lines here);
   reCAPTCHA and hCaptcha (trackers, excluded by the owner).
2. **Caps.** Per address: today's 5 an hour, plus 10 a day. Per beta key:
   at most N registrations an hour, so a leaked multi-use key cannot fill
   the service at once (needs the registration time per key, a small
   migration). Over all registrations: a ceiling an hour, above which
   `Register` answers `Busy` and the operator is told in the log (counts
   only, no addresses).
3. **A honeypot field** in the page's form, never sent when empty, and the
   page refusing to send a form filled in under three seconds. This only
   stops generic form bots; the proof of work is what binds a bot that
   speaks the API directly.
4. **Later, with e-mail (step 4):** e-mail is optional at registration, so
   disposable-address signals matter only for verification and 2FA by
   e-mail; a list would be a dependency and is left for then.

## Privacy

The proof of work processes no personal data. The per-address counters stay
in memory as today (the privacy notice already names them). A per-key count
is about keys, not people. Nothing new for the notice unless the owner picks
the third-party fallback, which would need an agreement and a new section.

## Questions for the owner

1. Does the open beta keep beta keys (widely shared, multi-use), or open
   registration without a key? Without keys, the proof of work and the caps
   are the only gate, and the numbers above should be lower.
2. The numbers: per-key and global ceilings (suggested: 20 an hour per key,
   200 an hour in all), and the starting difficulty.
3. Approve the self-hosted proof of work (recommended), or prefer the EU
   service despite the processor and the script?
