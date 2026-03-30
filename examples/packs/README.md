# Domain Packs

These packs are typed input examples that show how the same composure artifact
pipeline can be applied across different domains without changing `composure-core`.

Each pack includes:

- `pack.json`
- `scenario.json`
- `experiment-spec.json`
- `sweep-definition.json`
- `observed-trajectory.json`
- `README.md`

The packs now include a manifest that can be compiled and validated with the
CLI, but execution is still input-first. The repo still expects the caller to
provide a domain-specific `Simulator` implementation.

Available packs:

- [`health-recovery`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/README.md)
- [`campaign-fatigue`](/Users/randytorres/Projects/composure-sim/examples/packs/campaign-fatigue/README.md)
- [`supply-chain-disruption`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/README.md)

Typical flow for any pack:

1. Run `composure validate-pack path/to/pack.json`.
2. Run `composure inspect-pack path/to/pack.json`.
3. Load `experiment-spec.json` into `ExperimentSpec`.
4. Load `sweep-definition.json` into `SweepDefinition`.
5. Load `observed-trajectory.json` into `ObservedTrajectory`.
6. Map each `SweepCase` into an `ExperimentParameterSet` inside your domain adapter.
7. Run `execute_experiment_sweep` and/or `calibrate_experiment`.
8. Inspect the resulting artifacts with the `composure` CLI or the browser inspector.

The downstream artifact tooling is shared:

- `ExperimentBundle`
- `SweepExecutionResult`
- `RunSummary`
- `TrajectoryComparison`
- `DeterministicReport`
- `CalibrationResult`
