# Marketing Adapter Example

This example shows the request shapes accepted by the marketing adapter.

Files:

- [`request.json`](/Users/randytorres/Projects/composure-sim/examples/marketing/request.json)
- [`request-v2.json`](/Users/randytorres/Projects/composure-sim/examples/marketing/request-v2.json)

Suggested workflow:

1. Run a one-shot V1 simulation with `cargo run -p composure-cli -- simulate-marketing examples/marketing/request.json`.
2. Run the richer V2 request with `cargo run -p composure-cli -- simulate-marketing-v2 examples/marketing/request-v2.json`.
   This path stays deterministic even though `request-v2.json` already includes assisted-run evaluator metadata, so `engine.model` can reflect the requested evaluator while `llm_analysis` remains `null`.
3. Run the assisted V2 flow with an OpenAI-compatible frontier model:
   `cargo run -p composure-cli -- simulate-marketing-v2-assisted examples/marketing/request-v2.json`
   You can override the request's evaluator metadata at run time with `--provider`, `--model`, or `--reasoning-effort`.
4. Save a V2 artifact and export the markdown report:
   `cargo run -p composure-cli -- simulate-marketing-v2-assisted examples/marketing/request-v2.json --output /tmp/marketing-v2.json`
   `cargo run -p composure-cli -- export-marketing-v2-report-markdown /tmp/marketing-v2.json`
5. Swap the provider or model without editing JSON:
   `cargo run -p composure-cli -- simulate-marketing-v2-assisted examples/marketing/request-v2.json --provider openai --model gpt-5.4 --reasoning-effort high`
6. Compare at least two assisted V2 scenarios and save a comparison artifact:
   `cp examples/marketing/request-v2.json /tmp/request-v2-alt.json`
   Edit `/tmp/request-v2-alt.json` before comparing it so the scenario name or inputs actually differ.
   `cargo run -p composure-cli -- compare-marketing-v2-assisted examples/marketing/request-v2.json /tmp/request-v2-alt.json --output /tmp/marketing-v2-compare.json`
7. Export the comparison artifact to markdown:
   `cargo run -p composure-cli -- export-marketing-v2-compare-markdown /tmp/marketing-v2-compare.json --output /tmp/marketing-v2-compare.md`

The response shapes are designed for artifact-first local use so downstream
projects can keep their own scenarios and data while reusing the same engine.

The V2 request adds:

- persona-level weighting and breakdowns
- reusable scenario families such as `landing_page`, `short_form_video`, `community_event`, `in_store_enablement`, and `private_relationship`
- richer scorecards that separate channel fit, trust, clarity, conversion intent, shareability, belonging, credibility, retention fit, and recommendation confidence
- optional sequence steps so a scenario can model hook, proof, objection handling, CTA, and follow-up instead of a single touch
- optional `evaluator` metadata so downstream repos can declare the provider/model/reasoning profile they want associated with the run
- optional `llm_assist` config so downstream repos can layer frontier-model judgment onto the deterministic output without replacing the underlying scores
- optional `observed_outcomes` contracts so downstream repos can attach real signup, activation, retention, and share data for calibration-aware notes

`compare-marketing-v2-assisted` reports cross-scenario metric deltas per
scenario aggregate scorecard in the comparison JSON:

- `metric_deltas[]` captures each scenario aggregate metric score plus `delta_vs_compare_average`
- `delta_vs_compare_average` is computed as `metric score - round(cross-scenario average)` for that metric label
- `strongest_positive_delta_metric` / `strongest_positive_delta_value` identify the biggest positive lift vs compare average
- `weakest_delta_metric` / `weakest_delta_value` identify the largest negative gap vs compare average

`export-marketing-v2-compare-markdown` turns that JSON artifact into a markdown
summary with leaderboard, scenario notes, strongest/weakest delta callouts, and
a "Metric deltas vs compare average" table for each scenario. The ranking itself
is still deterministic today; the assisted layer adds narrative context and
recommended experiments without replacing the deterministic ordering.

For assisted runs, the most important request fields are:

- `evaluator.provider`: chooses the Responses API target. `openai` uses `OPENAI_BASE_URL` or `https://api.openai.com/v1`; `cliproxyapi` uses `CLIPROXYAPI_BASE_URL`, falls back to `OPENAI_BASE_URL`, then defaults to `http://127.0.0.1:8317/v1`.
- `evaluator.model`: required whenever `llm_assist.enabled` is true. The assisted CLI command can supply it with `--model` if the request JSON leaves it blank.
- `evaluator.reasoning_effort`: optional Responses API reasoning level forwarded as `{ "reasoning": { "effort": ... } }`. Override it with `--reasoning-effort` when you want to compare the same scenario under a different depth setting.
- `llm_assist.enabled`: defaults to true when the block is omitted. Set it to false to keep the assisted command on a deterministic, no-network path while still emitting the V2 artifact shape.
- `llm_assist.analysis_goal` and `llm_assist.max_output_tokens`: optional tuning knobs for the narrative pass without changing the deterministic scorecard underneath.
- `observed_outcomes`: optional real-world signup, activation, retention, conversion, or share measurements that the deterministic and assisted layers can reference when generating calibration notes.

Proxy note: the CLI does not live-stream tokens to the terminal. It tries a
normal Responses API request first and only falls back to a buffered streaming
parse when the provider returns an empty final `output_text`.

Environment requirements for assisted runs:

- `openai` provider: set `OPENAI_API_KEY`.
- `cliproxyapi` provider: set `CLIPROXYAPI_API_KEY` or let it fall back to `OPENAI_API_KEY`. Some local proxy setups also accept a placeholder bearer token, so the exact requirement can depend on proxy policy.
