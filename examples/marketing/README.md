# Marketing Adapter Example

This example shows the request shapes accepted by the marketing adapter.

Files:

- [`request.json`](/Users/randytorres/Projects/composure-sim/examples/marketing/request.json)
- [`request-v2.json`](/Users/randytorres/Projects/composure-sim/examples/marketing/request-v2.json)

Suggested workflow:

1. Run a one-shot V1 simulation with `cargo run -p composure-cli -- simulate-marketing examples/marketing/request.json`.
2. Run the richer V2 request with `cargo run -p composure-cli -- simulate-marketing-v2 examples/marketing/request-v2.json`.

The response shapes are designed for artifact-first local use so downstream
projects can keep their own scenarios and data while reusing the same engine.

The V2 request adds:

- persona-level weighting and breakdowns
- reusable scenario families such as `positioning`, `community_activation`, and `retention`
- richer scorecards that separate channel fit, trust, clarity, conversion intent, and shareability
