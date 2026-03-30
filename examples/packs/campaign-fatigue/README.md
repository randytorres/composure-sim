# Campaign Fatigue Pack

This pack models audience response decay and recovery during a paid media campaign.

Dimension mapping:

- `z[0]`: awareness
- `z[1]`: engagement
- `z[2]`: conversion intent

Primary levers in the sweep:

- frequency cap
- creative refresh cadence
- promotional depth

Files:

- [`scenario.json`](/Users/randytorres/Projects/composure-sim/examples/packs/campaign-fatigue/scenario.json)
- [`experiment-spec.json`](/Users/randytorres/Projects/composure-sim/examples/packs/campaign-fatigue/experiment-spec.json)
- [`sweep-definition.json`](/Users/randytorres/Projects/composure-sim/examples/packs/campaign-fatigue/sweep-definition.json)
- [`observed-trajectory.json`](/Users/randytorres/Projects/composure-sim/examples/packs/campaign-fatigue/observed-trajectory.json)

Suggested workflow:

1. Treat the scenario as a baseline media plan.
2. Map the sweep parameters into campaign-specific interventions inside your `Simulator`.
3. Run a sweep to score candidate plans by end-of-flight conversion intent or area under the response curve.
4. Calibrate against observed weekly response to infer the most plausible fatigue settings.
5. Export sweep and calibration results as CSV/markdown for planning reviews.
