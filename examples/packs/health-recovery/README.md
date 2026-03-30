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

- [`pack.json`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/pack.json)
- [`scenario.json`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/scenario.json)
- [`experiment-spec.json`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/experiment-spec.json)
- [`sweep-definition.json`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/sweep-definition.json)
- [`observed-trajectory.json`](/Users/randytorres/Projects/composure-sim/examples/packs/health-recovery/observed-trajectory.json)

Suggested workflow:

1. Validate the manifest with `composure validate-pack examples/packs/health-recovery/pack.json`.
2. Inspect the compiled pack with `composure inspect-pack examples/packs/health-recovery/pack.json`.
3. Run the built-in reference runtime with `composure run-pack examples/packs/health-recovery/pack.json`.
4. Replace the reference runtime with a health-specific `Simulator` when you need richer physiology logic.
5. Use `execute_experiment_sweep` to rank intervention mixes by end-state recovery.
6. Use `calibrate_experiment` against the observed trajectory to find the closest parameter set.
7. Build a `DeterministicReport` from baseline and candidate summaries, then inspect it with the CLI.
