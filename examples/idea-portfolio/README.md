# Idea Portfolio Example

This example answers: "Can we run a big list of business ideas through the
simulator?" Yes. Use `draft-idea-portfolio` for the MiroFish-style natural
language front door, then inspect or edit the JSON before running.

Draft an editable portfolio from the brief:

```bash
cargo run -p composure-cli -- draft-idea-portfolio \
  examples/idea-portfolio/ideas.md \
  --output /tmp/idea-portfolio.json
```

Run the portfolio through the default synthetic consumer AI panel:

```bash
cargo run -p composure-cli -- run-idea-portfolio \
  /tmp/idea-portfolio.json \
  --output /tmp/idea-portfolio-result.json
```

Export a decision memo:

```bash
cargo run -p composure-cli -- export-idea-portfolio-markdown \
  /tmp/idea-portfolio-result.json \
  --output /tmp/idea-portfolio-report.md
```

The result includes:

- overall idea leaderboard
- metric scores for market pull, viral loop, build speed, AI unlock, pay, retention, distribution, founder fit, and risk resilience
- winners for each launch scenario
- strongest and weakest synthetic audience segments
- sampled synthetic people with top idea, runner-up, adoption, share, and pay probabilities

This is not a replacement for live market data. It is a deterministic,
artifact-first way to reduce a giant creative scan into ranked hypotheses and
clear next tests.
