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

- [`pack.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/pack.json)
- [`scenario.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/scenario.json)
- [`experiment-spec.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/experiment-spec.json)
- [`counterfactual-definition.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/counterfactual-definition.json)
- [`sweep-definition.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/sweep-definition.json)
- [`observed-trajectory.json`](/Users/randytorres/Projects/composure-sim/examples/packs/supply-chain-disruption/observed-trajectory.json)

Suggested workflow:

1. Validate the manifest with `composure validate-pack examples/packs/supply-chain-disruption/pack.json`.
2. Inspect the compiled pack with `composure inspect-pack examples/packs/supply-chain-disruption/pack.json`.
3. Run the built-in reference runtime with `composure run-pack examples/packs/supply-chain-disruption/pack.json`.
4. Inspect the pack-managed branch plan with `composure inspect-pack-counterfactual examples/packs/supply-chain-disruption/pack.json`.
5. Execute the pack-managed branch plan with `composure run-pack-counterfactual examples/packs/supply-chain-disruption/pack.json --output /tmp/supply-chain-counterfactual-result.json`.
6. Inspect the saved branch result with `composure inspect-counterfactual-result /tmp/supply-chain-counterfactual-result.json`.
7. Replace the reference runtime with a supply-chain-specific simulator when you need richer propagation logic.
8. Run a sweep to score policy options by recovery speed and end-state fill rate.
9. Calibrate the policy space against the observed disruption trajectory.
10. Export bundle, sweep, and calibration summaries into markdown for operations review.
