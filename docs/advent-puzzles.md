# Advent tactical puzzles

Each player sees the same released board, the goal, both life totals, both sides' creatures, their own mana and spells, and the schedule of up to three decisions. A displayed draw adds that exact card to the hand. Either player can reach zero life after damage. A player with positive life wins immediately if the opponent reaches zero. The survival goal also succeeds when the last step ends with the player alive.

## Legal moves

- During a casting window, pass or play one card from hand and pay its printed generic mana cost. Each spell names its permitted targets. Identical cards in hand offer the same action and consume one copy.
- During an attack, choose any subset of living untapped friendly creatures. The printed opponent plan names each blocker's target. A blocker stays back if its designated attacker stays back or becomes ineligible.
- When defending, assign each untapped friendly creature to at most one active attacker. Several blockers can defend against the same attacker. A flying attacker requires a blocker with flying or reach.
- First-strike damage resolves before ordinary damage. Damage within one step is simultaneous. Lifelink applies in the same step. A creature marked with lethal damage or dealt positive deathtouch damage leaves the board. Trample can reach the defending player after lethal assignment. The puzzle automates damage allocation in stable blocker slot order and sends eligible trample excess to the player. An untapped vigilant attacker stays untapped. These rules use the published combat and keyword definitions. [Magic rules](https://magic.wizards.com/en/rules) · [Keyword glossary](https://magic.wizards.com/en/keyword-glossary) · [Combat damage assignment changes](https://magic.wizards.com/en/news/feature/foundations-release-notes)
- Where a new turn is explicitly marked, clear damage and temporary abilities or power changes, untap the active side and install the displayed mana. Surviving pieces keep their slot; defeated pieces never become targets again.

## Shared evaluation

`Puzzle::position` replays a prefix from a fresh board. `Puzzle::choices` returns all legal next actions. The client can preview its own board and retry instantly. Central validates each claim by applying `Puzzle::won` to the released server-owned board. The line contains only player decisions. Effective state, win status and reward identifiers come from the authority. Malformed or excessive input is rejected during replay or bounded decoding.

The private authoring process exhaustively visits all legal branches through `Puzzle::study` under an explicit work limit. It counts distinct winning and unsuccessful terminal lines, replays a winning witness, and checks stage depth. A make-or-break decision has at least one legal choice that removes every possible winning continuation. The minimum number of those decisions over all winning lines measures whether later steps actually depend on earlier play. Within a stage-depth and decision-difficulty band, the authoring study also weighs the number of choices, unsuccessful continuations and interacting abilities. The result measures tactical complexity; human solve times require observed play.

Campaign previews, bilingual hints, solution witnesses and door-to-board mappings stay in private operator stock. Repository tests use synthetic boards covering combat, legal targets, costs, fixed draws, alternate solutions and work limits.
