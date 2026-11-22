# Ten-game player and deck placements

On 22 November 2026 the production ranked rules processed 2,764,800 simulated victories and defeats across 27,648 new careers. The three tracks share these rules and keep separate records. Each of nine scenarios used six fixed seeds with 512 careers per seed. A career lasted 100 games at ten games per simulated day. The first deck started at 1500 Elo. The experiment supplied opponents directly and measured the rating and league decisions after each match.

In the dense field, opponents were established accounts with true skill equal to the nearest 50-point interval around the seeker's provisional matchmaking Elo. Each opponent brought a 1500-Elo deck. In the two-player fields the returning rival kept its earned player and deck ratings. Its intrinsic strength stayed fixed, as did the seeker's. The chance of winning was 4% plus 92% of the logistic Elo probability. Player strength alone determined outcomes. Deck Elo therefore measures how much of an initially unknown player's strength was attributed to their deck. These are controlled synthetic estimates.

| Field | True player Elo | Mean wins at game 10 | Player Elo at 10 | Matchmaking Elo at 10 | Deck Elo at 10 | Player Elo at 100 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Dense | 1100 | 1.69 | 1409.8 | 1361.8 | 1382.2 | 1120.4 |
| Dense | 1500 | 5.01 | 1500.5 | 1500.8 | 1500.9 | 1500.8 |
| Dense | 1900 | 8.33 | 1591.2 | 1639.9 | 1619.2 | 1880.5 |
| One 1000-Elo opponent | 1100 | 6.31 | 1480.5 | 1419.5 | 1396.3 | 1333.3 |
| One 1000-Elo opponent | 1500 | 9.13 | 1500.8 | 1501.0 | 1489.2 | 1501.9 |
| One 1000-Elo opponent | 1900 | 9.55 | 1503.9 | 1513.5 | 1503.9 | 1531.3 |
| One 2000-Elo opponent | 1100 | 0.47 | 1497.4 | 1487.4 | 1496.8 | 1470.4 |
| One 2000-Elo opponent | 1500 | 0.91 | 1501.2 | 1500.8 | 1512.2 | 1500.0 |
| One 2000-Elo opponent | 1900 | 3.75 | 1527.4 | 1587.3 | 1607.4 | 1666.1 |

Every account stayed unranked through game nine and received a rank at game ten. Against dense opponents, 1,981 of 3,072 weaker players opened in Paper, 2,051 average players opened in Iron, and 1,415 stronger players opened in Bronze. Provisional players won 15,742 times against a favourite rated at least 350 points higher during the first ten games in the strong-rival field. The largest favourite loss was 10.00 points.

Repeated games against just one opponent gave little information about absolute skill. A true 1900-Elo player who won 95.54 of 100 games against the same initially 1000-Elo rival finished at 1531 on average. This is the trade-off for reduced repeat stakes and bounded adjustments. These results support showing the number of opponents played alongside public Elo when presenting sparse-pool statistics. Queue waiting times and seasonal progression were examined in the separate Baylee league lab.

## Time to the elite leagues

A second run followed 384 careers at each of three fixed strengths for 320 consecutive days, ten games per day, in the dense field. Central processed 3,686,400 additional results. The table counts the first occasion a player held each league. Day 1 is the first day played. The median only includes players who reached the league.

| True player Elo | Reached Master | First / median day | Reached Grandmaster | First / median day |
| ---: | ---: | ---: | ---: | ---: |
| 2100 | 158 of 384 | 200 / 249 | 0 of 384 | — |
| 2300 | 384 of 384 | 200 / 200 | 384 of 384 | 250 / 254 |
| 2500 | 384 of 384 | 200 / 200 | 384 of 384 | 250 / 250 |

Every simulated Master had played 2,000 games over at least 200 play days. Grandmaster required at least 2,500. With repeated chances over 320 days, even some 2100-strength players briefly crossed the 2240 threshold. First passage does not measure how long they remained there.

## Changes to a placed deck

The deck-revision study started with three established reference ratings and a 60-card main deck plus 15 sideboard cards. Copies shared identities. Each experiment changed one to 75 copies, applied the production inheritance and settlement functions, compared direct changes to staging with a match between edits, and then played ten games on the final version along both paths. The in-memory test book models a single established reference and indexes version records by the sorted multiset. The PostgreSQL store suite separately exercises persisted records.

Across 71,100 result applications, direct and staged edits had the same final-version starting Elo and ten-game record. All 2,700 returns to an original version or main/sideboard permutations kept Elo and completed games. Full replacement left 150 Elo. A control that repeatedly inherited from the immediate predecessor retained a median 1,189 additional Elo at full replacement.

## Replaying the study

From the Central workspace, invoke the three opt-in tests with the pinned toolchain.

```sh
cargo test --locked -p central-postgres --lib placement_study -- --ignored --nocapture
```

The tests print the number of placements in every league, the mean public and matchmaking ratings, deck Elo, observed upset caps and the deck revision controls. They use the same settlement and league functions that Central uses for live results.
