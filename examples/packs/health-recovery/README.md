# Health Recovery Pack

This pack models a recovery program for an athlete returning from overload.

Dimension mapping:

- `z[0]`: sleep quality
- `z[1]`: tissue readiness
- `z[2]`: training capacity

Primary levers in the sweep:

- recovery protocol intensity
- mobility minutes
- deload length

Files:

- [`scenario.json`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/scenario.json)
- [`experiment-spec.json`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/experiment-spec.json)
- [`sweep-definition.json`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/sweep-definition.json)
- [`observed-trajectory.json`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/observed-trajectory.json)

Suggested workflow:

1. Deserialize the spec and sweep into `ExperimentSpec` and `SweepDefinition`.
2. Implement a health-specific `Simulator` that interprets the three dimensions.
3. Use `execute_experiment_sweep` to rank intervention mixes by end-state recovery.
4. Use `calibrate_experiment` against the observed trajectory to find the closest parameter set.
5. Build a `DeterministicReport` from baseline and candidate summaries, then inspect it with the CLI.
