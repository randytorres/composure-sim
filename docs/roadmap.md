# composure-sim roadmap

## Goal

`composure-sim` should stay a small, reusable simulation foundation rather than turning into a single domain product.
The next step is to add the layers that make a simulation engine practical across many projects:

- experiment packaging
- counterfactual comparison
- parameter exploration
- calibration
- deterministic reporting
- optional multi-entity/network simulation

## Current state

Today the repo provides a solid engine core:

- state representation via `SimState`
- domain-injected transitions via `Simulator`
- Monte Carlo execution
- composure/archetype analysis
- replay/state snapshot support
- Python and WASM bindings

What it does not yet provide is the product-facing layer around those primitives:

- reusable experiment bundles
- baseline-vs-intervention comparison
- sweep/calibration workflows
- summary artifacts for downstream tools
- CLI/reporting surfaces

## Design principles

### Keep core general-purpose

The core should remain domain-agnostic and numeric. It should not absorb application-specific concerns such as:

- graph databases
- LLM-driven ontology generation
- social-media-specific actions
- long-running app servers
- UI workflow logic

### Prefer deterministic artifacts

Everything useful should be serializable and portable:

- scenarios
- parameter sets
- run outputs
- comparison summaries
- report inputs

### Split platform features from engine features

When a capability can live in an optional crate, keep it out of the engine crate by default.

Examples:

- network/entity simulation should be an add-on crate
- report generation should be deterministic in core and richer elsewhere
- browser inspectors and CLIs should wrap the artifact model instead of owning custom formats

## Prioritized roadmap

### 1. Experiment bundles

Status: started

Add a reusable artifact model for:

- experiment spec
- parameter sets / variants
- run records
- Monte Carlo outputs
- composure outputs
- replay outputs

This becomes the portable unit that other tools can save, diff, rerun, and inspect.

### 2. Trajectory comparison

Status: started

Add first-class support for:

- baseline-vs-candidate comparison
- delta summaries
- divergence detection
- break-point shift analysis
- counterfactual result summaries

This is one of the highest-leverage features because nearly every simulation project ends up comparing interventions.

### 3. Sensitivity and parameter sweeps

Status: started

Add support for:

- grid sweeps
- sweep execution over generated cases
- random sweeps
- Latin hypercube style sampling
- ranked sensitivity summaries
- threshold maps

Current progress in core:

- grid case generation
- end-to-end sweep execution with caller-supplied parameter mapping
- experiment-backed sweep execution with bundle recording and failure collection
- scalar objective extraction from deterministic run summaries
- ranked numeric/categorical sensitivity summaries
- deterministic random and Latin hypercube sampling strategies

This should likely become a dedicated crate such as `composure-sensitivity`.

### 4. Calibration / fitting

Status: planned

Add a fitting layer for observed trajectories:

- fit simulator parameters to historical data
- return best-fit parameter sets
- expose residual/error summaries
- preserve deterministic seeds and search settings

Without calibration, users can simulate but cannot easily tune their model to reality.

### 5. Deterministic reporting

Status: started

Add a non-LLM reporting layer for:

- run summaries
- archetype changes
- break points
- recovery windows
- top divergent dimensions
- percentile band widening / narrowing

This should produce JSON-first outputs that are easy to render in CLI/web/report pipelines.

### 6. Optional network/entity simulation

Status: planned

Add a separate crate for multi-entity simulation where:

- entities each have local state
- edges mediate influence or contagion
- local interaction feeds aggregate composure

This should stay optional so `composure-core` remains lightweight.

### 7. Tooling surfaces

Status: started

Add:

- a CLI runner
- artifact inspection commands
- comparison commands
- a minimal browser replay inspector

Current progress in repo:

- CLI artifact inspection for experiment bundles
- CLI artifact inspection for sweep execution results
- CLI artifact inspection for run summaries
- CLI artifact inspection for trajectory comparisons
- CLI comparison command for saved Monte Carlo result artifacts
- CLI summary extraction from saved Monte Carlo result artifacts
- CLI summary extraction from bundle run records

These should consume the experiment and comparison artifacts rather than inventing their own data formats.

### 8. Example packs

Status: planned

Add examples that prove domain generality:

- health recovery
- campaign fatigue/recovery
- supply-chain disruption
- portfolio stress/recovery
- readiness / wargaming

## Suggested implementation order

### Phase 1

- experiment artifact model
- trajectory comparison
- README/docs updates

### Phase 2

- sweep APIs
- deterministic summary/report APIs
- richer examples

### Phase 3

- calibration crate
- CLI
- browser inspector

### Phase 4

- optional network/entity crate
- domain adapters built on top of core artifacts

## What should stay out of core

The following may be valuable in downstream projects, but they should not become hard dependencies of `composure-core`:

- GraphRAG and graph stores
- persona generation
- LLM-controlled agents
- product-specific report agents
- social platform models
- app-specific workflow state machines

## Implementation notes

The first feature slices added to the repo should optimize for portability:

- serializable structs
- explicit validation
- clean error types
- stable helper constructors
- no new heavy dependencies unless clearly necessary

That keeps `composure-sim` useful as a shared systems primitive instead of a one-off application.
