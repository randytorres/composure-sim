# composure-sim V2 engine roadmap

## Goal

`composure-sim` should evolve from a strong reusable simulation library into a
portable simulation runtime with deterministic counterfactual workflows.

The V2 goal is not to make the repo into a product-specific application. The
goal is to make the engine expressive enough that downstream tools can build:

- executable packs without custom Rust per domain
- adaptive state-dependent scenarios instead of fixed action scripts
- deterministic branch-based counterfactual analysis
- richer trajectory interpretation via regime transitions
- optional multi-entity and network simulation

## Product thesis in engine terms

The strongest next version of this repo is:

- not just a Monte Carlo engine
- not just an artifact model
- not just a browser inspector

It is a deterministic simulation workbench built on a stable artifact system.

That means:

1. scenarios and packs can be compiled and executed
2. state transitions can react to the evolving system
3. interventions can be branched and compared with matched seeds
4. analysis can explain not only outcomes, but phase changes
5. new problem classes can be modeled without bloating `composure-core`

## Design constraints

### Keep `composure-core` small

`composure-core` should remain responsible for:

- state model
- simulator trait
- Monte Carlo execution
- replay artifacts
- comparison
- summaries
- deterministic reports

It should not become:

- a DSL parser
- a rule engine
- a network topology framework
- a storage format kitchen sink
- a product-specific orchestration layer

### Keep everything deterministic and artifact-first

Every new feature should preserve:

- explicit seeds
- serializable inputs
- serializable outputs
- reproducible execution
- portable deterministic summaries

### Prefer optional crates over feature creep

New capabilities should be split by responsibility:

- `composure-runtime`
- `composure-sensitivity`
- `composure-network`
- `composure-artifacts`

The current repo already follows this pattern with `composure-calibration`.

## Proposed V2 crate layout

### `composure-core`

Keep and extend:

- `SimState`
- `Simulator`
- Monte Carlo
- replay
- experiment bundles
- sweep execution primitives
- comparison
- deterministic summaries and reports

Potential additions that still belong in core:

- counterfactual result artifact types
- regime/segment summary artifact types
- generic constraint violation artifact types

### `composure-runtime`

New crate for executable packs and dynamic scenario rules.

Owns:

- pack schemas
- pack validation
- pack compilation into an intermediate representation
- runtime rule engine
- generic DSL-driven simulator implementation
- scenario composition and branching helpers

Does not own:

- browser UI
- long-running services
- domain-specific business logic

### `composure-sensitivity`

Likely split the current sensitivity logic here over time.

Owns:

- current ranked parameter sensitivity summaries
- Sobol sensitivity
- Morris screening
- threshold map generation

### `composure-network`

New optional crate for multi-entity simulation.

Owns:

- node-local state orchestration
- edge propagation rules
- topology helpers
- network metrics
- aggregate trajectory/composure summaries

### `composure-artifacts`

Optional export and schema crate.

Owns:

- Arrow/Parquet export
- Protobuf schemas if needed
- artifact versioning helpers
- compatibility and migration helpers

### `composure-cli`

Evolves from artifact inspector into runtime entrypoint.

V2 commands should include:

- `run-pack`
- `sweep-pack`
- `calibrate-pack`
- `counterfactual-pack`
- `inspect-*`
- `export-*`

## V2 priority tracks

## 1. Declarative pack runtime

### Why it matters

This is the biggest unlock.

Today the repo has typed example packs, but they are input artifacts only.
V2 should make packs executable without requiring users to write a custom Rust
simulator for every trial workflow.

### Scope

Add a pack DSL that can describe:

- dimensions
- initial state
- noise profiles
- decay/recovery dynamics
- intervention effects
- scenario timelines
- event triggers
- constraints
- objective functions

### Recommended architecture

Use a three-layer model:

1. source schema
2. compiled intermediate representation
3. deterministic runtime evaluator

Do not interpret arbitrary JSON ad hoc at each step. Compile once, then run.

### Proposed crate API

```rust
pub struct PackDefinition { /* serializable source shape */ }
pub struct CompiledPack { /* validated IR */ }

pub fn compile_pack(pack: &PackDefinition) -> Result<CompiledPack, PackError>;
pub fn run_pack(pack: &CompiledPack, config: &RunConfig) -> Result<ExperimentBundle, RuntimeError>;
```

### CLI surface

```text
composure run-pack <path>
composure sweep-pack <path>
composure calibrate-pack <path>
```

### Engineering risks

- overdesigning the DSL before enough real pack usage
- mixing parsing and execution logic
- making extension points too implicit

## 2. Conditional event system

### Why it matters

Current scenarios are mostly fixed sequences. Real systems are reactive.

This feature makes simulations adaptive while staying deterministic under a
fixed seed and rule set.

### Scope

Support:

- threshold-based triggers
- delayed effects
- cooldown windows
- guard conditions
- cascading actions
- deterministic priority ordering

### Proposed model

```rust
pub struct EventRule {
    pub id: String,
    pub trigger: Trigger,
    pub guard: Option<Guard>,
    pub effect: Effect,
    pub cooldown_steps: Option<usize>,
    pub delay_steps: usize,
    pub priority: i32,
}
```

### Runtime requirements

- rules must evaluate in a stable order
- multiple triggered effects in the same step need explicit resolution rules
- rule firings should be recorded into replay artifacts

### Output impact

Replay and experiment artifacts should expose:

- triggered rule ID
- fire step
- scheduled application step
- skipped/blocked reason if applicable

## 3. Counterfactual analysis

### Why it matters

This is likely the most valuable decision-support feature that can be built on
top of the current comparison/reporting stack.

### Scope

Add a deterministic branching workflow:

- choose a branch point
- clone the run state and RNG path at that step
- apply baseline and candidate actions from the same starting point
- emit matched-seed branch outputs

### Proposed artifacts

```rust
pub struct CounterfactualBranch {
    pub branch_id: String,
    pub branch_from_t: usize,
    pub intervention_label: String,
    pub outcome: ExperimentOutcome,
    pub summary: RunSummary,
}

pub struct CounterfactualResult {
    pub baseline: CounterfactualBranch,
    pub candidate: CounterfactualBranch,
    pub comparison: TrajectoryComparison,
    pub report: DeterministicReport,
}
```

### API shape

```rust
pub fn run_counterfactual<S: Simulator>(
    simulator: &S,
    state_at_branch: &SimState,
    baseline_actions: &[Action],
    candidate_actions: &[Action],
    config: &MonteCarloConfig,
) -> Result<CounterfactualResult, CounterfactualError>;
```

### Why this should come before network simulation

It is lower cost, compounds existing comparison/reporting features, and creates
an immediately differentiated workflow for intervention analysis.

## 4. Regime detection

### Why it matters

Current archetypes summarize whole trajectories. Many systems have multiple
phases with distinct behavior.

### Scope

Add:

- changepoint detection
- segmented archetype classification
- regime transition summaries
- optional Monte Carlo regime transition probabilities

### Proposed artifacts

```rust
pub struct RegimeSegment {
    pub start_t: usize,
    pub end_t: usize,
    pub archetype: Archetype,
}

pub struct RegimeReport {
    pub segments: Vec<RegimeSegment>,
    pub transition_steps: Vec<usize>,
}
```

### Suggested implementation order

1. single-trajectory changepoint detection
2. segmented archetype classification
3. path-aggregated transition analysis for Monte Carlo

## 5. Multi-entity network simulation

### Why it matters

This opens entirely new problem classes without forcing them into single-vector
state semantics.

### Scope

Add:

- node-local `SimState`
- edge-mediated propagation
- delayed/attenuated influence
- topology generators
- network fragility and cascade metrics
- aggregate composure over the network

### Crate boundaries

This belongs in `composure-network`, not `composure-core`.

### Proposed top-level API

```rust
pub fn run_network_monte_carlo<N: NetworkSimulator>(
    simulator: &N,
    graph: &NetworkDefinition,
    config: &NetworkRunConfig,
) -> Result<NetworkMonteCarloResult, NetworkError>;
```

### Output requirements

Artifacts should support:

- per-node summaries
- per-cluster summaries
- aggregate network summaries
- propagation/cascade reports

## 6. Constraints and invariants

### Why it matters

The engine should be able to enforce domain-independent structural rules
without embedding domain business logic.

### Scope

Support:

- bounds
- conservation relationships
- coupling constraints
- violation counts
- violation events in replay

### Design rule

Constraints should be attached to packs/runtime configuration, not hard-coded
into `SimState`.

## 7. Advanced calibration and sensitivity

These are important, but they are not the first unlocks.

### Bayesian calibration

Add an alternative calibration strategy:

- brute-force sweep remains the baseline
- Bayesian optimization becomes an optional search mode
- search config and surrogate settings must be explicit and serializable

### Sobol and Morris sensitivity

Add richer global sensitivity options:

- first-order Sobol indices
- total-order Sobol indices
- Morris elementary effects for cheaper screening

These should fit under a shared `SensitivityReport` family instead of creating
entirely separate downstream formats.

## 8. Scenario composition

### Scope

Add:

- scenario templates
- chained scenarios
- branch-on-condition scenario trees
- scenario parameters that can be swept like model parameters

### Dependency order

This should come after:

- pack runtime
- event system
- constraints

Otherwise composition semantics will be unstable.

## 9. Export protocol upgrades

### Scope

Add optional pipeline-oriented exports:

- Arrow/Parquet for Monte Carlo paths and sweep samples
- versioned binary schemas if JSON size becomes a practical problem
- streaming sweep/calibration output for long-running execution

### Priority

This is useful infrastructure, but not the main differentiator. It should
follow the execution model, not lead it.

## Recommended implementation sequence

### Phase 1: executable packs

- create `composure-runtime`
- define pack source schema
- define compiled IR
- add pack validation and diagnostics
- add `run-pack` CLI support

### Phase 2: adaptive execution

- add conditional event system
- record rule firings into replay
- add constraints/invariants
- add scenario composition primitives

### Phase 3: branch-aware analysis

- add counterfactual artifact model
- add counterfactual execution APIs
- add regime detection
- add deterministic report integration for branch outputs

### Phase 4: advanced exploration

- split or expand sensitivity into `composure-sensitivity`
- add Sobol/Morris
- add Bayesian calibration mode
- add threshold maps

### Phase 5: new problem classes

- create `composure-network`
- implement node/edge propagation runtime
- add network-level artifact types and summaries

### Phase 6: artifact-scale interoperability

- add Arrow/Parquet export
- add streaming output
- add versioned artifact compatibility helpers

## What success looks like

A strong V2 should make the following possible:

1. A user can define and execute a pack without writing Rust.
2. Scenarios can react to the simulation state instead of only following a
   fixed timeline.
3. A user can branch a run at step `t` and get a deterministic causal delta.
4. A user can see where a trajectory changed regime, not only how it ended.
5. Multi-entity problems can be modeled in an optional crate without bloating
   core.

## Anti-goals

The following should still stay out of the engine:

- LLM orchestration
- product-specific workflow state
- app servers
- domain-specific UIs
- knowledge graphs
- persona systems

Those may exist in downstream tools, but they should consume the engine's
artifacts rather than reshape the engine around themselves.
