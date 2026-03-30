# composure-sim

Domain-agnostic simulation engine: Monte Carlo, Composure Curve, event-sourced replay, trajectory comparison, experiment artifacts, sweep execution and sensitivity analysis, and deterministic run summaries.

Use this to simulate any system that degrades and recovers under stress — health protocols, biological organisms, marketing campaigns, wargames, whatever.

## Crates

| Crate | Purpose |
|---|---|
| `composure-calibration` | Deterministic fitting utilities that score sweep candidates against observed trajectories |
| `composure-cli` | Minimal CLI for inspecting, summarizing, and comparing saved simulation artifacts |
| `composure-core` | Core library: SimState, Simulator trait, Monte Carlo (rayon parallel), Composure Curve (archetype classification), event-sourced replay, comparison, experiment bundles, execution, sweep runner, sensitivity, run summaries |
| `composure-py` | PyO3 Python bindings |
| `composure-runtime` | Pack manifests, cross-artifact validation, and runtime foundations for executable packs |
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

### Conditional Actions

Scenarios can now react to state with threshold-based conditional actions that
remain deterministic under the same seed and rule set:

```rust
use composure_core::{
    Action, ActionType, ConditionalActionRule, ConditionalTrigger, Scenario, SimState,
};

let mut scenario = Scenario::new("reactive", "Reactive", SimState::zeros(1), 6);
scenario.conditional_actions.push(ConditionalActionRule {
    id: "rescue".into(),
    trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.35 },
    action: Action {
        dimension: Some(0),
        magnitude: 0.25,
        action_type: ActionType::Intervention,
        metadata: None,
    },
    delay_steps: 1,
    cooldown_steps: 2,
    priority: 1,
    max_fires: Some(2),
});
```

The first slice supports threshold triggers, delay, cooldown, deterministic
priority ordering, and bounded firing counts. Rules evaluate after each step,
so `delay_steps: 0` means "apply on the next step"; crossing triggers compare
the previous state with the resulting state.

### Sensitivity Analysis

Define sweep cases and rank parameter influence against a scalar objective:

```rust
use composure_core::{
    analyze_sensitivity, generate_sweep_cases, ParameterValue, SensitivityConfig,
    SweepDefinition, SweepParameter, SweepSample,
};
use std::collections::BTreeMap;

let mut sweep = SweepDefinition::new("dose-sweep", "Dose Sweep");
sweep.strategy = composure_core::SweepStrategy::LatinHypercube;
sweep.sample_count = Some(12);
sweep.seed = Some(42);
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

`SweepStrategy::Grid` enumerates every combination. `SweepStrategy::Random` and
`SweepStrategy::LatinHypercube` generate deterministic sampled cases when `sample_count`
and an optional `seed` are set on `SweepDefinition`.

### CLI Artifacts

The `composure` CLI can inspect saved artifacts, transform them into summaries, and compare saved Monte Carlo results:

```bash
cargo run -p composure-cli -- validate-pack examples/packs/health-recovery/pack.json
cargo run -p composure-cli -- inspect-pack examples/packs/health-recovery/pack.json
cargo run -p composure-cli -- run-pack examples/packs/health-recovery/pack.json
cargo run -p composure-cli -- inspect-summary examples/artifacts/run-summary.json
cargo run -p composure-cli -- inspect-report examples/artifacts/report.json
cargo run -p composure-cli -- export-report-markdown examples/artifacts/report.json
cargo run -p composure-cli -- summarize-monte-carlo examples/artifacts/candidate-monte-carlo.json
cargo run -p composure-cli -- summarize-monte-carlo \
  examples/artifacts/baseline-monte-carlo.json \
  --output /tmp/run-summary.json
cargo run -p composure-cli -- build-report \
  examples/artifacts/baseline-run-summary.json \
  examples/artifacts/run-summary.json \
  --comparison examples/artifacts/comparison.json
cargo run -p composure-cli -- build-report \
  examples/artifacts/baseline-run-summary.json \
  examples/artifacts/run-summary.json \
  --comparison examples/artifacts/comparison.json \
  --output /tmp/report.json
cargo run -p composure-cli -- inspect-bundle examples/artifacts/experiment-bundle.json
cargo run -p composure-cli -- export-bundle-markdown examples/artifacts/experiment-bundle-with-output.json
cargo run -p composure-cli -- summarize-bundle-run \
  examples/artifacts/experiment-bundle-with-output.json \
  run-1
cargo run -p composure-cli -- summarize-bundle-run \
  examples/artifacts/experiment-bundle-with-output.json \
  run-1 \
  --output /tmp/bundle-run-summary.json
cargo run -p composure-cli -- inspect-sweep examples/artifacts/sweep-result.json
cargo run -p composure-cli -- export-sweep-summary-markdown examples/artifacts/sweep-result.json
cargo run -p composure-cli -- export-sweep-samples examples/artifacts/sweep-result.json
cargo run -p composure-cli -- export-sweep-samples-markdown examples/artifacts/sweep-result.json
cargo run -p composure-cli -- inspect-compare examples/artifacts/comparison.json
cargo run -p composure-cli -- inspect-calibration examples/artifacts/calibration-result.json
cargo run -p composure-cli -- export-calibration-candidates examples/artifacts/calibration-result.json
cargo run -p composure-cli -- export-calibration-candidates-markdown examples/artifacts/calibration-result.json
cargo run -p composure-cli -- compare-monte-carlo \
  examples/artifacts/baseline-monte-carlo.json \
  examples/artifacts/candidate-monte-carlo.json
cargo run -p composure-cli -- compare-monte-carlo \
  examples/artifacts/baseline-monte-carlo.json \
  examples/artifacts/candidate-monte-carlo.json \
  --divergence-threshold 0.02 \
  --sustained-steps 2 \
  --output /tmp/comparison.json
```

Sample artifacts live under [`examples/artifacts`](/Users/randytorres/Projects/composure-sim/examples/artifacts/README.md).

### Browser Inspector

A minimal static inspector is available under [`examples/browser-inspector`](/Users/randytorres/Projects/composure-sim/examples/browser-inspector/README.md):

```bash
python3 -m http.server 8000
```

Then open `http://127.0.0.1:8000/examples/browser-inspector/` and load saved JSON artifacts.

For an automated browser smoke pass that serves the repo locally and validates
the interactive inspector with Playwright CLI:

```bash
./scripts/browser-inspector-smoke.sh
```

### Domain Packs

Typed input packs for multiple domains live under [`examples/packs`](/Users/randytorres/Projects/composure-sim/examples/packs/README.md):

- [`health-recovery`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/README.md)
- [`campaign-fatigue`](/Users/randytorres/Projects/composure-sim/examples/packs/campaign-fatigue/README.md)
- [`supply-chain-disruption`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/README.md)

Each pack now includes a `pack.json` manifest that the CLI can validate and
compile into a pack summary, and the checked-in examples now include a
constrained built-in linear runtime model:

```bash
cargo run -p composure-cli -- validate-pack examples/packs/health-recovery/pack.json
cargo run -p composure-cli -- inspect-pack examples/packs/health-recovery/pack.json
cargo run -p composure-cli -- run-pack examples/packs/health-recovery/pack.json
```

The built-in runtime is intentionally simple. It is useful for executable
reference packs and early pipeline validation, while richer domain simulators
can still consume the same artifacts and downstream workflow.
`run-pack` only depends on the scenario, experiment spec, and runtime model, so
unused sweep or calibration inputs do not block execution.
Because packs load the shared `Scenario` shape directly, executable packs can
also use `conditional_actions` in `scenario.json` and the CLI will honor them
without extra runtime schema work.

### Deterministic Reports

Build a compact JSON-first report from two run summaries and an optional trajectory comparison:

```rust
use composure_core::{build_deterministic_report, compare_trajectories, ComparisonConfig};

let comparison = compare_trajectories(
    &baseline.mean_trajectory,
    &candidate.mean_trajectory,
    &ComparisonConfig::default(),
)?;

let report = build_deterministic_report(
    &baseline_summary,
    &candidate_summary,
    Some(&comparison),
);

println!("Archetype changed: {}", report.archetype_change.changed);
println!("Band change: {:?}", report.percentile_band_change.direction);
```

The CLI can also build the same artifact from saved summaries:

```bash
cargo run -p composure-cli -- build-report \
  examples/artifacts/baseline-run-summary.json \
  examples/artifacts/run-summary.json \
  --comparison examples/artifacts/comparison.json
```

For PRs and docs, the same artifact can be exported to markdown:

```bash
cargo run -p composure-cli -- export-report-markdown examples/artifacts/report.json
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

### Calibration / Fitting

Use `composure-calibration` to score sweep candidates against an observed trajectory:

```rust
use composure_calibration::{calibrate_experiment, CalibrationConfig, ObservedTrajectory};
use composure_core::{ExperimentParameterSet, ExperimentSpec, MonteCarloConfig, Scenario, SimState};

let mut spec = ExperimentSpec::new(
    "exp-001",
    "Dose Fit",
    Scenario::new("baseline", "Baseline", SimState::zeros(1), 4),
);
spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(100, 4, 42));

let observed = ObservedTrajectory::new(
    "obs-1",
    "Observed Recovery",
    vec![0.45, 0.52, 0.6, 0.7],
);

let result = calibrate_experiment(
    &my_sim,
    &spec,
    &observed,
    &sweep,
    &CalibrationConfig::default(),
    |spec, case| {
        let mut parameter_set = ExperimentParameterSet::new(
            format!("ps-{}", case.case_id),
            format!("Case {}", case.case_id),
            spec.scenario.clone(),
        );
        Ok(parameter_set)
    },
)?;

println!("Best case: {:?}", result.best_case_id);
println!("Best score: {:?}", result.best_score);
```

The checked-in example artifact can be inspected directly:

```bash
cargo run -p composure-cli -- inspect-calibration examples/artifacts/calibration-result.json
cargo run -p composure-cli -- export-calibration-candidates examples/artifacts/calibration-result.json
```

Bundle and sweep artifacts can also be exported as markdown summaries:

```bash
cargo run -p composure-cli -- export-bundle-markdown examples/artifacts/experiment-bundle-with-output.json
cargo run -p composure-cli -- export-sweep-summary-markdown examples/artifacts/sweep-result.json
```

Sweep samples can also be exported as a flat CSV or markdown table:

```bash
cargo run -p composure-cli -- export-sweep-samples examples/artifacts/sweep-result.json
cargo run -p composure-cli -- export-sweep-samples-markdown examples/artifacts/sweep-result.json
```

Calibration candidate rankings can be exported in CSV or markdown:

```bash
cargo run -p composure-cli -- export-calibration-candidates examples/artifacts/calibration-result.json
cargo run -p composure-cli -- export-calibration-candidates-markdown examples/artifacts/calibration-result.json
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

# Inspect saved artifacts
cargo run -p composure-cli -- inspect-sweep path/to/sweep-result.json
cargo run -p composure-cli -- inspect-bundle path/to/experiment-bundle.json
cargo run -p composure-cli -- inspect-report path/to/report.json
cargo run -p composure-cli -- inspect-compare path/to/comparison.json
cargo run -p composure-cli -- inspect-calibration path/to/calibration-result.json
cargo run -p composure-cli -- export-bundle-markdown path/to/bundle.json
cargo run -p composure-cli -- export-report-markdown path/to/report.json
cargo run -p composure-cli -- export-sweep-summary-markdown path/to/sweep-result.json
cargo run -p composure-cli -- export-sweep-samples path/to/sweep-result.json
cargo run -p composure-cli -- export-sweep-samples-markdown path/to/sweep-result.json
cargo run -p composure-cli -- export-calibration-candidates path/to/calibration-result.json
cargo run -p composure-cli -- export-calibration-candidates-markdown path/to/calibration-result.json
cargo run -p composure-cli -- summarize-monte-carlo path/to/monte-carlo.json
cargo run -p composure-cli -- summarize-bundle-run path/to/bundle.json run-id
cargo run -p composure-cli -- compare-monte-carlo baseline.json candidate.json
cargo run -p composure-cli -- build-report baseline-summary.json candidate-summary.json
cargo run -p composure-cli -- summarize-monte-carlo path/to/monte-carlo.json --output run-summary.json
cargo run -p composure-cli -- summarize-bundle-run path/to/bundle.json run-id --output run-summary.json
cargo run -p composure-cli -- compare-monte-carlo baseline.json candidate.json --output comparison.json
cargo run -p composure-cli -- build-report baseline-summary.json candidate-summary.json --comparison comparison.json --output report.json
cargo run -p composure-cli -- export-bundle-markdown path/to/bundle.json --output bundle.md
cargo run -p composure-cli -- export-report-markdown path/to/report.json --output report.md
cargo run -p composure-cli -- export-sweep-summary-markdown path/to/sweep-result.json --output sweep-summary.md
cargo run -p composure-cli -- export-sweep-samples path/to/sweep-result.json --output sweep-samples.csv
cargo run -p composure-cli -- export-sweep-samples-markdown path/to/sweep-result.json --output sweep-samples.md
cargo run -p composure-cli -- export-calibration-candidates path/to/calibration-result.json --output calibration-candidates.csv
cargo run -p composure-cli -- export-calibration-candidates-markdown path/to/calibration-result.json --output calibration-candidates.md

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
