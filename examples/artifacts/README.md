# Artifact examples

These files are small JSON examples for the `composure` CLI:

```bash
cargo run -p composure-cli -- inspect-summary examples/artifacts/run-summary.json
cargo run -p composure-cli -- inspect-bundle examples/artifacts/experiment-bundle.json
cargo run -p composure-cli -- inspect-sweep examples/artifacts/sweep-result.json
cargo run -p composure-cli -- inspect-compare examples/artifacts/comparison.json
cargo run -p composure-cli -- compare-monte-carlo \
  examples/artifacts/baseline-monte-carlo.json \
  examples/artifacts/candidate-monte-carlo.json
```

If you want to save the comparison command output as an artifact:

```bash
cargo run -p composure-cli -- compare-monte-carlo \
  examples/artifacts/baseline-monte-carlo.json \
  examples/artifacts/candidate-monte-carlo.json \
  > /tmp/comparison.json
```
