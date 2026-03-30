#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-43123}"
URL="http://${HOST}:${PORT}/examples/browser-inspector/"
SESSION="${PLAYWRIGHT_CLI_SESSION:-composure-browser-smoke-$(date +%s)-${RANDOM:-0}}"
SERVER_LOG="$(mktemp -t composure-browser-inspector.XXXXXX.log)"
COUNTERFACTUAL_RESULT_PATH="$(mktemp "${TMPDIR:-/tmp}/composure-counterfactual-result.XXXXXX.json")"

export CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
export PWCLI="${PWCLI:-$CODEX_HOME/skills/playwright/scripts/playwright_cli.sh}"

if ! command -v npx >/dev/null 2>&1; then
  echo "npx is required to run the browser inspector smoke test." >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to serve the browser inspector." >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to poll the local browser inspector server." >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required to generate the counterfactual smoke artifact." >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to build the Playwright smoke payload." >&2
  exit 1
fi

if [[ ! -x "$PWCLI" ]]; then
  echo "Playwright wrapper not found at $PWCLI" >&2
  exit 1
fi

pwcli() {
  if [[ -n "$SESSION" ]]; then
    "$PWCLI" --session "$SESSION" "$@"
  else
    "$PWCLI" "$@"
  fi
}

cleanup() {
  local exit_code=$?

  pwcli close >/dev/null 2>&1 || true

  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi

  rm -rf "$ROOT_DIR/.playwright-cli"
  rm -f "$SERVER_LOG"
  rm -f "$COUNTERFACTUAL_RESULT_PATH"
  exit "$exit_code"
}

trap cleanup EXIT

python3 -m http.server "$PORT" --bind "$HOST" --directory "$ROOT_DIR" >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 50); do
  if curl -fsS "$URL" >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
done

if ! curl -fsS "$URL" >/dev/null 2>&1; then
  cat "$SERVER_LOG" >&2
  echo "Browser inspector server did not become ready at $URL" >&2
  exit 1
fi

cargo run -q -p composure-cli -- run-counterfactual \
  "$ROOT_DIR/examples/artifacts/counterfactual-definition.json" \
  --output "$COUNTERFACTUAL_RESULT_PATH"

find "${TMPDIR:-/tmp}/playwright-cli" -type s \
  \( -name "$SESSION" -o -name "$SESSION.sock" -o -name "$SESSION.*" \) \
  -delete 2>/dev/null || true

artifacts=(
  "$ROOT_DIR/examples/artifacts/run-summary.json"
  "$ROOT_DIR/examples/artifacts/counterfactual-definition.json"
  "$COUNTERFACTUAL_RESULT_PATH"
  "$ROOT_DIR/examples/artifacts/report.json"
  "$ROOT_DIR/examples/artifacts/calibration-result.json"
  "$ROOT_DIR/examples/artifacts/experiment-bundle-with-output.json"
  "$ROOT_DIR/examples/artifacts/sweep-result.json"
)

expected_text=(
  "RunSummary"
  "CounterfactualDefinition"
  "CounterfactualResult"
  "DeterministicReport"
  "CalibrationResult"
  "ExperimentBundle"
  "SweepExecutionResult"
  "Counterfactual Setup"
  "Counterfactual Overview"
  "Runtime Model"
  "Calibration Candidates"
  "Scored Samples"
  "Bundle Overview"
  "Sweep Overview"
  "Top Sensitivities"
)

files_json="$(printf '%s\n' "${artifacts[@]}" | jq -R . | jq -s -c .)"
expected_json="$(printf '%s\n' "${expected_text[@]}" | jq -R . | jq -s -c .)"

read -r -d '' smoke_code <<EOF || true
const files = ${files_json};
const expectedText = ${expected_json};

await page.locator("#file-input").setInputFiles(files);
await page.waitForFunction(
  (count) => document.querySelector("#status")?.textContent === "Loaded " + count + " files.",
  files.length,
);

const renderedText = await page.locator("#results").innerText();
for (const expected of expectedText) {
  if (!renderedText.includes(expected)) {
    throw new Error("Missing expected text: " + expected);
  }
}

const badgeText = await page.locator(".badge").allInnerTexts();
for (const expected of [
  "RunSummary",
  "CounterfactualDefinition",
  "CounterfactualResult",
  "DeterministicReport",
  "CalibrationResult",
  "ExperimentBundle",
  "SweepExecutionResult",
]) {
  if (!badgeText.includes(expected)) {
    throw new Error("Missing artifact badge: " + expected);
  }
}

const controlGroups = await page.locator(".table-controls").count();
if (controlGroups < 2) {
  throw new Error("Expected at least 2 interactive table control groups, found " + controlGroups);
}

const sweepSection = page.locator("section.section").filter({
  has: page.getByRole("heading", { name: "Scored Samples" }),
});
await sweepSection.getByLabel("Filter").fill("variant-a");
await page.waitForTimeout(100);
const sweepSummary = await sweepSection.locator(".table-summary").textContent();
if (sweepSummary !== "Showing 1 of 1 sample.") {
  throw new Error("Unexpected sweep summary: " + sweepSummary);
}
await sweepSection.getByLabel("Sort").selectOption("objective-desc");

const calibrationSection = page.locator("section.section").filter({
  has: page.getByRole("heading", { name: "Calibration Candidates" }),
});
await calibrationSection.getByLabel("Filter").fill("dose-sweep-1");
await page.waitForTimeout(100);
const calibrationSummary = await calibrationSection.locator(".table-summary").textContent();
if (calibrationSummary !== "Showing 1 of 1 candidate.") {
  throw new Error("Unexpected calibration summary: " + calibrationSummary);
}
await calibrationSection.getByLabel("Sort").selectOption("score-asc");
EOF

echo "Opening ${URL}"
pwcli open "$URL"
pwcli run-code "$smoke_code" >/dev/null

echo "Browser inspector smoke test passed."
