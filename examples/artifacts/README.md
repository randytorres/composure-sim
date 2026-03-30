# Artifact examples

These files are small JSON examples for the `composure` CLI:

```bash
cargo run -p composure-cli -- inspect-summary examples/artifacts/run-summary.json
cargo run -p composure-cli -- inspect-report examples/artifacts/report.json
cargo run -p composure-cli -- export-report-markdown examples/artifacts/report.json
cargo run -p composure-cli -- summarize-monte-carlo examples/artifacts/candidate-monte-carlo.json
cargo run -p composure-cli -- summarize-monte-carlo \
  examples/artifacts/baseline-monte-carlo.json \
  --output /tmp/run-summary.json
cargo run -p composure-cli -- build-report \
  examples/artifacts/baseline-run-summary.json \
  examples/artifacts/run-summary.json \
  --comparison examples/artifacts/comparison.json
cargo run -p composure-cli -- build-report \
  examples/artifacts/baseline-run-summary.json \
  examples/artifacts/run-summary.json \
  --comparison examples/artifacts/comparison.json \
  --output /tmp/report.json
cargo run -p composure-cli -- inspect-bundle examples/artifacts/experiment-bundle.json
cargo run -p composure-cli -- summarize-bundle-run \
  examples/artifacts/experiment-bundle-with-output.json \
  run-1
cargo run -p composure-cli -- summarize-bundle-run \
  examples/artifacts/experiment-bundle-with-output.json \
  run-1 \
  --output /tmp/bundle-run-summary.json
cargo run -p composure-cli -- inspect-sweep examples/artifacts/sweep-result.json
cargo run -p composure-cli -- export-sweep-samples examples/artifacts/sweep-result.json
cargo run -p composure-cli -- inspect-compare examples/artifacts/comparison.json
cargo run -p composure-cli -- inspect-calibration examples/artifacts/calibration-result.json
cargo run -p composure-cli -- export-calibration-candidates examples/artifacts/calibration-result.json
cargo run -p composure-cli -- compare-monte-carlo \
  examples/artifacts/baseline-monte-carlo.json \
  examples/artifacts/candidate-monte-carlo.json
cargo run -p composure-cli -- compare-monte-carlo \
  examples/artifacts/baseline-monte-carlo.json \
  examples/artifacts/candidate-monte-carlo.json \
  --divergence-threshold 0.02 \
  --sustained-steps 2 \
  --output /tmp/comparison.json
```

If you want to save the comparison command output as an artifact:

```bash
cargo run -p composure-cli -- compare-monte-carlo \
  examples/artifacts/baseline-monte-carlo.json \
  examples/artifacts/candidate-monte-carlo.json \
  > /tmp/comparison.json
```

Generated export examples are also checked in:

```text
examples/artifacts/report.md
examples/artifacts/sweep-samples.csv
examples/artifacts/calibration-candidates.csv
```

If you want a browser view instead of CLI output, serve the repo root and open
`/examples/browser-inspector/`.
