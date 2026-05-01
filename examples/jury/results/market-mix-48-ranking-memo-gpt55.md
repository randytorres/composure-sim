# Market Mix 48 Ranking Memo - GPT-5.5

## Status

The 48 experimental ideas from `market-research-report-2026-2027.md` are now ranked in four bracketed GPT-5.5 jury runs, and the strongest/most interesting 16 were ranked again in a cross-bracket finals run.

This is the honest structure:

- **All 48 ideas:** ranked within their 12-idea bracket.
- **Top/founder-interest 16:** directly compared against each other in finals.
- **Not done:** one giant 48-way comparative ranking. That would be less reliable because the prompt becomes too large and the model tends to flatten niche ideas.

## Artifacts

- All 48 source request: `market-mix-gaming-biotech-social-entertainment-48-request.json`
- Bracket A result/report: `market-mix-a-games-jobs-bracket-gpt55-result.json`, `market-mix-a-games-jobs-bracket-gpt55-report.md`
- Bracket B result/report: `market-mix-b-biotech-body-bracket-gpt55-result.json`, `market-mix-b-biotech-body-bracket-gpt55-report.md`
- Bracket C result/report: `market-mix-c-social-agon-bracket-gpt55-result.json`, `market-mix-c-social-agon-bracket-gpt55-report.md`
- Bracket D result/report: `market-mix-d-entertainment-mystery-bracket-gpt55-result.json`, `market-mix-d-entertainment-mystery-bracket-gpt55-report.md`
- Finals request/result/report: `market-mix-finals-top-plus-wildcards-request.json`, `market-mix-finals-top-plus-wildcards-gpt55-result.json`, `market-mix-finals-top-plus-wildcards-gpt55-report.md`
- Finals synthesis: `market-mix-finals-top-plus-wildcards-gpt55-synthesis.json`, `market-mix-finals-top-plus-wildcards-gpt55-synthesis.md`

## Coverage

| Run | Concepts | Sessions | Reactions | Failures |
|---|---:|---:|---:|---:|
| Bracket A - Games / Jobs | 12 | 9 | 108 | 0 |
| Bracket B - Biotech / Body | 12 | 10 | 120 | 0 |
| Bracket C - Social / AGON | 12 | 10 | 120 | 0 |
| Bracket D - Entertainment / Mystery | 12 | 10 | 120 | 0 |
| Finals - Top + Wildcards | 16 | 10 | 160 | 0 |

Bracket A has 9 sessions rather than 10, but every concept still received 9 complete comparative reactions and no cells failed.

## Finals Ranking

| Rank | Concept | Avg Rank | #1 Votes | Top 3 | Signup Y/M/N | Tell-Friend Y/M/N |
|---:|---|---:|---:|---:|---:|---:|
| 1 | Playable Trailer Lab | 7.00 | 1 | 2/10 | 2/7/1 | 2/7/1 |
| 2 | REALM Oath Court Mini | 7.00 | 0 | 3/10 | 3/3/4 | 3/3/4 |
| 3 | The Witness Board | 7.10 | 2 | 2/10 | 3/3/4 | 3/3/4 |
| 4 | Bloodwork Mystery Board | 7.40 | 3 | 4/10 | 3/4/3 | 3/3/4 |
| 5 | Watch Party Prediction Court | 7.40 | 0 | 2/10 | 2/5/3 | 2/5/3 |
| 6 | Court of Public Opinion: Fiction Mode | 7.60 | 0 | 1/10 | 0/9/1 | 0/9/1 |
| 7 | Body Data Black Box | 7.80 | 0 | 4/10 | 3/3/4 | 3/3/4 |
| 8 | Tow Yard Cold Dispatch | 8.40 | 2 | 3/10 | 3/3/4 | 3/3/4 |
| 9 | Discord Quest Tribunal | 8.60 | 1 | 3/10 | 3/3/4 | 3/3/4 |
| 10 | Airport Ramp Black Box | 8.60 | 0 | 1/10 | 2/5/3 | 2/5/3 |
| 11 | Supplement Court Of Claims | 8.70 | 0 | 3/10 | 2/6/2 | 2/7/1 |
| 12 | Friend Group Newspaper With Receipts | 8.70 | 0 | 0/10 | 1/6/3 | 1/6/3 |
| 13 | Skywatch Unknown Card | 9.40 | 0 | 0/10 | 0/9/1 | 0/9/1 |
| 14 | AGON Seven-Day Escape Pilot | 9.50 | 1 | 1/10 | 1/4/5 | 2/3/5 |
| 15 | Immersive Night Operator | 10.60 | 0 | 1/10 | 1/4/5 | 1/4/5 |
| 16 | SPECTER RF Ghost Replay | 12.20 | 0 | 0/10 | 0/2/8 | 0/3/7 |

## Bracket Winners

### A. Games / Jobs / Dark Strategy

1. Tow Yard Cold Dispatch
2. Airport Ramp Black Box
3. The Witness Board
4. Private Security Incident Log
5. THE SHOP After Midnight

Notable result: REALM Oath Court Mini ranked only 8th in the broad games/jobs bracket, but rose to 2nd in finals when compared against cross-sector artifacts. The read is not "Realm is dead"; it is "Realm needs the right framing: one oath, one witness, one betrayal, one receipt."

### B. Biotech / Body Evidence

1. Body Data Black Box
2. Bloodwork Mystery Board
3. AGON Seven-Day Escape Pilot
4. Lab Anomaly Interviewer
5. Supplement Court Of Claims

Notable result: AGON is extremely polarizing. It got five #1 votes in the body bracket, but also four last-place votes. That means the format has heat, not consensus.

### C. Social / AGON / Local Status

1. Playable Trailer Lab
2. Court of Public Opinion: Fiction Mode
3. Discord Quest Tribunal
4. Friend Group Newspaper With Receipts
5. Local Relic Pass

Notable result: The jury liked formats that turn passive content into a playable artifact, but repeatedly punished anything that sounded like moderation debt or community admin.

### D. Entertainment / Mystery / Proof

1. Watch Party Prediction Court
2. Immersive Night Operator
3. Skywatch Unknown Card
4. Episode Aftermath Cards
5. Local Mystery Case Walks

Notable result: Skywatch held up as broadly admired but cautious. SPECTER remained a founder-taste object with unclear buyer/distribution.

## Synthesis Read

The strongest pattern is **artifact-first consequence**. Winners are not broad platforms; they are small loops that output a thing people can inspect:

- playable scene
- oath receipt
- witness board
- bloodwork case board
- prediction scorecard
- body-data crash report
- dispatch log

The jury penalized three things hard:

- too many systems before a first loop
- trust/safety or moderation debt
- ideas that turn fun, health, or social life into homework

## Best Next Tests

1. **Playable Trailer Lab** - build a one-link demo for one fictional show/game/product and test whether people actually interact before watching.
2. **REALM Oath Court Mini** - storyboard or prototype the three-turn betrayal receipt. This remains the strongest founder-fit world test.
3. **The Witness Board** - test as a Discord-native fictional social deduction game, not a broad social platform.
4. **Bloodwork Mystery Board / Body Data Black Box** - test as one beautiful evidence artifact, not a health app.
5. **Tow Yard Cold Dispatch** - keep as the best fresh job-sim wedge; it has ugly, legible pressure and a strong first scene.

