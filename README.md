# composure-sim

Domain-agnostic simulation engine: Monte Carlo, Composure Curve, event-sourced replay.

Use this to simulate any system that degrades and recovers under stress — health protocols, biological organisms, marketing campaigns, wargames, whatever.

## Crates

| Crate | Purpose |
|---|---|
| `composure-core` | Core library: SimState, Simulator trait, Monte Carlo (rayon parallel), Composure Curve (archetype classification), event-sourced replay |
| `composure-py` | PyO3 Python bindings |
| `composure-wasm` | WASM bindings for browser |

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
