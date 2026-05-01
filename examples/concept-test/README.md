# Concept Test Example

This example runs a synthetic-population concept test. It is the first small
slice of the longer-term decision-lab workflow: define segments, variants, a
scenario, touchpoints, and success metrics, then run the variants against a
deterministic population.

Draft an editable scenario matrix from a JSON brief:

```bash
cargo run -p composure-cli -- draft-concept-test-matrix \
  examples/concept-test/draft.json \
  --output /tmp/concept-test-drafted-matrix.json
```

Plain text briefs also work:

```bash
cargo run -p composure-cli -- draft-concept-test-matrix \
  examples/concept-test/draft.txt \
  --output /tmp/concept-test-gameplay-matrix.json
```

The draft command does not run the simulation. It creates an auditable
`ConceptTestMatrixRequest` first, so you can edit segments, variants, presets,
population size, and observed outcomes before spending a run.

Run it with:

```bash
cargo run -p composure-cli -- run-concept-test examples/concept-test/request.json
```

Write the JSON artifact to disk:

```bash
cargo run -p composure-cli -- run-concept-test examples/concept-test/request.json \
  --output /tmp/concept-test-result.json
```

Export a decision-friendly report:

```bash
cargo run -p composure-cli -- export-concept-test-report-markdown \
  /tmp/concept-test-result.json \
  --output /tmp/concept-test-report.md
```

Compare two concept-test artifacts:

```bash
cargo run -p composure-cli -- run-concept-test examples/concept-test/request-alt.json \
  --output /tmp/concept-test-alt-result.json

cargo run -p composure-cli -- compare-concept-tests \
  /tmp/concept-test-result.json \
  /tmp/concept-test-alt-result.json \
  --output /tmp/concept-test-compare.json

cargo run -p composure-cli -- export-concept-test-compare-markdown \
  /tmp/concept-test-compare.json \
  --output /tmp/concept-test-compare.md
```

Run a scenario matrix:

```bash
cargo run -p composure-cli -- run-concept-test-matrix \
  examples/concept-test/matrix.json \
  --output /tmp/concept-test-matrix.json

cargo run -p composure-cli -- export-concept-test-matrix-markdown \
  /tmp/concept-test-matrix.json \
  --output /tmp/concept-test-matrix.md
```

Run the drafted matrix:

```bash
cargo run -p composure-cli -- run-concept-test-matrix \
  /tmp/concept-test-drafted-matrix.json \
  --output /tmp/concept-test-drafted-result.json
```

Matrix cases can either provide a full `scenario` or use a named `preset`.
Current presets include `cheap_acquisition`, `trust_collapse`,
`retention_loop`, `referral_loop`, `pricing_pressure`,
`onboarding_friction`, `procurement_skepticism`, and `gameplay_fatigue`.

The result includes:

- ranked variants
- segment-level winners and weak spots
- touchpoint-level timeline performance
- sampled synthetic individuals
- funnel rates for click, signup, activation, retention, and referral
- calibration notes when observed outcomes are attached
- touchpoint calibration notes when observed outcomes include `touchpoint_id`
- matrix rollups showing robust and fragile variants across scenario cases
