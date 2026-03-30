const fileInput = document.querySelector("#file-input");
const statusNode = document.querySelector("#status");
const resultsNode = document.querySelector("#results");

const ARTIFACT_TYPES = {
  RUN_SUMMARY: "RunSummary",
  COUNTERFACTUAL_RESULT: "CounterfactualResult",
  COUNTERFACTUAL_DEFINITION: "CounterfactualDefinition",
  TRAJECTORY_COMPARISON: "TrajectoryComparison",
  DETERMINISTIC_REPORT: "DeterministicReport",
  CALIBRATION_RESULT: "CalibrationResult",
  EXPERIMENT_BUNDLE: "ExperimentBundle",
  SWEEP_EXECUTION_RESULT: "SweepExecutionResult",
};
const FILTER_ALL = "__all__";

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
    isObject(data.baseline) &&
    isObject(data.candidate) &&
    isObject(data.comparison) &&
    isObject(data.report) &&
    isObject(data.baseline.summary) &&
    isObject(data.candidate.summary)
  ) {
    return ARTIFACT_TYPES.COUNTERFACTUAL_RESULT;
  }

  if (
    typeof data.id === "string" &&
    typeof data.name === "string" &&
    isObject(data.branch_state) &&
    isObject(data.baseline) &&
    isObject(data.candidate) &&
    isObject(data.config) &&
    isObject(data.runtime_model)
  ) {
    return ARTIFACT_TYPES.COUNTERFACTUAL_DEFINITION;
  }

  if (
    isObject(data.definition) &&
    Array.isArray(data.executed_cases) &&
    Array.isArray(data.failures) &&
    Array.isArray(data.samples) &&
    isObject(data.config)
  ) {
    return ARTIFACT_TYPES.SWEEP_EXECUTION_RESULT;
  }

  if (
    isObject(data.spec) &&
    Array.isArray(data.parameter_sets) &&
    Array.isArray(data.runs)
  ) {
    return ARTIFACT_TYPES.EXPERIMENT_BUNDLE;
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
      <div class="callout">Expected a RunSummary, CounterfactualResult, CounterfactualDefinition, TrajectoryComparison, DeterministicReport, CalibrationResult, ExperimentBundle, or SweepExecutionResult artifact.</div>
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
    case ARTIFACT_TYPES.COUNTERFACTUAL_RESULT:
      return renderCounterfactualResult(data);
    case ARTIFACT_TYPES.COUNTERFACTUAL_DEFINITION:
      return renderCounterfactualDefinition(data);
    case ARTIFACT_TYPES.TRAJECTORY_COMPARISON:
      return renderTrajectoryComparison(data);
    case ARTIFACT_TYPES.DETERMINISTIC_REPORT:
      return renderDeterministicReport(data);
    case ARTIFACT_TYPES.CALIBRATION_RESULT:
      return renderCalibrationResult(data);
    case ARTIFACT_TYPES.EXPERIMENT_BUNDLE:
      return renderExperimentBundle(data);
    case ARTIFACT_TYPES.SWEEP_EXECUTION_RESULT:
      return renderSweepExecutionResult(data);
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

function renderCounterfactualResult(result) {
  const comparisonMetrics = result.comparison?.metrics || {};
  const divergence = result.comparison?.divergence;
  const wrapper = document.createElement("div");
  wrapper.className = "section";

  wrapper.appendChild(section("Counterfactual Overview", `
    <div class="grid">
      ${metric("Baseline", labelPair(result.baseline?.intervention_label, result.baseline?.branch_id))}
      ${metric("Candidate", labelPair(result.candidate?.intervention_label, result.candidate?.branch_id))}
      ${metric("Branch From t", fmt(result.baseline?.branch_from_t ?? result.candidate?.branch_from_t))}
      ${metric("End Delta", num(comparisonMetrics.end_delta))}
      ${metric("RMSE", num(comparisonMetrics.rmse))}
      ${metric("Improved Steps", fmt(comparisonMetrics.improved_steps))}
      ${metric("Regressed Steps", fmt(comparisonMetrics.regressed_steps))}
      ${metric("Failure Shift", fmt(result.report?.comparison?.failure_shift))}
      ${metric("Divergence Window", divergence ? `${fmt(divergence.start_t)}-${fmt(divergence.end_t)}` : "none")}
    </div>
  `));

  wrapper.appendChild(htmlBlock(`
    <div class="callout ${comparisonMetrics.end_delta >= 0 ? "good" : "bad"}">
      ${escapeHtml(buildComparisonHeadline(comparisonMetrics))}
    </div>
  `));

  wrapper.appendChild(renderCounterfactualBranchSection("Baseline Branch", result.baseline));
  wrapper.appendChild(renderCounterfactualBranchSection("Candidate Branch", result.candidate));
  wrapper.appendChild(renderTrajectoryComparison(result.comparison));
  wrapper.appendChild(renderDeterministicReport(result.report));
  return wrapper;
}

function renderCounterfactualBranchSection(title, branch) {
  const summary = branch?.summary || {};
  const monteCarlo = summary.monte_carlo;
  const composure = summary.composure;

  return section(title, `
    <div class="grid">
      ${metric("Branch", labelPair(branch?.intervention_label, branch?.branch_id))}
      ${metric("From t", fmt(branch?.branch_from_t))}
      ${metric("Paths", fmt(monteCarlo?.num_paths))}
      ${metric("Time Steps", fmt(monteCarlo?.time_steps))}
      ${metric("Start", num(monteCarlo?.start))}
      ${metric("End", num(monteCarlo?.end))}
      ${metric("AUC", num(monteCarlo?.auc))}
      ${metric("Archetype", fmt(composure?.archetype))}
      ${metric("Slope", num(composure?.slope))}
      ${metric("Band Width", num(monteCarlo?.final_band_width))}
    </div>
  `);
}

function renderCounterfactualDefinition(definition) {
  const branchState = definition.branch_state || {};
  const baseline = definition.baseline || {};
  const candidate = definition.candidate || {};
  const config = definition.config || {};
  const runtimeModel = definition.runtime_model || {};
  const conditionalRules = [
    ...collectConditionalRules("Baseline", baseline.conditional_actions),
    ...collectConditionalRules("Candidate", candidate.conditional_actions),
  ];
  const runtimeDimensions = Array.isArray(runtimeModel.dimensions) ? runtimeModel.dimensions.slice(0, 4) : [];
  const wrapper = document.createElement("div");
  wrapper.className = "section";

  wrapper.appendChild(section("Counterfactual Setup", `
    <div class="grid">
      ${metric("Definition", labelPair(definition.name, definition.id))}
      ${metric("Branch State t", fmt(branchState.t))}
      ${metric("State Dimensions", fmt(branchState.z?.length))}
      ${metric("Baseline", labelPair(baseline.intervention_label, baseline.branch_id))}
      ${metric("Candidate", labelPair(candidate.intervention_label, candidate.branch_id))}
      ${metric("Paths", fmt(config.monte_carlo?.num_paths))}
      ${metric("Time Steps", fmt(config.monte_carlo?.time_steps))}
      ${metric("Failure Threshold", num(config.comparison?.failure_threshold))}
      ${metric("Divergence Threshold", num(config.comparison?.divergence_threshold))}
      ${metric("Retain Paths", boolLabel(config.execution?.retain_paths))}
      ${metric("Analyze Composure", boolLabel(config.execution?.analyze_composure))}
      ${metric("Runtime Model", fmt(runtimeModel.kind))}
    </div>
  `));

  if (definition.description) {
    wrapper.appendChild(htmlBlock(`
      <div class="callout">
        ${escapeHtml(definition.description)}
      </div>
    `));
  }

  wrapper.appendChild(section("Branch State", `
    <div class="grid">
      ${metric("z", arrayPreview(branchState.z))}
      ${metric("m", arrayPreview(branchState.m))}
      ${metric("u", arrayPreview(branchState.u))}
    </div>
  `));

  wrapper.appendChild(section("Branch Inputs", `
    <table>
      <thead>
        <tr>
          <th>Branch</th>
          <th>Label</th>
          <th>Actions</th>
          <th>Conditional Rules</th>
          <th>Action Mix</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>Baseline</td>
          <td>${escapeHtml(labelPair(baseline.intervention_label, baseline.branch_id))}</td>
          <td>${escapeHtml(fmt(baseline.actions?.length))}</td>
          <td>${escapeHtml(fmt(baseline.conditional_actions?.length))}</td>
          <td>${escapeHtml(actionTypeSummary(baseline.actions))}</td>
        </tr>
        <tr>
          <td>Candidate</td>
          <td>${escapeHtml(labelPair(candidate.intervention_label, candidate.branch_id))}</td>
          <td>${escapeHtml(fmt(candidate.actions?.length))}</td>
          <td>${escapeHtml(fmt(candidate.conditional_actions?.length))}</td>
          <td>${escapeHtml(actionTypeSummary(candidate.actions))}</td>
        </tr>
      </tbody>
    </table>
  `));

  if (conditionalRules.length) {
    const rows = conditionalRules.slice(0, 6).map((rule) => `
      <div class="list-row">
        <strong>${escapeHtml(rule.branch)} · ${escapeHtml(rule.id)}</strong>
        <div>${escapeHtml(rule.summary)}</div>
      </div>
    `).join("");
    wrapper.appendChild(section("Conditional Rules", `<div class="list">${rows}</div>`));
  }

  wrapper.appendChild(section("Runtime Model", `
    <div class="grid">
      ${metric("Dimensions", fmt(runtimeModel.dimensions?.length))}
      ${metric("Noise Scale", num(runtimeModel.noise_scale))}
      ${metric("Aggregate Weights", arrayPreview(runtimeModel.aggregate_weights))}
      ${metric("Intervention Scale", num(runtimeModel.action_type_scales?.intervention))}
      ${metric("Stressor Onset Scale", num(runtimeModel.action_type_scales?.stressor_onset))}
      ${metric("Hold Scale", num(runtimeModel.action_type_scales?.hold))}
      ${metric("Custom Action Types", fmt(Object.keys(runtimeModel.action_type_scales?.custom || {}).length))}
    </div>
  `));

  if (runtimeDimensions.length) {
    const rows = runtimeDimensions.map((dimension, index) => `
      <tr>
        <td>${escapeHtml(fmt(index))}</td>
        <td>${escapeHtml(num(dimension.drift))}</td>
        <td>${escapeHtml(num(dimension.action_gain))}</td>
        <td>${escapeHtml(num(dimension.memory_decay))}</td>
        <td>${escapeHtml(num(dimension.memory_to_state))}</td>
        <td>${escapeHtml(num(dimension.min_value))}-${escapeHtml(num(dimension.max_value))}</td>
      </tr>
    `).join("");

    wrapper.appendChild(section("Runtime Dimensions", `
      <table>
        <thead>
          <tr>
            <th>#</th>
            <th>Drift</th>
            <th>Action Gain</th>
            <th>Memory Decay</th>
            <th>Memory To State</th>
            <th>Clamp</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
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

  if (result.candidates?.length) {
    wrapper.appendChild(renderCalibrationCandidatesSection(result.candidates));
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

function renderExperimentBundle(bundle) {
  const statusCounts = countRunStatuses(bundle.runs);
  const topSets = Array.isArray(bundle.parameter_sets) ? bundle.parameter_sets.slice(0, 6) : [];
  const topRuns = Array.isArray(bundle.runs) ? bundle.runs.slice(0, 6) : [];
  const wrapper = document.createElement("div");
  wrapper.className = "section";

  wrapper.appendChild(section("Bundle Overview", `
    <div class="grid">
      ${metric("Experiment", labelPair(bundle.spec?.name, bundle.spec?.id))}
      ${metric("Scenario", labelPair(bundle.spec?.scenario?.name, bundle.spec?.scenario?.id))}
      ${metric("Time Steps", fmt(bundle.spec?.scenario?.time_steps))}
      ${metric("Failure Threshold", num(bundle.spec?.scenario?.failure_threshold))}
      ${metric("Parameter Sets", fmt(bundle.parameter_sets?.length))}
      ${metric("Runs", fmt(bundle.runs?.length))}
      ${metric("Default Paths", fmt(bundle.spec?.default_monte_carlo?.num_paths))}
      ${metric("Tags", fmt(bundle.spec?.tags?.length))}
    </div>
  `));

  wrapper.appendChild(section("Run States", `
    <div class="grid">
      ${metric("Completed", fmt(statusCounts.Completed))}
      ${metric("Failed", fmt(statusCounts.Failed))}
      ${metric("Running", fmt(statusCounts.Running))}
      ${metric("Pending", fmt(statusCounts.Pending))}
      ${metric("Created", unixTime(bundle.spec?.created_at_unix_s))}
    </div>
  `));

  if (bundle.spec?.description) {
    wrapper.appendChild(htmlBlock(`
      <div class="callout">
        ${escapeHtml(bundle.spec.description)}
      </div>
    `));
  }

  if (topSets.length) {
    const rows = topSets.map((parameterSet) => `
      <tr>
        <td>${escapeHtml(labelPair(parameterSet.name, parameterSet.id))}</td>
        <td>${escapeHtml(fmt(parameterSet.scenario?.time_steps))}</td>
        <td>${escapeHtml(fmt(parameterSet.monte_carlo?.num_paths))}</td>
        <td>${escapeHtml(fmt(parameterSet.monte_carlo?.seed_base))}</td>
      </tr>
    `).join("");

    wrapper.appendChild(section("Parameter Sets", `
      <table>
        <thead>
          <tr>
            <th>Set</th>
            <th>Steps</th>
            <th>Paths</th>
            <th>Seed</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    `));
  }

  if (topRuns.length) {
    const rows = topRuns.map((run) => `
      <tr>
        <td>${escapeHtml(fmt(run.run_id))}</td>
        <td>${escapeHtml(fmt(run.parameter_set_id))}</td>
        <td>${escapeHtml(fmt(run.status))}</td>
        <td>${escapeHtml(fmt(run.seed))}</td>
        <td>${escapeHtml(summarizeOutcome(run.outcome))}</td>
      </tr>
    `).join("");

    wrapper.appendChild(section("Recent Runs", `
      <table>
        <thead>
          <tr>
            <th>Run</th>
            <th>Parameter Set</th>
            <th>Status</th>
            <th>Seed</th>
            <th>Artifacts</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    `));
  }

  return wrapper;
}

function renderSweepExecutionResult(result) {
  const executedCases = Array.isArray(result.executed_cases) ? result.executed_cases : [];
  const topCases = executedCases.slice(0, 6);
  const topFailures = Array.isArray(result.failures) ? result.failures.slice(0, 6) : [];
  const topRankings = Array.isArray(result.sensitivity?.rankings) ? result.sensitivity.rankings.slice(0, 5) : [];
  const wrapper = document.createElement("div");
  wrapper.className = "section";

  wrapper.appendChild(section("Sweep Overview", `
    <div class="grid">
      ${metric("Sweep", labelPair(result.definition?.name, result.definition?.id))}
      ${metric("Strategy", fmt(result.definition?.strategy))}
      ${metric("Configured Samples", fmt(result.definition?.sample_count))}
      ${metric("Seed", fmt(result.definition?.seed))}
      ${metric("Parameters", fmt(result.definition?.parameters?.length))}
      ${metric("Executed Cases", fmt(result.executed_cases?.length))}
      ${metric("Failures", fmt(result.failures?.length))}
      ${metric("Scored Samples", fmt(result.samples?.length))}
      ${metric("Bundle Attached", result.bundle ? "yes" : "no")}
      ${metric("Failure Mode", fmt(result.config?.failure_mode))}
    </div>
  `));

  if (Array.isArray(result.definition?.parameters) && result.definition.parameters.length) {
    const rows = result.definition.parameters.slice(0, 6).map((parameter) => `
      <tr>
        <td>${escapeHtml(fmt(parameter.name))}</td>
        <td>${escapeHtml(fmt(parameter.values?.length))}</td>
        <td>${escapeHtml(parameterValuesSummary(parameter.values))}</td>
      </tr>
    `).join("");

    wrapper.appendChild(section("Sweep Parameters", `
      <table>
        <thead>
          <tr>
            <th>Parameter</th>
            <th>Values</th>
            <th>Preview</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    `));
  }

  if (result.sensitivity) {
    wrapper.appendChild(section("Sensitivity Summary", `
      <div class="grid">
        ${metric("Sample Count", fmt(result.sensitivity.sample_count))}
        ${metric("Objective Min", num(result.sensitivity.objective?.min))}
        ${metric("Objective Mean", num(result.sensitivity.objective?.mean))}
        ${metric("Objective Max", num(result.sensitivity.objective?.max))}
        ${metric("Best Case", fmt(result.sensitivity.objective?.best_case_id))}
        ${metric("Worst Case", fmt(result.sensitivity.objective?.worst_case_id))}
      </div>
    `));
  }

  if (topRankings.length) {
    const rows = topRankings.map((ranking) => `
      <tr>
        <td>${escapeHtml(fmt(ranking.parameter))}</td>
        <td>${escapeHtml(num(ranking.score))}</td>
        <td>${escapeHtml(fmt(ranking.direction))}</td>
        <td>${escapeHtml(sensitivityKindSummary(ranking.kind))}</td>
      </tr>
    `).join("");

    wrapper.appendChild(section("Top Sensitivities", `
      <table>
        <thead>
          <tr>
            <th>Parameter</th>
            <th>Score</th>
            <th>Direction</th>
            <th>Detail</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    `));
  }

  if (result.samples?.length) {
    wrapper.appendChild(renderSweepSamplesSection(result.samples, executedCases));
  }

  if (topCases.length) {
    const rows = topCases.map((entry) => `
      <tr>
        <td>${escapeHtml(fmt(entry.case?.case_id))}</td>
        <td>${escapeHtml(fmt(entry.run?.status))}</td>
        <td>${escapeHtml(num(entry.sample?.objective))}</td>
        <td>${escapeHtml(fmt(entry.parameter_set?.id))}</td>
        <td>${escapeHtml(parameterSummary(entry.case?.parameters))}</td>
      </tr>
    `).join("");

    wrapper.appendChild(section("Executed Cases", `
      <table>
        <thead>
          <tr>
            <th>Case</th>
            <th>Status</th>
            <th>Objective</th>
            <th>Parameter Set</th>
            <th>Parameters</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>
    `));
  }

  if (topFailures.length) {
    const rows = topFailures.map((failure) => `
      <div class="list-row">
        <strong>${escapeHtml(fmt(failure.case?.case_id))}</strong>
        <div>${escapeHtml(fmt(failure.parameter_set_id))} · ${escapeHtml(failure.error || "Unknown error")}</div>
      </div>
    `).join("");
    wrapper.appendChild(section("Failures", `<div class="list">${rows}</div>`));
  }

  return wrapper;
}

function renderCalibrationCandidatesSection(candidates) {
  const rows = candidates.map((candidate, index) => normalizeCalibrationCandidate(candidate, index));
  return renderInteractiveTableSection({
    title: "Calibration Candidates",
    rowLabel: "candidate",
    rows,
    queryPlaceholder: "Search case, parameter set, status, or parameters",
    filterLabel: "Status",
    getFilterValue: (row) => row.status,
    defaultSort: "original",
    sortOptions: [
      {
        value: "original",
        label: "Original order",
        compare: (left, right) => left.index - right.index,
      },
      {
        value: "score-asc",
        label: "Score low to high",
        compare: (left, right) => compareNullableNumbers(left.score, right.score),
      },
      {
        value: "score-desc",
        label: "Score high to low",
        compare: (left, right) => compareNullableNumbers(left.score, right.score, "desc"),
      },
      {
        value: "rmse-asc",
        label: "RMSE low to high",
        compare: (left, right) => compareNullableNumbers(left.rmse, right.rmse),
      },
      {
        value: "case-asc",
        label: "Case A-Z",
        compare: (left, right) => compareTextValues(left.caseId, right.caseId),
      },
    ],
    emptyMessage: "No calibration candidates match the current controls.",
    buildTableMarkup: (filteredRows) => `
      <table>
        <thead>
          <tr>
            <th>Case</th>
            <th>Parameter Set</th>
            <th>Status</th>
            <th>Score</th>
            <th>RMSE</th>
            <th>End Delta</th>
            <th>Parameters</th>
          </tr>
        </thead>
        <tbody>
          ${filteredRows.map((row) => `
            <tr>
              <td>${escapeHtml(row.caseId)}</td>
              <td>${escapeHtml(row.parameterSet)}</td>
              <td>${escapeHtml(row.status)}</td>
              <td>${escapeHtml(num(row.score))}</td>
              <td>${escapeHtml(num(row.rmse))}</td>
              <td>${escapeHtml(num(row.endDelta))}</td>
              <td>${escapeHtml(row.parameters)}</td>
            </tr>
          `).join("")}
        </tbody>
      </table>
    `,
  });
}

function normalizeCalibrationCandidate(candidate, index) {
  const caseId = fmt(candidate.case?.case_id);
  const parameterSet = labelPair(candidate.parameter_set?.name, candidate.parameter_set?.id);
  const score = toNumberOrNull(candidate.score);
  const rmse = toNumberOrNull(candidate.comparison?.metrics?.rmse);
  const endDelta = toNumberOrNull(candidate.comparison?.metrics?.end_delta);
  const status = fmt(candidate.run?.status);
  const parameters = parameterDetails(candidate.case?.parameters);

  return {
    index,
    caseId,
    parameterSet,
    score,
    rmse,
    endDelta,
    status,
    parameters,
    searchText: [caseId, parameterSet, status, parameters].join(" ").toLowerCase(),
  };
}

function renderSweepSamplesSection(samples, executedCases) {
  const caseLookup = new Map(
    executedCases
      .filter((entry) => entry?.case?.case_id)
      .map((entry) => [entry.case.case_id, entry]),
  );
  const rows = samples.map((sample, index) => normalizeSweepSample(sample, index, caseLookup));

  return renderInteractiveTableSection({
    title: "Scored Samples",
    rowLabel: "sample",
    rows,
    queryPlaceholder: "Search case, parameter set, run, status, or parameters",
    filterLabel: "Parameter Set",
    getFilterValue: (row) => row.parameterSetId,
    defaultSort: "original",
    sortOptions: [
      {
        value: "original",
        label: "Original order",
        compare: (left, right) => left.index - right.index,
      },
      {
        value: "objective-asc",
        label: "Objective low to high",
        compare: (left, right) => compareNullableNumbers(left.objective, right.objective),
      },
      {
        value: "objective-desc",
        label: "Objective high to low",
        compare: (left, right) => compareNullableNumbers(left.objective, right.objective, "desc"),
      },
      {
        value: "case-asc",
        label: "Case A-Z",
        compare: (left, right) => compareTextValues(left.caseId, right.caseId),
      },
      {
        value: "parameter-set-asc",
        label: "Parameter Set A-Z",
        compare: (left, right) => compareTextValues(left.parameterSetId, right.parameterSetId),
      },
    ],
    emptyMessage: "No sweep samples match the current controls.",
    buildTableMarkup: (filteredRows) => `
      <table>
        <thead>
          <tr>
            <th>Case</th>
            <th>Objective</th>
            <th>Status</th>
            <th>Parameter Set</th>
            <th>Run</th>
            <th>Parameters</th>
          </tr>
        </thead>
        <tbody>
          ${filteredRows.map((row) => `
            <tr>
              <td>${escapeHtml(row.caseId)}</td>
              <td>${escapeHtml(num(row.objective))}</td>
              <td>${escapeHtml(row.status)}</td>
              <td>${escapeHtml(row.parameterSetId)}</td>
              <td>${escapeHtml(row.runId)}</td>
              <td>${escapeHtml(row.parameters)}</td>
            </tr>
          `).join("")}
        </tbody>
      </table>
    `,
  });
}

function normalizeSweepSample(sample, index, caseLookup) {
  const executedCase = caseLookup.get(sample.case_id);
  const caseId = fmt(sample.case_id);
  const objective = toNumberOrNull(sample.objective);
  const parameterSetId = fmt(
    sample.metadata?.parameter_set_id
      ?? executedCase?.parameter_set?.id
      ?? executedCase?.run?.parameter_set_id,
  );
  const runId = fmt(sample.metadata?.run_id ?? executedCase?.run?.run_id);
  const status = fmt(executedCase?.run?.status);
  const parameters = parameterDetails(sample.parameters ?? executedCase?.case?.parameters);

  return {
    index,
    caseId,
    objective,
    parameterSetId,
    runId,
    status,
    parameters,
    searchText: [caseId, parameterSetId, runId, status, parameters].join(" ").toLowerCase(),
  };
}

function renderInteractiveTableSection({
  title,
  rowLabel,
  rows,
  queryPlaceholder,
  filterLabel,
  getFilterValue,
  defaultSort,
  sortOptions,
  emptyMessage,
  buildTableMarkup,
}) {
  const sectionNode = document.createElement("section");
  sectionNode.className = "section";

  const titleNode = document.createElement("h3");
  titleNode.textContent = title;

  const controlsNode = document.createElement("div");
  controlsNode.className = "table-controls";

  const queryField = createTextControl("Filter", "query", queryPlaceholder);
  const queryInput = queryField.querySelector("input");
  controlsNode.appendChild(queryField);

  const filterValues = collectFilterValues(rows, getFilterValue);
  const shouldRenderFilter = filterValues.length > 1;
  let filterSelect = null;
  if (shouldRenderFilter) {
    const filterField = createSelectControl(filterLabel, "filter", [
      { value: FILTER_ALL, label: `All ${filterLabel.toLowerCase()}s` },
      ...filterValues.map((value) => ({ value, label: value })),
    ], FILTER_ALL);
    filterSelect = filterField.querySelector("select");
    controlsNode.appendChild(filterField);
  }

  const sortField = createSelectControl("Sort", "sort", sortOptions, defaultSort);
  const sortSelect = sortField.querySelector("select");
  controlsNode.appendChild(sortField);

  const summaryNode = document.createElement("p");
  summaryNode.className = "table-summary";

  const tableShell = document.createElement("div");
  tableShell.className = "table-shell";

  const updateTable = () => {
    const query = queryInput.value.trim().toLowerCase();
    const activeFilter = filterSelect ? filterSelect.value : FILTER_ALL;
    const activeSort = sortOptions.find((option) => option.value === sortSelect.value) || sortOptions[0];

    const filteredRows = rows
      .filter((row) => {
        if (query && !row.searchText.includes(query)) {
          return false;
        }

        if (activeFilter !== FILTER_ALL && getFilterValue(row) !== activeFilter) {
          return false;
        }

        return true;
      })
      .slice()
      .sort((left, right) => activeSort.compare(left, right) || left.index - right.index);

    summaryNode.textContent = `Showing ${filteredRows.length} of ${rows.length} ${rowLabel}${rows.length === 1 ? "" : "s"}.`;
    tableShell.innerHTML = filteredRows.length
      ? buildTableMarkup(filteredRows)
      : `<div class="callout">${escapeHtml(emptyMessage)}</div>`;
  };

  queryInput.addEventListener("input", updateTable);
  sortSelect.addEventListener("change", updateTable);
  if (filterSelect) {
    filterSelect.addEventListener("change", updateTable);
  }

  sectionNode.append(titleNode, controlsNode, summaryNode, tableShell);
  updateTable();
  return sectionNode;
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

function collectConditionalRules(branchLabel, rules) {
  if (!Array.isArray(rules)) {
    return [];
  }

  return rules.map((rule, index) => ({
    branch: branchLabel,
    id: fmt(rule?.id || `${branchLabel.toLowerCase()}-rule-${index + 1}`),
    summary: `${triggerSummary(rule?.trigger)} -> ${actionSummary(rule?.action)} · ${ruleWindowSummary(rule)}`,
  }));
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
  const details = parameterDetails(parameters);
  return details === "n/a" ? "No parameter details available." : `Parameters: ${details}`;
}

function arrayPreview(values, maxItems = 4) {
  if (!Array.isArray(values) || !values.length) {
    return "n/a";
  }
  const preview = values.slice(0, maxItems).map((value) => (
    typeof value === "number" ? num(value) : fmt(value)
  ));
  return values.length > maxItems ? `${preview.join(", ")}, ...` : preview.join(", ");
}

function boolLabel(value) {
  if (value === null || value === undefined) {
    return "n/a";
  }
  return value ? "yes" : "no";
}

function actionTypeSummary(actions) {
  if (!Array.isArray(actions) || !actions.length) {
    return "none";
  }

  const counts = new Map();
  for (const action of actions) {
    const actionType = fmt(action?.action_type);
    counts.set(actionType, (counts.get(actionType) || 0) + 1);
  }

  return Array.from(counts.entries())
    .map(([actionType, count]) => `${actionType} x${count}`)
    .join(", ");
}

function parameterDetails(parameters) {
  if (!isObject(parameters)) {
    return "n/a";
  }
  const parts = Object.entries(parameters).map(([key, value]) => `${key}=${renderParameterValue(value)}`);
  return parts.length ? parts.join(", ") : "n/a";
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

function triggerSummary(trigger) {
  if (!isObject(trigger)) {
    return "trigger n/a";
  }

  const details = Object.entries(trigger)
    .filter(([key]) => key !== "kind")
    .map(([key, value]) => `${key}=${fmt(value)}`);
  return details.length ? `${fmt(trigger.kind)} (${details.join(", ")})` : fmt(trigger.kind);
}

function actionSummary(action) {
  if (!isObject(action)) {
    return "action n/a";
  }
  return `${fmt(action.action_type)} dim=${fmt(action.dimension)} mag=${num(action.magnitude)}`;
}

function ruleWindowSummary(rule) {
  if (!isObject(rule)) {
    return "delay=n/a";
  }
  return [
    `delay=${fmt(rule.delay_steps)}`,
    `cooldown=${fmt(rule.cooldown_steps)}`,
    `priority=${fmt(rule.priority)}`,
    `max=${fmt(rule.max_fires)}`,
  ].join(", ");
}

function countRunStatuses(runs) {
  const counts = {
    Completed: 0,
    Failed: 0,
    Running: 0,
    Pending: 0,
  };

  if (!Array.isArray(runs)) {
    return counts;
  }

  for (const run of runs) {
    if (run && typeof run.status === "string" && run.status in counts) {
      counts[run.status] += 1;
    }
  }

  return counts;
}

function summarizeOutcome(outcome) {
  if (!isObject(outcome)) {
    return "none";
  }

  const parts = [];
  if (outcome.monte_carlo) {
    parts.push("monte_carlo");
  }
  if (outcome.composure) {
    parts.push("composure");
  }
  if (outcome.replay) {
    parts.push("replay");
  }
  return parts.length ? parts.join(", ") : "none";
}

function parameterValuesSummary(values) {
  if (!Array.isArray(values) || !values.length) {
    return "n/a";
  }
  const preview = values.slice(0, 4).map(renderParameterValue);
  return values.length > 4 ? `${preview.join(", ")}, ...` : preview.join(", ");
}

function sensitivityKindSummary(kind) {
  if (!isObject(kind)) {
    return "n/a";
  }
  if (isObject(kind.Numeric)) {
    return `numeric corr=${num(kind.Numeric.correlation)} slope=${num(kind.Numeric.slope)}`;
  }
  if (isObject(kind.Categorical)) {
    return `categorical range=${num(kind.Categorical.range)} buckets=${fmt(kind.Categorical.buckets?.length)}`;
  }
  return "n/a";
}

function unixTime(value) {
  if (typeof value !== "number") {
    return "n/a";
  }
  const date = new Date(value * 1000);
  if (Number.isNaN(date.getTime())) {
    return "n/a";
  }
  return date.toLocaleString();
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

function createTextControl(label, role, placeholder) {
  const field = document.createElement("label");
  field.className = "control-field";

  const labelNode = document.createElement("span");
  labelNode.className = "control-label";
  labelNode.textContent = label;

  const input = document.createElement("input");
  input.type = "search";
  input.placeholder = placeholder;
  input.dataset.role = role;

  field.append(labelNode, input);
  return field;
}

function createSelectControl(label, role, options, selectedValue) {
  const field = document.createElement("label");
  field.className = "control-field";

  const labelNode = document.createElement("span");
  labelNode.className = "control-label";
  labelNode.textContent = label;

  const select = document.createElement("select");
  select.dataset.role = role;

  for (const option of options) {
    const optionNode = document.createElement("option");
    optionNode.value = option.value;
    optionNode.textContent = option.label;
    if (option.value === selectedValue) {
      optionNode.selected = true;
    }
    select.appendChild(optionNode);
  }

  field.append(labelNode, select);
  return field;
}

function collectFilterValues(rows, getFilterValue) {
  return Array.from(new Set(rows.map((row) => fmt(getFilterValue(row)))))
    .sort((left, right) => compareTextValues(left, right));
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

function toNumberOrNull(value) {
  return typeof value === "number" && !Number.isNaN(value) ? value : null;
}

function compareNullableNumbers(left, right, direction = "asc") {
  const leftMissing = left === null || left === undefined;
  const rightMissing = right === null || right === undefined;

  if (leftMissing && rightMissing) {
    return 0;
  }
  if (leftMissing) {
    return 1;
  }
  if (rightMissing) {
    return -1;
  }

  return direction === "desc" ? right - left : left - right;
}

function compareTextValues(left, right) {
  return fmt(left).localeCompare(fmt(right), undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;")
    .replaceAll("'", "&#39;");
}
