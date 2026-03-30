const fileInput = document.querySelector("#file-input");
const statusNode = document.querySelector("#status");
const resultsNode = document.querySelector("#results");

const ARTIFACT_TYPES = {
  RUN_SUMMARY: "RunSummary",
  TRAJECTORY_COMPARISON: "TrajectoryComparison",
  DETERMINISTIC_REPORT: "DeterministicReport",
  CALIBRATION_RESULT: "CalibrationResult",
};

fileInput.addEventListener("change", async (event) => {
  const files = Array.from(event.target.files || []);
  if (!files.length) {
    renderEmptyState("No files loaded.");
    return;
  }

  setStatus(`Loading ${files.length} file${files.length === 1 ? "" : "s"}...`);
  resultsNode.innerHTML = "";

  const cards = await Promise.all(files.map(readArtifactFile));
  resultsNode.replaceChildren(...cards);
  setStatus(`Loaded ${files.length} file${files.length === 1 ? "" : "s"}.`);
});

function setStatus(message) {
  statusNode.textContent = message;
}

function renderEmptyState(message) {
  resultsNode.innerHTML = `<article class="artifact"><div class="artifact-body"><p class="empty">${escapeHtml(message)}</p></div></article>`;
  setStatus(message);
}

async function readArtifactFile(file) {
  try {
    const raw = await file.text();
    const data = JSON.parse(raw);
    const artifactType = detectArtifactType(data);
    return renderArtifactCard(file.name, artifactType, data);
  } catch (error) {
    return renderErrorCard(file.name, error);
  }
}

function detectArtifactType(data) {
  if (!isObject(data)) {
    return null;
  }

  if (
    isObject(data.definition) &&
    isObject(data.observed) &&
    isObject(data.observed_summary) &&
    Array.isArray(data.candidates) &&
    Array.isArray(data.failures)
  ) {
    return ARTIFACT_TYPES.CALIBRATION_RESULT;
  }

  if (
    isObject(data.start_delta) &&
    isObject(data.end_delta) &&
    isObject(data.auc_delta) &&
    isObject(data.percentile_band_change)
  ) {
    return ARTIFACT_TYPES.DETERMINISTIC_REPORT;
  }

  if (
    typeof data.series_len === "number" &&
    Array.isArray(data.deltas) &&
    isObject(data.metrics) &&
    isObject(data.config)
  ) {
    return ARTIFACT_TYPES.TRAJECTORY_COMPARISON;
  }

  if ("monte_carlo" in data || "composure" in data) {
    return ARTIFACT_TYPES.RUN_SUMMARY;
  }

  return null;
}

function renderArtifactCard(fileName, artifactType, data) {
  const article = document.createElement("article");
  article.className = "artifact";

  const header = document.createElement("div");
  header.className = "artifact-header";
  header.innerHTML = `
    <div class="artifact-title">
      <h2>${escapeHtml(fileName)}</h2>
      <span class="badge">${escapeHtml(artifactType || "Unsupported JSON")}</span>
    </div>
    <code>${escapeHtml(formatByteSize(JSON.stringify(data).length))}</code>
  `;

  const body = document.createElement("div");
  body.className = "artifact-body";

  if (!artifactType) {
    body.innerHTML = `
      <p class="error">This JSON does not match one of the supported composure artifact shapes.</p>
      <div class="callout">Expected a RunSummary, TrajectoryComparison, DeterministicReport, or CalibrationResult artifact.</div>
    `;
  } else {
    body.appendChild(renderArtifactBody(artifactType, data));
  }

  article.append(header, body);
  return article;
}

function renderArtifactBody(artifactType, data) {
  switch (artifactType) {
    case ARTIFACT_TYPES.RUN_SUMMARY:
      return renderRunSummary(data);
    case ARTIFACT_TYPES.TRAJECTORY_COMPARISON:
      return renderTrajectoryComparison(data);
    case ARTIFACT_TYPES.DETERMINISTIC_REPORT:
      return renderDeterministicReport(data);
    case ARTIFACT_TYPES.CALIBRATION_RESULT:
      return renderCalibrationResult(data);
    default:
      return htmlBlock("<p class=\"error\">Unsupported artifact type.</p>");
  }
}

function renderRunSummary(summary) {
  const wrapper = document.createElement("div");
  wrapper.className = "section";

  const monteCarlo = summary.monte_carlo;
  const composure = summary.composure;

  wrapper.appendChild(htmlBlock(`
    <div class="grid">
      ${metric("Monte Carlo", monteCarlo ? "present" : "none")}
      ${metric("Composure", composure ? "present" : "none")}
      ${metric("Time steps", fmt(monteCarlo?.time_steps))}
      ${metric("Paths", fmt(monteCarlo?.num_paths))}
    </div>
  `));

  if (monteCarlo) {
    wrapper.appendChild(section("Monte Carlo Summary", `
      <div class="grid">
        ${metric("Start", num(monteCarlo.start))}
        ${metric("End", num(monteCarlo.end))}
        ${metric("AUC", num(monteCarlo.auc))}
        ${metric("Mean", num(monteCarlo.mean))}
        ${metric("Min", num(monteCarlo.min))}
        ${metric("Max", num(monteCarlo.max))}
        ${metric("P10 End", num(monteCarlo.p10_end))}
        ${metric("P50 End", num(monteCarlo.p50_end))}
        ${metric("P90 End", num(monteCarlo.p90_end))}
        ${metric("Band Width", num(monteCarlo.final_band_width))}
      </div>
    `));
  }

  if (composure) {
    wrapper.appendChild(section("Composure Summary", `
      <div class="grid">
        ${metric("Archetype", fmt(composure.archetype))}
        ${metric("Slope", num(composure.slope))}
        ${metric("Variance", num(composure.variance))}
        ${metric("Peak", num(composure.peak))}
        ${metric("Trough", num(composure.trough))}
        ${metric("Recovery Half-Life", fmt(composure.recovery_half_life))}
        ${metric("Residual Damage", num(composure.residual_damage))}
        ${metric("Break Point", fmt(composure.break_point))}
      </div>
    `));
  }

  return wrapper;
}

function renderTrajectoryComparison(comparison) {
  const metricsData = comparison.metrics || {};
  const divergence = comparison.divergence;
  const failure = metricsData.failure;
  const topDeltas = Array.isArray(comparison.deltas) ? comparison.deltas.slice(0, 8) : [];

  const wrapper = document.createElement("div");
  wrapper.className = "section";
  wrapper.appendChild(htmlBlock(`
    <div class="grid">
      ${metric("Series Length", fmt(comparison.series_len))}
      ${metric("Mean Delta", num(metricsData.mean_delta))}
      ${metric("Mean Abs Delta", num(metricsData.mean_abs_delta))}
      ${metric("RMSE", num(metricsData.rmse))}
      ${metric("End Delta", num(metricsData.end_delta))}
      ${metric("Cumulative Delta", num(metricsData.cumulative_delta))}
      ${metric("Improved Steps", fmt(metricsData.improved_steps))}
      ${metric("Regressed Steps", fmt(metricsData.regressed_steps))}
      ${metric("Unchanged Steps", fmt(metricsData.unchanged_steps))}
    </div>
  `));

  wrapper.appendChild(htmlBlock(`
    <div class="callout ${metricsData.end_delta >= 0 ? "good" : "bad"}">
      ${escapeHtml(buildComparisonHeadline(metricsData))}
    </div>
  `));

  wrapper.appendChild(section("Windows And Extremes", `
    <div class="grid">
      ${metric("Divergence Start", fmt(divergence?.start_t))}
      ${metric("Divergence End", fmt(divergence?.end_t))}
      ${metric("Divergence Length", fmt(divergence?.length))}
      ${metric("Peak Abs Delta", num(divergence?.peak_abs_delta))}
      ${metric("Best Improvement", pointSummary(metricsData.max_improvement))}
      ${metric("Worst Regression", pointSummary(metricsData.max_regression))}
    </div>
  `));

  if (failure) {
    wrapper.appendChild(section("Failure Comparison", `
      <div class="grid">
        ${metric("Threshold", num(failure.threshold))}
        ${metric("Outcome", fmt(failure.outcome))}
        ${metric("Baseline Break", fmt(failure.baseline_break_t))}
        ${metric("Candidate Break", fmt(failure.candidate_break_t))}
        ${metric("Shift", fmt(failure.shift))}
      </div>
    `));
  }

  if (topDeltas.length) {
    const rows = topDeltas.map((delta) => `
      <tr>
        <td>${escapeHtml(fmt(delta.t))}</td>
        <td>${escapeHtml(num(delta.baseline))}</td>
        <td>${escapeHtml(num(delta.candidate))}</td>
        <td>${escapeHtml(num(delta.delta))}</td>
        <td>${escapeHtml(num(delta.abs_delta))}</td>
      </tr>
    `).join("");

    wrapper.appendChild(section("Leading Point Deltas", `
      <table>
        <thead>
          <tr>
            <th>t</th>
            <th>Baseline</th>
            <th>Candidate</th>
            <th>Delta</th>
            <th>Abs Delta</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    `));
  }

  return wrapper;
}

function renderDeterministicReport(report) {
  const comparison = report.comparison;
  const wrapper = document.createElement("div");
  wrapper.className = "section";

  wrapper.appendChild(section("Key Deltas", `
    <div class="grid">
      ${metric("Start Delta", deltaTriple(report.start_delta))}
      ${metric("End Delta", deltaTriple(report.end_delta))}
      ${metric("AUC Delta", deltaTriple(report.auc_delta))}
      ${metric("Residual Damage", deltaTriple(report.residual_damage_delta))}
    </div>
  `));

  wrapper.appendChild(section("Behavior Change", `
    <div class="grid">
      ${metric("Archetype", archetypeLabel(report.archetype_change))}
      ${metric("Break Point Shift", shiftLabel(report.break_point_shift))}
      ${metric("Recovery Shift", shiftLabel(report.recovery_shift))}
      ${metric("Band Change", bandLabel(report.percentile_band_change))}
    </div>
  `));

  if (comparison) {
    wrapper.appendChild(section("Comparison Snapshot", `
      <div class="grid">
        ${metric("Mean Delta", num(comparison.mean_delta))}
        ${metric("Mean Abs Delta", num(comparison.mean_abs_delta))}
        ${metric("RMSE", num(comparison.rmse))}
        ${metric("End Delta", num(comparison.end_delta))}
        ${metric("Divergence Start", fmt(comparison.divergence_start_t))}
        ${metric("Divergence End", fmt(comparison.divergence_end_t))}
        ${metric("Failure Shift", fmt(comparison.failure_shift))}
      </div>
    `));
  }

  return wrapper;
}

function renderCalibrationResult(result) {
  const topCandidate = Array.isArray(result.candidates) ? result.candidates[0] : null;
  const wrapper = document.createElement("div");
  wrapper.className = "section";

  wrapper.appendChild(section("Calibration Overview", `
    <div class="grid">
      ${metric("Definition", labelPair(result.definition?.name, result.definition?.id))}
      ${metric("Observed", labelPair(result.observed?.name, result.observed?.id))}
      ${metric("Strategy", fmt(result.definition?.strategy))}
      ${metric("Objective", fmt(result.config?.objective))}
      ${metric("Failure Mode", fmt(result.config?.failure_mode))}
      ${metric("Candidates", fmt(result.candidates?.length))}
      ${metric("Failures", fmt(result.failures?.length))}
      ${metric("Best Case", fmt(result.best_case_id))}
      ${metric("Best Score", num(result.best_score))}
      ${metric("Bundle Attached", result.bundle ? "yes" : "no")}
    </div>
  `));

  wrapper.appendChild(section("Observed Summary", `
    <div class="grid">
      ${metric("Observed Points", fmt(result.observed?.values?.length))}
      ${metric("Observed Start", num(result.observed_summary?.monte_carlo?.start))}
      ${metric("Observed End", num(result.observed_summary?.monte_carlo?.end))}
      ${metric("Observed AUC", num(result.observed_summary?.monte_carlo?.auc))}
    </div>
  `));

  if (topCandidate) {
    wrapper.appendChild(section("Top Candidate", `
      <div class="grid">
        ${metric("Case", fmt(topCandidate.case?.case_id))}
        ${metric("Parameter Set", fmt(topCandidate.parameter_set?.id))}
        ${metric("Score", num(topCandidate.score))}
        ${metric("RMSE", num(topCandidate.comparison?.metrics?.rmse))}
        ${metric("End Delta", num(topCandidate.comparison?.metrics?.end_delta))}
        ${metric("Status", fmt(topCandidate.run?.status))}
      </div>
      <div class="callout">
        ${escapeHtml(parameterSummary(topCandidate.case?.parameters))}
      </div>
    `));
  }

  if (result.failures?.length) {
    const rows = result.failures.slice(0, 6).map((failure) => `
      <div class="list-row">
        <strong>${escapeHtml(fmt(failure.case?.case_id))}</strong>
        <div>${escapeHtml(failure.error || "Unknown error")}</div>
      </div>
    `).join("");
    wrapper.appendChild(section("Failures", `<div class="list">${rows}</div>`));
  }

  return wrapper;
}

function renderErrorCard(fileName, error) {
  const message = error instanceof Error ? error.message : String(error);
  const article = document.createElement("article");
  article.className = "artifact";
  article.innerHTML = `
    <div class="artifact-header">
      <div class="artifact-title">
        <h2>${escapeHtml(fileName)}</h2>
        <span class="badge">Parse Error</span>
      </div>
    </div>
    <div class="artifact-body">
      <p class="error">${escapeHtml(message)}</p>
    </div>
  `;
  return article;
}

function buildComparisonHeadline(metricsData) {
  if (!isObject(metricsData)) {
    return "No comparison metrics available.";
  }

  const direction = metricsData.end_delta >= 0 ? "ahead of" : "behind";
  return `Candidate finishes ${direction} baseline by ${num(metricsData.end_delta)} with RMSE ${num(metricsData.rmse)}.`;
}

function pointSummary(point) {
  if (!isObject(point)) {
    return "n/a";
  }
  return `t=${fmt(point.t)} delta=${num(point.delta)}`;
}

function deltaTriple(delta) {
  if (!isObject(delta)) {
    return "n/a";
  }
  return `${num(delta.candidate)} vs ${num(delta.baseline)} (${signed(delta.delta)})`;
}

function archetypeLabel(change) {
  if (!isObject(change)) {
    return "n/a";
  }
  const baseline = fmt(change.baseline);
  const candidate = fmt(change.candidate);
  return `${candidate} vs ${baseline}${change.changed ? " (changed)" : ""}`;
}

function shiftLabel(shift) {
  if (!isObject(shift)) {
    return "n/a";
  }
  return `${fmt(shift.candidate)} vs ${fmt(shift.baseline)} (${signed(shift.shift)})`;
}

function bandLabel(change) {
  if (!isObject(change)) {
    return "n/a";
  }
  return `${fmt(change.direction)} (${signed(change.delta)})`;
}

function labelPair(name, id) {
  if (!name && !id) {
    return "n/a";
  }
  return id ? `${fmt(name)} (${fmt(id)})` : fmt(name);
}

function parameterSummary(parameters) {
  if (!isObject(parameters)) {
    return "No parameter details available.";
  }
  const parts = Object.entries(parameters).map(([key, value]) => `${key}=${renderParameterValue(value)}`);
  return parts.length ? `Parameters: ${parts.join(", ")}` : "No parameter details available.";
}

function renderParameterValue(value) {
  if (Array.isArray(value)) {
    return value.map(renderParameterValue).join(", ");
  }
  if (isObject(value)) {
    const entries = Object.entries(value);
    if (entries.length === 1) {
      return fmt(entries[0][1]);
    }
    return JSON.stringify(value);
  }
  return fmt(value);
}

function metric(label, value) {
  return `
    <div class="metric">
      <span class="metric-label">${escapeHtml(label)}</span>
      <span class="metric-value">${escapeHtml(value)}</span>
    </div>
  `;
}

function section(title, innerHtml) {
  return htmlBlock(`<section class="section"><h3>${escapeHtml(title)}</h3>${innerHtml}</section>`);
}

function htmlBlock(markup) {
  const wrapper = document.createElement("div");
  wrapper.innerHTML = markup;
  return wrapper.firstElementChild || wrapper;
}

function num(value) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "n/a";
  }
  return typeof value === "number" ? value.toFixed(4).replace(/\.?0+$/, "") : String(value);
}

function signed(value) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "n/a";
  }
  if (typeof value !== "number") {
    return String(value);
  }
  return `${value > 0 ? "+" : ""}${num(value)}`;
}

function fmt(value) {
  if (value === null || value === undefined || value === "") {
    return "n/a";
  }
  return String(value);
}

function formatByteSize(size) {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / (1024 * 1024)).toFixed(2)} MB`;
}

function isObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;")
    .replaceAll("'", "&#39;");
}
