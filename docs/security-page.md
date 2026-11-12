# The security page: what it claims and where each claim comes from

`/{lang}/security` (`crates/central-web/src/site/pages/security.rs`, its
words at the end of `crates/central-web/locales/{en,de}/site.ftl`) explains
four flows to a visitor in diagrams: signing in, the password and the keys in
memory, a chat message, and signing the client in from the website by a link.

A public page about security is a list of promises unless every sentence is
checked. This file is that check: **every sentence and every arrow of the
page maps to a row below. A claim changes here first, then on the page.**
Paths are in this repository unless they start with `baylee:` (the game
repository). "Browser" is this website, "client" is Baylee's native client.
The rows were checked by reading the named sources on 2026-11-11, and then a
second time by a reader who had not written them and was told to assume every
sentence false until the code showed it: that pass changed A7, A8, B2, B5 to
B7, C1 to C7, D9, E2, E5 and E7 and the sentences that stand on them. Nothing
here is an audit.

## Rules for the page's words

- Mechanisms, not adjectives: no "sicher", "secure", "safe", "unbreakable",
  no words about legal compliance. `security.rs`' test `no_promises` refuses
  them in both languages.
- "Never" and "only" stand beside what bounds them: the password sentence
  beside the legacy exception (A8), "end to end" only as the mechanism (C2 to
  C4) beside what Central still sees and what the format lacks (C5 to C8).
- Every flow ends in its limits, and the page ends in a list of what none of
  it can do (E1 to E7).
- The browser and the client differ in what stays in memory (B3, B7, B10).
  The page says so instead of one sentence for both.

## 1. Signing in and registering (`sec-signin-*`)

| # | Claim on the page | Source |
|---|---|---|
| A1 | Sign-in and registration use OPAQUE (RFC 9807; opaque-ke, Ristretto255 OPRF, 3DH, SHA-512); the server receives protocol messages, not the password and no plain hash of it | docs/crypto-protocol.md:9-11; crates/central-crypto/src/opaque.rs:34-39 (Suite), :81-93, :157-168; crates/central-web/src/islands/auth.rs:3-4 |
| A2 | The password is stretched on the device with Argon2id, 64 MiB, three passes; the server cannot choose other settings | docs/crypto-protocol.md:10-11; crates/central-crypto/src/opaque/stretch.rs:14-17; opaque.rs:17 (the transcript names the profile) |
| A3 | Browser: the first message is 96 bytes (KE1), a wrong password fails on the device before the proof is sent, the proof is 64 bytes (KE3) | crates/central-web/src/islands/crypto.rs:1-5; auth.rs:131-150; crates/central-web-crypto/src/flow.rs:146-163 |
| A4 | Central answers a proven sign-in with the wrapped master key and sets the session cookie; the page cannot read the session (`__Host-central`, Secure, HttpOnly, SameSite=Strict) | auth.rs:151-163; docs/web-accounts.md:52-55; crates/central-http/src/browser.rs:1156 |
| A5 | Registration: the device draws a random 256-bit master key and wraps it once under a key derived from OPAQUE's export key and once under a recovery code; Central stores the OPAQUE record and both wrappers, never the key | flow.rs:86-105; docs/crypto-protocol.md:42-47; docs/web-accounts.md:39-46 |
| A6 | The wrapping key comes from OPAQUE's export key, which the server does not learn; so the wrapper "opens only on your device, with the right password" | opaque.rs:194-197, :67-73, :147-148; docs/crypto-protocol.md:42-45 |
| A7 | Limit: whoever takes Central's database and keys can guess passwords; each guess costs one Argon2id run; a password found that way opens the account's encrypted data and its stored chats (C6); a weak password stays a risk | docs/threat-model.md:10-12; crates/central-crypto/src/vault/derived.rs:72-90 |
| A8 | **Exception** (without it "the password is in none of the messages" would be false for some accounts): accounts of the old official gateway that have not moved yet. The client, not the website, sends the typed password at each sign-in attempt until the account has moved (also a mistyped one, and a retry after a failed enrolment), over verified TLS, to Central; Central checks it against the old hash and neither stores nor logs it (a type without `Debug`, `Display` or `Clone`, wiped when dropped; the handler logs nothing); once moved, the account uses OPAQUE; the exception ends at the deadline set for the move | docs/native-login.md:67-74, :151; crates/central-api/src/legacy.rs:18-23; crates/central-identity/src/legacy.rs:29-63; crates/central-crypto/src/legacy.rs:1-10; docs/threat-model.md:27; baylee:crates/baylee-account/src/account.rs:54-58 |
| A9 | The website refuses a not-yet-moved account instead of sending its password | auth.rs:129 (`Discovery::Legacy => return Err(Problem::NotMoved)`) |
| A10 | No game gateway receives a password (not drawn; the page names only the device and Central) | docs/native-login.md:5-6, :19-21 |
| A11 | Step 1 "blinds" the password: OPAQUE's OPRF combines it with a fresh random scalar, and the server computes on the blinded element with its own key | RFC 9807 §3 and §4 (OPRF); opaque.rs:34-39 (`OprfCs = Ristretto255`) |
| A12 | Guessing through the sign-in is slowed, not stopped: attempts that are not completed with the right password are counted, at most 100 a day on one account from all addresses it has not signed in from, 30 a day from each it has, and at most 100 a day from one address on all accounts together; past that Central answers `Busy` (the page's "too many attempts") until the hour or the day ends. A common password is still found | docs/native-login.md §"Guessing a password online"; crates/central-identity/src/guesses.rs (the `ACCOUNT_PER_*`, `PAIR_PER_*`, `PLACE_PER_*` constants and their tests); docs/threat-model.md (the row "Online password guessing…") |
| A13 | Whoever knows a user name can use up the account's attempts (30 in an hour, 100 in a day, from a few addresses); until that window ends the account signs in only where it proved its password in the last 30 days. Those addresses are kept in memory only: a restart forgets them, and the counts with them | docs/native-login.md §"Guessing a password online", "What it costs: keeping someone out"; guesses.rs (`OWN_DAYS`, `Own`, `Guesses::weigh`) |

Arrows: 1 device → Central (blinded request, A11, A3), 2 Central → device
(A1), 3 on the device (A2, A3), 4 device → Central (proof, A3), 5 Central →
device (session and wrapper, A4, A6).

## 2. Password and keys in memory (`sec-memory-*`)

| # | Claim | Source |
|---|---|---|
| B1 | Browser: the Worker's Rust code holds the password in `secrecy::SecretString` (overwritten when dropped) only until the OPAQUE step ends | opaque.rs:60-65, :101, :136-140, :176; flow.rs:33-34, :107-112 |
| B2 | OPAQUE's export key and session key are overwritten after the wrapping key is derived; Argon2id's 64 MiB of working memory is overwritten on success, error or unwinding. The page says "the keys it derived on the way and holds itself": what `opaque-ke` holds inside (its states are `ZeroizeOnDrop`; an HKDF state within a call was not traced) and the copy made as the string enters the module are not claimed | opaque.rs:63, :115-116, :138, :194-197; opaque/stretch.rs:20-27, :38-44; docs/crypto-protocol.md:12-13 |
| B3 | Browser: the master key is opened in the Worker to check the sign-in and overwritten at once; the page keeps no key | flow.rs:165-175; crates/central-crypto/src/vault.rs:33 |
| B4 | Browser, account with a chat: the Worker derives the chat keys (HKDF-SHA-512) from the master key and overwrites the master key | flow.rs:409-420; crates/central-crypto/src/vault/derived.rs:15-16, :79; docs/central-chat.md:34-37 |
| B5 | The page empties its password field after submitting and drops its copy after handing it to the Worker; the sign-in page ends its Worker when the flow ends. Not every Worker: a chat page starts one of its own that holds the chat keys unsealed while it is open, and the chat page's password fallback keeps its sign-in Worker as the chat's | auth.rs:479, :137, :99-106; crypto.rs:106-131; crates/central-web/js/crypto-worker.js:9-11, :132-153; islands/chat/mod.rs:341-348, :388-399; chat/engine.rs:175 |
| B6 | **Limit**, browser: the typed text, the message to the Worker and JavaScript strings belong to the browser; they are freed, not overwritten (neither the page's own Rust copies, which could be, nor the browser's, which cannot), and when that memory is reused is the browser's decision | docs/central-chat.md:82, :90-91; auth.rs:131-137, :474-479; crypto.rs:89-91; crypto-worker.js:162, :182 |
| B7 | Kept in the browser while signed in: the chat keys, sealed with AES-256-GCM under a non-extractable Web Crypto key, in IndexedDB. Signing out and deleting the account delete the record; after a session that ended by itself, the next page of the site that loads deletes it (or an open chat that is refused) | crypto-worker.js:78-130; islands/account.rs:111-114, :593-596; chat/engine.rs:215-219; crates/central-web/js/boot.js (the block after `signedIn`); docs/central-chat.md:59-73; docs/threat-model.md:20 |
| B8 | **Limit**: script running on this site, or whoever can read the browser profile while the record exists, can use or obtain the chat keys | docs/central-chat.md:75-81; docs/threat-model.md:20 |
| B9 | Client: the typed password is held in memory that is overwritten when released (`Zeroizing<String>`) and dropped right after the sign-in | baylee:crates/baylee-account/src/account.rs:49-52, :74-75 |
| B10 | Client: the master key stays in memory while signed in, because the client opens the account's encrypted data with it | crates/central-client/src/session.rs:12, :61-75; baylee:docs/central-chat-sync.md:3-5 |
| B11 | The password is never stored, by the browser's code or the client | docs/web-accounts.md:52-55; baylee:docs/proposals/deep-links.md:413-415 |

Arrows: 1 page → Worker (B5), 2 in the Worker (B1, B2), 3 in the Worker
(B3, B4), 4 Worker → storage (B7), 5 the Worker is ended (B5). The frame
says that all of it happens inside the browser.

## 3. Chat (`sec-chat-*`)

| # | Claim | Source |
|---|---|---|
| C1 | A chat key is an Ed25519 key that signs and an X25519 key that messages are wrapped for. The wire calls it a device and allows five per epoch; today every browser and client of an account derives the same one from the master key, so the page says "chat key", not "device" | crates/central-crypto/src/chat/device.rs:1, :35-39; crates/central-crypto/src/vault/derived.rs:72-90; crates/central-client/src/chat.rs:126; docs/central-chat.md:31-38, :108-109 |
| C2 | Sending: one random key per message; the text is encrypted with XChaCha20-Poly1305 under it; the message key is wrapped for each chat key the recipient's account has registered (X25519); the sender's chat key signs header and ciphertext (Ed25519) | crates/central-crypto/src/chat/message.rs:4-6, :142-211, :373-393; docs/central-chat.md:31-33, :43 |
| C3 | Central checks signature and recipients, stores the ciphertext with the wraps and hands each chat key only its own wrap; it has neither message keys nor account keys | crates/central-postgres/src/accounts/social/chat/send.rs:119-123, :171-216; .../chat/inbox.rs:26-45; docs/central-chat.md:15-17, :119-127 |
| C4 | Receiving: check the signature, unwrap the message key with the account's chat key, decrypt | message.rs:212-215, :230 |
| C5 | Central sees metadata: contacts, groups with their names and members, display names, the public chat keys, and each message's signed header in plain text (sender, conversation, signing key, time, kind) and its size; as any server, the address of a request | crates/central-api/src/chat.rs:61-75; crates/central-api/src/groups.rs:27; .../chat/send.rs:247; docs/central-chat.md:15-17 |
| C6 | **Limit**: no double ratchet, no forward secrecy after a recipient key is compromised. The chat key is derived from the master key and Central keeps ciphertext after delivery, so Central's database with its keys and a guessed password, or the recovery code, open the stored messages | docs/central-chat.md:43-44, :124-125; derived.rs:79-90; .../chat/inbox.rs:26-31 |
| C7 | **Limit**: first contact trusts the keys Central names; comparing the safety number over another channel checks them; later key changes block sending until confirmed (website and client) | docs/central-chat.md:45-48; crates/central-client/src/chat.rs:139-146; baylee:crates/baylee-client-core/src/social/keys.rs:41-45, :177, :218, :232-238 |
| C8 | **Limit**: group membership is decided by Central | docs/central-chat.md:48-50 |

Arrows: 1 and 2 on the sender's device (C2), 3 sender → Central (C3), 4
Central → the contact's device (C3), 5 on that device (C4).

## 4. Signing the client in by a link (`sec-pair-*`)

| # | Claim | Source |
|---|---|---|
| D1 | Purpose: someone signed in on the website starts the client signed in; the password takes no part in the exchange | docs/web-accounts.md:455-461; crates/central-client/src/pairing.rs:1-10 |
| D2 | The page asks Central for a pairing (valid five minutes) and draws a one-time secret that Central never sees | docs/web-accounts.md:478-482; crates/central-crypto/src/pairing.rs:48, :61; docs/crypto-protocol.md:161-162 |
| D3 | The link `baylee://pair` carries handle and secret to the client on the same device | docs/web-accounts.md:481-482; central-client/src/pairing.rs:5 |
| D4 | The client makes a fresh X25519 key pair and sends Central the public half and an HMAC-SHA-256 under the secret | docs/web-accounts.md:483-487; docs/crypto-protocol.md:148-150; central-crypto/src/pairing.rs:176-232 |
| D5 | The page verifies that value; both sides show the same short code; the person approves on the page | docs/web-accounts.md:488-490; docs/crypto-protocol.md:151-153; central-crypto/src/pairing.rs:112, :126-130 |
| D6 | The page seals the master key to the client's public key with HPKE (RFC 9180) in PSK mode with the secret; Central stores the seal once | docs/web-accounts.md:491-496; docs/crypto-protocol.md:154-175; central-crypto/src/pairing.rs:334-343 |
| D7 | Central cannot open the seal and, not knowing the secret, cannot make one of its own | docs/crypto-protocol.md:157-162 |
| D8 | The client redeems once, receives a session with the seal, opens it with its private key and the secret; the pairing is deleted whatever the redeem finds | docs/web-accounts.md:498-505; central-client/src/pairing.rs:101, :124; central-crypto/src/pairing.rs:244-253 |
| D9 | A paired session's password proof is the browser session's, not the time of pairing: it counts as no newer than the sign-in on the page it came from, and pairing never renews it. (It is not "never fresh": paired right after a sign-in it passes the step-ups that sign-in would.) | crates/central-postgres/src/accounts/pairing.rs:519-524; docs/web-accounts.md:507-515 |
| D10 | The page that pairs has the master key only when it has just opened it with the password | docs/web-accounts.md:457-461; baylee:docs/proposals/deep-links.md:411-417 |
| D11 | **Status** (2026-11-11): Central's server, its crypto and the client library implement pairing and are tested against each other (`crates/central-identity/src/pairing.rs`, `crates/central-client/src/pairing.rs`, `crates/central-client/tests/support/pairing.rs`). This website has no pairing page (`crates/central-web/src/islands` has none), and the game client's half is not released (game pull request 422, open on that day) | as named |

Arrows: 1 website → Central (D2), 2 website → client, past Central (D3), 3
client → Central (D4, D5), 4 website → Central (D6, D7), 5 Central → client
(D8).

**To remove later:** `sec-pair-status` is the one sentence that says the flow
cannot be used yet; it stands above the chart. When the website has its
pairing page and a released client redeems links, delete that message in
every language, set `status: None` in `PAIRING` in `security.rs`, and update
D11.

## 5. What none of it can do (`sec-limit-*`)

| # | Claim | Source |
|---|---|---|
| E1 | A compromised device, a tampered client or a browser extension that reads pages is outside what this design solves | docs/threat-model.md:41-43 |
| E2 | A taken-over Central can reveal metadata, refuse service and attack sign-ins; it can name false keys at a first contact or add a member to a group and so read later messages (C7, C8); what it stores of data and chats is encrypted and opens with the password or the recovery code | docs/threat-model.md:10-12; crates/central-client/src/chat.rs:139-146; docs/central-chat.md:48-50 |
| E3 | No independent audit: the reviews so far are the implementers' own | docs/security-assurance.md ("not independent external audits"); docs/crypto-protocol.md:201-202; docs/threat-model.md:3 |
| E4 | Losing both the password and the recovery code loses the encrypted data: Central has only the wrappers | A5; docs/threat-model.md:22 |
| E5 | A website's code comes from its server: whoever takes the server over could deliver code that captures the password; integrity hashes and the Content Security Policy guard against changes from outside, not against that | crates/central-web/src/site/shell.rs:190-218 (`integrity=`); crates/central-web-server/src/routes.rs:160-163; docs/threat-model.md:32 |
| E6 | The client checks a signature (Ed25519) before it installs an update; an archive without a valid one is refused | baylee:crates/baylee-update/src/lib.rs:10-11; baylee:crates/baylee-update/src/check.rs:14-15, :333, :389, :425-452 |
| E7 | Sign-in follows OPAQUE (RFC 9807) with Argon2id (RFC 9106) and pairing seals with HPKE (RFC 9180): informational RFCs of the IRTF, not IETF standards. The pairing's sequence and the chat format are Baylee's own combinations of standard primitives | docs/crypto-protocol.md:9-11, :17-18, :154-156; crates/central-crypto/src/pairing.rs:1-16; docs/central-chat.md:31-33 |

## 6. The two links to the page

| # | Claim | Rows |
|---|---|---|
| F1 | FAQ (`faq-security-a`): the device proves with OPAQUE that it knows the password without sending it; chat messages are encrypted on the device; "the one exception and the limits" are on the page | A1, A8, C2 |
| F2 | Sign-in page (`sec-signin-link`): a link, no claim | none |

## How the page is built

- No script. Each diagram is inline SVG drawn by the server; six radio
  buttons before it choose "play" or one step, and `style/security.css`
  reads which is checked. The site's policy (`style-src 'self'`, no inline
  style, no inline script) is unchanged; the page adds no request to another
  origin. The four documents it links to open in a tab of their own with
  `rel="noopener noreferrer"`.
- Many languages. SVG text neither wraps nor shrinks, so a diagram holds no
  sentence: only its actors' names (`sec-actor-*`, a word or two) and the
  steps' numbers. Each name has room for twice its English width; the test
  `names_fit_with_room_to_grow` measures every language of `Lang::ALL`, and
  the browser test measures the drawn text. What a step sends
  (`sec-*-c1` to `c5`), its title and its sentence are HTML in the list
  beside the diagram, under the same number and the same sign. Every word,
  visible or spoken, is a message in `site.ftl`; the page has no `match` on
  a language.
- The diagram is hidden from assistive technology (`aria-hidden`); the
  ordered list beside it says the same steps in words. The radio group is
  reachable by keyboard, its labels are 44 px targets, and the ring shows on
  the chosen step's label.
- Choosing a step stops the loop (WCAG 2.2.2). A step that stands back dims
  its drawing only: its number and every word keep their contrast.
- `prefers-reduced-motion: reduce`: nothing moves; the whole diagram is
  shown at rest, the first button reads "All steps", and choosing a step
  only lights it.
- Hovering changes colours only (`tests/hover.rs`).

## Review (three passes, by the implementing agent; not an independent audit)

Design pass: every sentence was written from the table above, not the other
way round. Three traps were looked for on purpose. "Never": the legacy
gateway accounts (A8) are an exception in the client, so the sign-in section
says "in none of them" about OPAQUE's messages and names the exception in its
own note; the website itself refuses such accounts (A9). Memory: the browser
and the client differ, and what the browser cannot promise (B6, B8) is said in
the section, not in a footnote. Pairing: built and tested in Central and the
client library, but usable from neither this website nor a released client
(D11), so the section says so in one removable sentence.

Implementation pass: no `style` attribute and no script on the page
(`crates/central-web-server/tests/pages.rs` checks every listed page);
positions are SVG attributes, movement is classes; every word is a message in
both languages; external links only through `rich`, which adds the tab and
`rel`; no new dependency. The words the page must not use are refused by
`no_promises` in every language's file.

Adversarial pass: each sentence with "only", "never", "cannot" or "nobody"
was read against its source for a counterexample. Findings that changed the
text: Central does see the client's public key in pairing, so the page says
Central "cannot make one of its own" because of the secret, not because the
key is hidden (D7); the browser keeps the chat keys, so "nothing stays" would
be false and step 4 shows what stays (B7); a session cookie is "a cookie the
page itself cannot read", not "no cookie" (A4); the FAQ sentence points to
"the one exception" (F1).

## Verified on 2026-11-11

- `cargo test -p central-web -p central-web-server -p xtask --all-targets
  --all-features`, clippy `-D warnings` for the same crates, `cargo fmt --all
  -- --check`, `cargo xtask policy`, `cargo xtask site`.
- `web/e2e-site/security.spec.ts` (20 tests: desktop and phone; both
  languages and both colour schemes with axe-core for WCAG 2.2 A/AA and the
  44 px target check; the keyboard; reduced motion; no sentence in a
  diagram; 320 to 2560 px without sideways scrolling; the three links to the
  page), against a `central-web` on a port of its own with the synthetic
  data of `crates/central-web-server/tests/data`.
- Looked at in Chromium, dark and light, at 1440, 768, 390 and 320 px, and
  at rest with reduced motion.

## Not verified

- No lawyer has read the page; it makes no statement about legal
  compliance and should not start to.
- No independent cryptographer has reviewed the protocols or this page (E3).
- The deadline of the legacy exception (A8) is not a date on the page: the
  sources name a deadline, not its day.
- The claims about the client (B9, B10, E6) were read in the game
  repository's sources on 2026-11-11, not tested against a released client.
- RFC section numbers in A11 are from the published RFC, not re-read line by
  line for this page.
- Only Chromium rendered the page; Firefox and Safari were not tried. The
  stylesheet uses nothing newer than `transform-box` and custom properties
  inside keyframes.
- No screen reader was run; the structure was checked with axe-core and by
  reading the accessibility tree's roles and names in the tests.
- The words exist in English and German. A new language has to add its own
  words to the list in `no_promises`.
