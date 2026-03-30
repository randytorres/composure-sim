# Domain Packs

These packs are typed input examples that show how the same composure artifact
pipeline can be applied across different domains without changing `composure-core`.

Each pack includes:

- `pack.json`
- `scenario.json`
- `experiment-spec.json`
- `counterfactual-definition.json`
- `sweep-definition.json`
- `observed-trajectory.json`
- `README.md`

The packs now include a manifest that can be compiled and validated with the
CLI. The checked-in examples also include a constrained built-in linear runtime
model, so they can be executed directly as reference packs.

Available packs:

- [`health-recovery`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/README.md)
- [`campaign-fatigue`](/Users/randytorres/Projects/composure-sim/examples/packs/campaign-fatigue/README.md)
- [`supply-chain-disruption`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/README.md)

Typical flow for any pack:

1. Run `composure validate-pack path/to/pack.json`.
2. Run `composure inspect-pack path/to/pack.json`.
3. Run `composure run-pack path/to/pack.json` to emit a baseline `ExperimentBundle`.
   This only requires `scenario.json`, `experiment-spec.json`, and
   `runtime_model` in `pack.json`.
   `scenario.json` can also include `conditional_actions`, and `run-pack` will
   execute them through the same scenario-aware Monte Carlo path used in core.
4. Run `composure inspect-pack-counterfactual path/to/pack.json`.
5. Run `composure run-pack-counterfactual path/to/pack.json --output /tmp/counterfactual-result.json`.
6. Run `composure inspect-counterfactual-result /tmp/counterfactual-result.json`.
7. Load `experiment-spec.json` into `ExperimentSpec`.
8. Load `sweep-definition.json` into `SweepDefinition`.
9. Load `observed-trajectory.json` into `ObservedTrajectory`.
10. Map each `SweepCase` into an `ExperimentParameterSet` inside your domain adapter.
11. Run `execute_experiment_sweep` and/or `calibrate_experiment`.
12. Inspect the resulting artifacts with the `composure` CLI or the browser inspector.

The downstream artifact tooling is shared:

- `ExperimentBundle`
- `SweepExecutionResult`
- `RunSummary`
- `TrajectoryComparison`
- `DeterministicReport`
- `CalibrationResult`
