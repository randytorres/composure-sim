# Domain Packs

These packs are typed input examples that show how the same composure artifact
pipeline can be applied across different domains without changing `composure-core`.

Each pack includes:

- `scenario.json`
- `experiment-spec.json`
- `sweep-definition.json`
- `observed-trajectory.json`
- `README.md`

The packs are intentionally input-first. They do not rely on a generic CLI
runner because the repo still expects the caller to provide a domain-specific
`Simulator` implementation.

Available packs:

- [`health-recovery`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/README.md)
- [`campaign-fatigue`](/Users/randytorres/Projects/composure-sim/examples/packs/campaign-fatigue/README.md)
- [`supply-chain-disruption`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/README.md)

Typical flow for any pack:

1. Load `experiment-spec.json` into `ExperimentSpec`.
2. Load `sweep-definition.json` into `SweepDefinition`.
3. Load `observed-trajectory.json` into `ObservedTrajectory`.
4. Map each `SweepCase` into an `ExperimentParameterSet` inside your domain adapter.
5. Run `execute_experiment_sweep` and/or `calibrate_experiment`.
6. Inspect the resulting artifacts with the `composure` CLI or the browser inspector.

The downstream artifact tooling is shared:

- `ExperimentBundle`
- `SweepExecutionResult`
- `RunSummary`
- `TrajectoryComparison`
- `DeterministicReport`
- `CalibrationResult`
