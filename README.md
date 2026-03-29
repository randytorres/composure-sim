# composure-sim

Domain-agnostic simulation engine: Monte Carlo, Composure Curve, event-sourced replay, trajectory comparison, experiment artifacts, sweep execution and sensitivity analysis, and deterministic run summaries.

Use this to simulate any system that degrades and recovers under stress — health protocols, biological organisms, marketing campaigns, wargames, whatever.

## Crates

| Crate | Purpose |
|---|---|
| `composure-core` | Core library: SimState, Simulator trait, Monte Carlo (rayon parallel), Composure Curve (archetype classification), event-sourced replay, comparison, experiment bundles, execution, sweep runner, sensitivity, run summaries |
| `composure-py` | PyO3 Python bindings |
| `composure-wasm` | WASM bindings for browser |

Roadmap and next-step feature plan: [docs/roadmap.md](/Users/randytorres/Projects/composure-sim/docs/roadmap.md)

## Core Concepts

### SimState (`z_t`, `m_t`, `u_t`)

Every simulation tracks three vectors per time step:

- **`z`** — Current functional state (health, performance, viability, etc.)
- **`m`** — Accumulated memory (damage, adaptation, fatigue, hysteresis)
- **`u`** — Uncertainty (confidence in predictions; used for OOD gating)

The library doesn't know what the dimensions mean. You define that via the `Simulator` trait.

### Simulator Trait

Implement one method: `step(state, action, rng) -> next_state`. That's your domain logic. The library handles everything else (Monte Carlo orchestration, composure analysis, replay).

```rust
use composure_core::{Simulator, SimState, Action};

struct MySimulator;

impl Simulator for MySimulator {
    fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
        // Your domain-specific transition function
        todo!()
    }
}
```

### Monte Carlo

Runs N parallel paths (rayon) with seeded determinism:

```rust
use composure_core::{run_monte_carlo, MonteCarloConfig, SimState};

let config = MonteCarloConfig::with_seed(10_000, 180, 42);
let result = run_monte_carlo_checked(&my_sim, &initial_state, &actions, &config, false)?;

println!("Mean trajectory: {:?}", result.mean_trajectory);
println!("P10-P90 bands: {:?}", result.percentiles);
```

### Composure Curve

Analyzes a trajectory into degradation/recovery metrics and classifies an archetype:

| Archetype | Pattern |
|---|---|
| Steady | Consistent. Low variance, stable trend. |
| Cliff Faller | Strong start, sudden collapse. |
| Phoenix | Fast drop, strong recovery. |
| Oscillator | Alternating highs and lows. |
| Plateau | Flat. Neither improving nor degrading. |
| Surge | Improving under pressure. |

```rust
use composure_core::{analyze_composure_checked, classify_archetype};

let curve = analyze_composure_checked(&health_indices, 0.3)?;
println!("Archetype: {}", curve.archetype.label());
println!("Slope: {}", curve.metrics.slope);
println!("Recovery half-life: {:?}", curve.metrics.recovery_half_life);
```

### Event-Sourced Replay

Deterministic, replayable simulation runs with full state snapshots and event logs:

```rust
use composure_core::replay::ReplayBuilder;
use composure_core::EventKind;

let mut replay = ReplayBuilder::new("run-001", seed);
// During simulation:
replay.snapshot(&state, health_index);
replay.emit(t, EventKind::ActionApplied, None);
// After:
let run = replay.finish(final_state);
// run.event_log, run.state_snapshots are serializable
```

### Trajectory Comparison

Compare a baseline and candidate trajectory with divergence detection:

```rust
use composure_core::{compare_trajectories, ComparisonConfig};

let comparison = compare_trajectories(
    &baseline_health,
    &candidate_health,
    &ComparisonConfig::default(),
)?;

println!("Mean delta: {}", comparison.metrics.mean_delta);
println!("First divergence: {:?}", comparison.divergence);
```

### Experiment Bundles

Store reusable experiment specs, variants, and run artifacts:

```rust
use composure_core::{ExperimentBundle, ExperimentSpec, Scenario, SimState};

let scenario = Scenario::new("baseline", "Baseline", SimState::zeros(3), 100);
let spec = ExperimentSpec::new("exp-001", "Recovery Sweep", scenario);
let bundle = ExperimentBundle::new(spec);

println!("Bundle ready with {} runs", bundle.runs.len());
```

### Experiment Execution

Execute a validated experiment parameter set and record portable outputs:

```rust
use composure_core::{
    execute_parameter_set, ExperimentExecutionConfig, ExperimentParameterSet,
    MonteCarloConfig, Scenario, SimState,
};

let mut parameter_set =
    ExperimentParameterSet::new("variant-a", "Variant A", Scenario::new("baseline", "Baseline", SimState::zeros(3), 100));
parameter_set.monte_carlo = Some(MonteCarloConfig::with_seed(1_000, 100, 42));

let run = execute_parameter_set(
    "run-variant-a",
    &my_sim,
    &parameter_set,
    &ExperimentExecutionConfig::default(),
)?;

println!("Run status: {:?}", run.status);
```

### Sensitivity Analysis

Define sweep cases and rank parameter influence against a scalar objective:

```rust
use composure_core::{
    analyze_sensitivity, generate_sweep_cases, ParameterValue, SensitivityConfig,
    SweepDefinition, SweepParameter, SweepSample,
};
use std::collections::BTreeMap;

let mut sweep = SweepDefinition::new("dose-sweep", "Dose Sweep");
sweep.parameters.push(SweepParameter {
    name: "dose".into(),
    values: vec![ParameterValue::Int(1), ParameterValue::Int(2), ParameterValue::Int(3)],
});

let cases = generate_sweep_cases(&sweep)?;
let samples: Vec<SweepSample> = cases
    .into_iter()
    .map(|case| SweepSample {
        case_id: case.case_id,
        objective: 0.0,
        parameters: case.parameters,
        metadata: None,
    })
    .collect();

let report = analyze_sensitivity(&samples, &SensitivityConfig::default())?;
println!("Top parameter: {}", report.rankings[0].parameter);
```

### Sweep Runner

Execute a generated sweep end-to-end by mapping each `SweepCase` into an `ExperimentParameterSet`
and then extracting a scalar objective from the resulting `RunSummary`:

```rust
use composure_core::{
    execute_sweep, ExperimentParameterSet, MonteCarloConfig, ParameterValue, Scenario,
    SimState, SweepDefinition, SweepParameter, SweepRunnerConfig,
};

let mut sweep = SweepDefinition::new("dose-sweep", "Dose Sweep");
sweep.parameters.push(SweepParameter {
    name: "dose".into(),
    values: vec![ParameterValue::Int(1), ParameterValue::Int(2), ParameterValue::Int(3)],
});

let results = execute_sweep(
    &my_sim,
    &sweep,
    &SweepRunnerConfig::default(),
    |case| {
        let dose = match case.parameters.get("dose") {
            Some(ParameterValue::Int(value)) => *value as f64,
            _ => return Err("dose must be present".into()),
        };

        let mut parameter_set = ExperimentParameterSet::new(
            format!("ps-{}", case.case_id),
            format!("Case {}", case.case_id),
            Scenario::new("baseline", "Baseline", SimState::zeros(3), 100),
        );
        parameter_set.monte_carlo = Some(MonteCarloConfig::with_seed(1_000, 100, 42));
        parameter_set.scenario.actions.push(composure_core::Action {
            dimension: Some(0),
            magnitude: dose,
            action_type: composure_core::ActionType::Intervention,
            metadata: None,
        });
        Ok(parameter_set)
    },
    |_, _, _, summary| {
        summary
            .monte_carlo
            .as_ref()
            .and_then(|monte_carlo| monte_carlo.end)
            .ok_or_else(|| "missing final mean".into())
    },
)?;

println!("Executed {} cases", results.executed_cases.len());
println!(
    "Top sensitivity: {}",
    results.sensitivity.as_ref().unwrap().rankings[0].parameter
);
```

For experiment-driven workflows, use `execute_experiment_sweep` to inherit
`ExperimentSpec.default_monte_carlo`, persist successful cases into an `ExperimentBundle`,
and optionally continue past per-case failures:

```rust
use composure_core::{
    execute_experiment_sweep, ExperimentSpec, SweepFailureMode, SweepRunnerConfig,
};

let mut spec = ExperimentSpec::new(
    "exp-001",
    "Recovery Sweep",
    Scenario::new("baseline", "Baseline", SimState::zeros(3), 100),
);
spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(1_000, 100, 42));

let results = execute_experiment_sweep(
    &my_sim,
    &spec,
    &sweep,
    &SweepRunnerConfig {
        failure_mode: SweepFailureMode::Continue,
        ..SweepRunnerConfig::default()
    },
    |spec, case| {
        let mut parameter_set = ExperimentParameterSet::new(
            format!("ps-{}", case.case_id),
            format!("Case {}", case.case_id),
            spec.scenario.clone(),
        );
        parameter_set.scenario.id = format!("scenario-{}", case.case_id);
        parameter_set.scenario.name = format!("Scenario {}", case.case_id);
        Ok(parameter_set)
    },
    |_, _, _, summary| Ok(summary.monte_carlo.as_ref().and_then(|m| m.end)),
)?;

println!("Bundle runs: {}", results.bundle.as_ref().unwrap().runs.len());
println!("Case failures: {}", results.failures.len());
```

### Run Summaries

Extract compact deterministic metrics for reports and sweep objectives:

```rust
use composure_core::{summarize_composure, summarize_monte_carlo};

let mc_summary = summarize_monte_carlo(&result);
let curve_summary = summarize_composure(&curve);

println!("Final mean: {:?}", mc_summary.end);
println!("Archetype: {:?}", curve_summary.archetype);
```

## Python Usage

```python
import composure_py as composure

result = composure.run_monte_carlo(
    initial_z=[0.5, 0.7, 0.6],
    initial_m=[0.0, 0.0, 0.0],
    initial_u=[0.5, 0.5, 0.5],
    num_paths=10000,
    time_steps=180,
    seed=42,
)

curve = composure.analyze_composure(values=result["mean_trajectory"], threshold=0.3)
print(curve["archetype"])
```

## WASM Usage

```typescript
import init, { run_monte_carlo, classify_archetype } from 'composure-wasm';

await init();
const result = JSON.parse(run_monte_carlo(
  new Float64Array([0.5, 0.7, 0.6]),
  new Float64Array([0.0, 0.0, 0.0]),
  new Float64Array([0.5, 0.5, 0.5]),
  1000, 90, 42n
));
```

## Build

```bash
# Core library
cargo build --release

# Run tests
cargo test

# Python bindings (requires maturin)
cd crates/composure-py && maturin develop --features python-module

# WASM (requires wasm-pack)
wasm-pack build --target web crates/composure-wasm
```

## Lineage

Patterns extracted from:
- `forge-sim` (marketing-os) — Monte Carlo runner, seeded deterministic engine
- `wargame-engine` — Event-sourced replay, deterministic seeding, scenario management
- `wargame-py` — PyO3 Python bindings pattern
- `sim` ProfileBuilder.ts — Composure Curve archetype classification (ported TS → Rust)
