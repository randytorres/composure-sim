# Decision Lab Roadmap

## Vision

Turn `composure-sim` into a synthetic decision lab: a place to test business
ideas, positioning, designs, gameplay loops, and procurement narratives against
large synthetic populations before spending real-world budget.

The product should feel as easy as writing a brief, but the core advantage
should be stronger than a black-box agent demo:

- every run starts from an editable JSON artifact
- every scenario is deterministic and rerunnable
- every recommendation can be traced to segments, variants, touchpoints, and
  observed calibration data
- every project can reuse the same engine without copying strategy logic into
  the core

## MiroFish Benchmark

Public MiroFish positioning emphasizes natural-language setup, GraphRAG-backed
population/world generation, thousands of interacting agents, variable injection,
structured prediction reports, and post-run exploration through agent interviews.
Useful references:

- https://www.mirofish.work/
- https://github.com/666ghj/MiroFish

That is the right product bar for ease and vividness. Our target is to beat it
on decision quality:

- structured drafts before execution
- repeatable matrix testing instead of one narrative run
- calibration against observed outcomes
- explicit robustness rollups across adverse scenarios
- diffable artifacts that downstream projects can store, review, and rerun
- deterministic reports that work without network calls

## Product Pillars

### 0. Idea Portfolio Simulator

Status: started.

The newest slice is `draft-idea-portfolio`, `run-idea-portfolio`, and
`export-idea-portfolio-markdown`. It accepts a plain-text or JSON idea scan,
extracts many business ideas, then scores them across default consumer AI
segments and scenarios.

This directly supports the "50 ideas, find the most testable wedges" workflow
without forcing raw business ideas into the narrower concept-test schema.

Next upgrades:

- explicit market-evidence imports with source links, dates, and sample sizes
- richer scenario packs for gameplay, defense, local services, creator tools,
  social commerce, and consumer wellness
- idea clustering so duplicate categories do not dominate the leaderboard
- auto-generated live-test plans for the top 3 ideas
- calibrated priors from previous launches and project-specific outcomes

### 1. Brief To Matrix

Status: started.

The first slice is `draft-concept-test-matrix`. It accepts a JSON or plain text
brief and emits an editable `ConceptTestMatrixRequest`.

This is the bridge between MiroFish-style natural input and our artifact-first
system. The rule is: draft first, inspect/edit second, run third.

Next upgrades:

- richer extraction of explicit variants, audiences, constraints, and observed
  metrics from briefs
- optional LLM-assisted drafting that still emits the same JSON schema
- confidence warnings when the brief is too underspecified
- domain packs for startup, consumer, gameplay, defense procurement, civic, and
  creative testing

### 2. Population Lab

Status: started through `composure-population` integration.

The system needs configurable synthetic people, not vague personas. Each segment
should expose traits, channels, objections, budgets, trust thresholds, and
scenario-specific sensitivities.

Next upgrades:

- correlated trait generation
- imported seed audiences from product repos
- persona interview samples after a run
- segment coverage checks that warn when a matrix lacks important buyer types
- synthetic panel snapshots for "show me 25 likely reactions"

### 3. Scenario Matrix

Status: started.

Matrices now run one concept test across multiple scenario cases and produce
variant robustness rollups.

Next upgrades:

- more presets for defense, gameplay, enterprise sales, retail, policy, and
  creative audience testing
- scenario parameter sweeps for population size, channel mix, price, evidence
  strength, onboarding friction, and trust shocks
- case clustering so similar stress tests do not overcount the same signal
- explicit "decision gate" outputs: kill, revise, run small live test, scale

### 4. Calibration And Reality Checks

Status: started with aggregate and touchpoint observed outcomes.

The long-term advantage is calibration. A simulation should get more useful as
real tests come back.

Next upgrades:

- calibration score per matrix case
- confidence intervals and residual summaries
- automatic recommendations for which metric to measure next
- import adapters for ad tests, landing pages, gameplay telemetry, sales calls,
  and procurement feedback
- warning labels for uncalibrated fantasy runs

### 5. Post-Run Exploration

Status: planned.

MiroFish has an appealing post-run exploration shape. We should add our own
version, but grounded in the run artifact.

Next upgrades:

- interview sampled individuals from the actual run output
- ask "why did this segment reject variant B?"
- generate segment-specific objections and next-test scripts
- compare two run artifacts through a conversational inspector
- preserve transcripts as evidence, not as the source of truth

### 6. Interfaces

Status: CLI-first.

The CLI is enough for core development, but the decision lab eventually needs a
review surface.

Next upgrades:

- local HTML report for matrix results
- side-by-side scenario and variant diff viewer
- project folders with briefs, drafted requests, runs, reports, and observed data
- shareable Markdown/PDF reports
- optional app/API wrapper once the artifact model settles

## Near-Term Build Plan

1. Harden the draft compiler.
2. Add richer domain packs and presets.
3. Add persona interview samples from real run outputs.
4. Add confidence and calibration scoring to matrix reports.
5. Add a run folder convention so every project can keep brief, draft, result,
   report, and observed outcomes together.
6. Build a small local report UI once the artifacts are stable.

## Design Principle

The simulator should become easy to talk to, but never lose the artifact.

Natural language is the front door. JSON is the contract. Deterministic,
calibrated, inspectable runs are the moat.
