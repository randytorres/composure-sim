# composure-sim marketing simulation system roadmap

## Goal

Build `composure-sim` into a reusable marketing simulation system that can support
very different products, including:

- AC
- Mirrorlife / SelfRX
- future products with different channels, audiences, and conversion flows

The goal is **not** to clone MiroFish. The goal is to build something better on
top of composure's stronger foundation:

- deterministic where possible
- artifact-first
- calibratable
- inspectable
- reusable across products

## Current status

The repo now has an initial V2 marketing layer in progress:

- persona-aware request and result schemas
- persona-level scorecards
- reusable scenario families in the request model
- scenario-family dynamics in the simulation path
- `simulate-marketing-v2` in the CLI

That means the next steps are no longer "invent V2 from scratch." The next
steps are to deepen the model, improve outputs, and calibrate it.

## Product thesis

Today `composure-sim` has a strong engine and a thin marketing adapter.

That means it can already help with:

- ranking hypotheses
- comparing messages consistently
- structuring ICP assumptions

But it is still weak at:

- persona-level explanation
- campaign-sequence simulation
- network effects
- store / budtender / member ecosystems
- calibration from real-world outcomes
- rich output artifacts that decision-makers can actually use

The next version should become:

1. a reusable simulation engine
2. a reusable marketing adapter
3. a reusable reporting and inspection layer

Downstream repos should supply domain truth and observed data, not invent new
core simulation logic.

## Repo boundaries

### What belongs in `composure-sim`

Put it here if it is reusable across products.

Examples:

- richer generic marketing state models
- persona-level output schemas
- funnel metric output schemas
- counterfactual and sequence simulation
- calibration interfaces and scoring workflows
- multi-entity and network simulation
- deterministic reports and inspectors
- reusable marketing CLI and HTTP surfaces

Rule:

If the same feature would help AC, Mirrorlife, and a third future product, it
belongs in `composure-sim`.

### What belongs in downstream product repos

Downstream repos should own:

- persona definitions
- scenario files
- evidence notes
- real funnel and growth data
- project-specific metrics
- interpretation of results

Examples:

- AC owns firefighters, run club, budtenders, Vault, golf, and store hypotheses
- Mirrorlife owns GLP-1, peptide, privacy, coach-share, landing page, and TikTok scenarios

Rule:

If it depends on product strategy or ground truth from one business, it should
stay in that repo.

## What MiroFish has that we should beat

MiroFish appears to have a stronger simulation product layer:

- agent/world framing
- long-term memory
- GraphRAG and world building
- persona generation
- interactive post-run exploration
- more visibly "alive" output

We should aim to beat it on:

- deterministic reproducibility
- calibration against real outcomes
- artifact quality
- counterfactual analysis
- multi-project reusability
- inspectability

The design target is:

- more vivid than a scorer
- more grounded than a pure LLM playground

## Current limitations

The current marketing adapter is still a compact heuristic layer.

Current traits:

- text-heavy audience matching
- one blended audience result
- fixed 6-step approach evaluation
- only 3 hidden state dimensions:
  - attention
  - resonance
  - share propensity
- only 2 main headline scores:
  - engagement
  - viral potential

That is useful for rough ranking, but not enough for serious planning.

## System requirements for V2

The next useful version of the marketing system should answer:

1. Which audience is most receptive?
2. Which message wins with which segment?
3. Which channels fit which messages?
4. Which messages help conversion vs awareness vs retention?
5. Which sequence of touches works best?
6. What likely happens if we swap strategies?
7. How sure are we?
8. How does the model compare to observed performance?

## Phased roadmap

## Phase 1: useful

### Goal

Upgrade the current adapter from "two scores" into a real decision-support layer.

### Deliverables

#### 1. Persona-level output

Instead of one blended audience result, return:

- per-persona scores
- weighted aggregate scores
- segment conflict summaries

Example output shape:

- best message for wellness buyers
- best message for budtenders
- weak audience fit warnings

#### 2. More than 2 scores

Add a richer output schema with reusable marketing metrics such as:

- `consumer_receptivity`
- `conversion_intent`
- `credibility`
- `share_likelihood`
- `community_belonging`
- `recommendation_lift`
- `retention_fit`
- `objection_pressure`

The exact mix can evolve, but the key is to separate different jobs.

#### 3. Channel-native output

Add outputs that feel native to the channel or context being simulated.

Examples:

- TikTok:
  - hook strength
  - watch continuation
  - shareability
- landing page:
  - clarity
  - trust
  - signup intent
- X:
  - novelty
  - credibility
  - debate risk
- in-store:
  - recommendation confidence
  - story-retention
  - menu-fit
- private event:
  - trust depth
  - referral likelihood
  - partner progression

#### 4. Better artifact/report schema

Add report fields such as:

- why this won
- why this lost
- main objections
- failure modes
- uncertainty band
- recommended next test

### Outcome

At the end of Phase 1, the system should already be much more useful for AC and
Mirrorlife without changing the engine architecture dramatically.

## Phase 2: believable

### Goal

Move from isolated-message scoring to campaign and funnel simulation.

### Deliverables

#### 1. Sequence simulation

Support simulations like:

- first touch
- second exposure
- proof follow-up
- objection handling
- conversion CTA
- retention/reactivation

This should build on composure's scenario and counterfactual foundations rather
than inventing a separate product-specific engine.

#### 2. Counterfactual comparisons

Support direct strategy comparisons such as:

- firefighter-first vs run-club-first
- budtender-first vs consumer-first
- GLP-1-proof vs peptide-proof
- privacy-first vs outcome-first

#### 3. Funnel-state dimensions

Introduce more reusable marketing state dimensions.

Good candidates:

- hook strength
- credibility
- clarity
- objection load
- saveability
- shareability
- conversion intent
- fatigue

Not every scenario will use every dimension equally, but this is closer to
marketing reality than the current 3-variable abstraction.

#### 4. Scenario types

Add reusable scenario families:

- `landing_page`
- `short_form_video`
- `community_event`
- `in_store_enablement`
- `private_relationship`
- `founder_story`

These should be generic enough to reuse across products while still producing
channel-native behavior.

### Outcome

At the end of Phase 2, the system should be believable enough to drive real
planning decisions, not just brainstorming.

## Phase 3: differentiated

### Goal

Make the system meaningfully better than MiroFish for business decision support.

### Deliverables

#### 1. Multi-entity network simulation

This is where AC and future products benefit most.

Model entities such as:

- consumers
- budtenders
- stores
- founders
- creators
- members
- events

And simulate:

- referrals
- recommendation cascades
- store effects
- social proof spillover
- repeated exposure
- partner relationship compounding

This should likely live in `composure-network`, not `composure-core`.

#### 2. Portfolio simulation

Most marketing plans are bundles, not single messages.

Add support for scoring a portfolio of approaches:

- top-of-funnel hook
- credibility content
- objection-resolver
- conversion page
- retention follow-up

Outputs should cover:

- complementarity
- redundancy
- fatigue
- coverage across personas
- overall expected outcome mix

#### 3. Interactive result inspection

Add a reusable inspector layer that can show:

- persona leaderboard
- objection heatmap
- funnel stage breakdown
- network propagation summary
- regime changes over time
- recommended next comparisons

This is where we can borrow the best UX ideas from MiroFish without copying its
likely black-box architecture.

### Outcome

At the end of Phase 3, the system should be differentiated:

- reusable
- inspectable
- calibrated
- network-aware
- visibly useful for real strategy work

## Phase 4: calibration

### Goal

Tie simulation to observed outcomes so it becomes progressively more trustworthy.

### Deliverables

#### 1. Calibration input contracts

Downstream repos should be able to provide observed data such as:

- acquisition source
- creative ID
- hook ID
- landing variant
- signup conversion
- activation
- retention
- revenue / paid conversion
- event attendance
- reorder velocity
- recommendation rate

#### 2. Parameter fitting workflows

Use `composure-calibration` and future extensions to fit parameters such as:

- base receptivity
- objection sensitivity
- platform response
- sequence fatigue
- referral strength

#### 3. Confidence and reliability outputs

The system should say not just what it predicts, but:

- how confident it is
- which parts are weakly calibrated
- which recommendations are still exploratory

### Outcome

At the end of Phase 4, the system becomes genuinely hard to replace.

## Suggested implementation order

### Track A: `composure-marketing-v2`

Start here.

This track should define:

- new request schema
- new output schema
- persona-level results
- richer metric families
- better reports

This is the fastest path to making the system meaningfully better.

### Track B: sequence + counterfactual marketing simulation

After V2 metrics land, add:

- campaign sequence runs
- strategy branch comparison
- budget and fatigue effects

### Track C: `composure-network`

After sequence simulation is solid, add:

- multi-entity graph runtime
- propagation rules
- network reports

### Track D: calibration and inspection

Parallel or slightly after the above:

- observed-data contracts
- fitting workflows
- browser inspector
- richer report explorer

## Immediate next step

The next concrete artifact to create is:

**`composure-marketing-v2` schema design**

That should define:

1. request fields
2. reusable state dimensions
3. output metrics
4. persona-level reporting
5. counterfactual/report artifact shapes

## Non-goals

For now, this roadmap intentionally does **not** include:

- marketing-os integration planning
- product-specific business logic inside `composure-sim`
- unbounded LLM-agent worldbuilding inside the engine core

Those can come later or live elsewhere.

## Success criteria

We should consider this roadmap successful when:

- AC can simulate audiences, community systems, budtenders, stores, and Vault hypotheses without product-specific hacks in the engine
- Mirrorlife can simulate ICP, landing page, short-form content, funnel progression, and activation hypotheses using the same reusable machinery
- `composure-sim` becomes the obvious reusable home for simulation work across products
- the system is more useful than MiroFish for real decision-making, not just more theatrical
