# Supply Chain Disruption Pack

This pack models disruption recovery for a multi-supplier fulfillment network.

Dimension mapping:

- `z[0]`: fill rate
- `z[1]`: inventory health
- `z[2]`: supplier confidence

Primary levers in the sweep:

- safety stock weeks
- expedited freight budget
- alternate supplier activation

Files:

- [`scenario.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/scenario.json)
- [`experiment-spec.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/experiment-spec.json)
- [`sweep-definition.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/sweep-definition.json)
- [`observed-trajectory.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/observed-trajectory.json)

Suggested workflow:

1. Load the pack inputs and bind them to a supply-chain-specific simulator.
2. Run a sweep to score policy options by recovery speed and end-state fill rate.
3. Calibrate the policy space against the observed disruption trajectory.
4. Export bundle, sweep, and calibration summaries into markdown for operations review.
