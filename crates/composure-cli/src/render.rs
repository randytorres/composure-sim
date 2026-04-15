use crate::MarketingV2ComparisonReport;
use composure_calibration::CalibrationResult;
use composure_core::{
    CounterfactualResult, DeterministicReport, ExperimentBundle, ExperimentRunStatus,
    ParameterValue, RunSummary, SensitivityKind, SweepExecutionResult, TrajectoryComparison,
};
use composure_market::MarketSimulationResult;
use composure_marketing::{MarketingSimulationResultV2, MetricKind};

pub(crate) fn format_bundle(bundle: &ExperimentBundle) -> String {
    let completed = bundle
        .runs
        .iter()
        .filter(|run| run.status == ExperimentRunStatus::Completed)
        .count();
    let failed = bundle
        .runs
        .iter()
        .filter(|run| run.status == ExperimentRunStatus::Failed)
        .count();
    let running = bundle
        .runs
        .iter()
        .filter(|run| run.status == ExperimentRunStatus::Running)
        .count();
    let pending = bundle
        .runs
        .iter()
        .filter(|run| run.status == ExperimentRunStatus::Pending)
        .count();

    [
        format!("Bundle: {} ({})", bundle.spec.name, bundle.spec.id),
        format!(
            "Scenario: {} ({})",
            bundle.spec.scenario.name, bundle.spec.scenario.id
        ),
        format!("Time steps: {}", bundle.spec.scenario.time_steps),
        format!("Parameter sets: {}", bundle.parameter_sets.len()),
        format!("Runs: {}", bundle.runs.len()),
        format!(
            "Run states: completed={}, failed={}, running={}, pending={}",
            completed, failed, running, pending
        ),
    ]
    .join("\n")
}

pub(crate) fn format_sweep(result: &SweepExecutionResult) -> String {
    let top_sensitivity = result
        .sensitivity
        .as_ref()
        .and_then(|report| report.rankings.first())
        .map(|ranking| match &ranking.kind {
            SensitivityKind::Numeric(stats) => format!(
                "{} ({:?}, corr={:.4}, slope={:.4})",
                ranking.parameter, ranking.direction, stats.correlation, stats.slope
            ),
            SensitivityKind::Categorical(stats) => format!(
                "{} ({:?}, range={:.4}, buckets={})",
                ranking.parameter,
                ranking.direction,
                stats.range,
                stats.buckets.len()
            ),
        })
        .unwrap_or_else(|| "none".into());

    [
        format!(
            "Sweep: {} ({})",
            result.definition.name, result.definition.id
        ),
        format!("Strategy: {:?}", result.definition.strategy),
        format!("Configured samples: {:?}", result.definition.sample_count),
        format!("Seed: {:?}", result.definition.seed),
        format!("Executed cases: {}", result.executed_cases.len()),
        format!("Failures: {}", result.failures.len()),
        format!("Scored samples: {}", result.samples.len()),
        format!("Bundle attached: {}", result.bundle.is_some()),
        format!("Top sensitivity: {top_sensitivity}"),
    ]
    .join("\n")
}

pub(crate) fn format_summary(summary: &RunSummary) -> String {
    let mut lines = Vec::new();
    match &summary.monte_carlo {
        Some(monte_carlo) => {
            lines.push("Monte Carlo: present".into());
            lines.push(format!("  paths: {}", monte_carlo.num_paths));
            lines.push(format!("  time steps: {}", monte_carlo.time_steps));
            lines.push(format!("  final mean: {:?}", monte_carlo.end));
            lines.push(format!("  auc: {:?}", monte_carlo.auc));
            lines.push(format!(
                "  final band width: {:?}",
                monte_carlo.final_band_width
            ));
        }
        None => lines.push("Monte Carlo: none".into()),
    }

    match &summary.composure {
        Some(composure) => {
            lines.push("Composure: present".into());
            lines.push(format!("  archetype: {:?}", composure.archetype));
            lines.push(format!("  slope: {:.4}", composure.slope));
            lines.push(format!("  trough: {:.4}", composure.trough));
            lines.push(format!(
                "  residual damage: {:.4}",
                composure.residual_damage
            ));
        }
        None => lines.push("Composure: none".into()),
    }

    lines.join("\n")
}

pub(crate) fn format_comparison(comparison: &TrajectoryComparison) -> String {
    let failure = comparison
        .metrics
        .failure
        .as_ref()
        .map(|failure| {
            format!(
                "{:?} (baseline={:?}, candidate={:?}, shift={:?})",
                failure.outcome, failure.baseline_break_t, failure.candidate_break_t, failure.shift
            )
        })
        .unwrap_or_else(|| "none".into());
    let divergence = comparison
        .divergence
        .as_ref()
        .map(|divergence| {
            format!(
                "start={}, end={}, len={}, peak_abs_delta={:.4}",
                divergence.start_t, divergence.end_t, divergence.length, divergence.peak_abs_delta
            )
        })
        .unwrap_or_else(|| "none".into());

    [
        format!("Series length: {}", comparison.series_len),
        format!("Mean delta: {:.4}", comparison.metrics.mean_delta),
        format!("Mean abs delta: {:.4}", comparison.metrics.mean_abs_delta),
        format!("RMSE: {:.4}", comparison.metrics.rmse),
        format!("End delta: {:.4}", comparison.metrics.end_delta),
        format!("Divergence: {divergence}"),
        format!("Failure comparison: {failure}"),
        format!(
            "Best improvement: t={} delta={:.4}",
            comparison.metrics.max_improvement.t, comparison.metrics.max_improvement.delta
        ),
        format!(
            "Worst regression: t={} delta={:.4}",
            comparison.metrics.max_regression.t, comparison.metrics.max_regression.delta
        ),
    ]
    .join("\n")
}

pub(crate) fn format_report(report: &DeterministicReport) -> String {
    let mut lines = vec![
        format!("Start delta: {}", format_delta(&report.start_delta)),
        format!("End delta: {}", format_delta(&report.end_delta)),
        format!("AUC delta: {}", format_delta(&report.auc_delta)),
        format!(
            "Residual damage delta: {}",
            format_delta(&report.residual_damage_delta)
        ),
        format!(
            "Archetype change: baseline={:?}, candidate={:?}, changed={}",
            report.archetype_change.baseline,
            report.archetype_change.candidate,
            report.archetype_change.changed
        ),
        format!(
            "Break point shift: baseline={:?}, candidate={:?}, shift={:?}",
            report.break_point_shift.baseline,
            report.break_point_shift.candidate,
            report.break_point_shift.shift
        ),
        format!(
            "Recovery shift: baseline={:?}, candidate={:?}, shift={:?}",
            report.recovery_shift.baseline,
            report.recovery_shift.candidate,
            report.recovery_shift.shift
        ),
        format!(
            "Percentile band change: baseline={:?}, candidate={:?}, delta={:?}, direction={:?}",
            report.percentile_band_change.baseline,
            report.percentile_band_change.candidate,
            report.percentile_band_change.delta,
            report.percentile_band_change.direction
        ),
    ];

    match &report.comparison {
        Some(comparison) => {
            lines.push(format!(
                "Comparison: rmse={:.4}, mean_abs_delta={:.4}, end_delta={:.4}, divergence={:?}->{:?}, failure_shift={:?}",
                comparison.rmse,
                comparison.mean_abs_delta,
                comparison.end_delta,
                comparison.divergence_start_t,
                comparison.divergence_end_t,
                comparison.failure_shift
            ));
        }
        None => lines.push("Comparison: none".into()),
    }

    lines.join("\n")
}

pub(crate) fn format_counterfactual_result(result: &CounterfactualResult) -> String {
    [
        format!(
            "Counterfactual result: baseline={} ({}) vs candidate={} ({})",
            result.baseline.intervention_label,
            result.baseline.branch_id,
            result.candidate.intervention_label,
            result.candidate.branch_id
        ),
        format!("Baseline branch_from_t: {}", result.baseline.branch_from_t),
        format!(
            "Candidate branch_from_t: {}",
            result.candidate.branch_from_t
        ),
        format!("End delta: {:.4}", result.comparison.metrics.end_delta),
        format!(
            "Divergence: {:?}",
            result
                .comparison
                .divergence
                .as_ref()
                .map(|window| (window.start_t, window.end_t))
        ),
        format!(
            "Failure shift: {:?}",
            result
                .report
                .comparison
                .as_ref()
                .and_then(|comparison| comparison.failure_shift)
        ),
        "Baseline summary:".into(),
        format_summary(&result.baseline.summary),
        "Candidate summary:".into(),
        format_summary(&result.candidate.summary),
    ]
    .join("\n")
}

pub(crate) fn format_calibration(result: &CalibrationResult) -> String {
    let mut lines = vec![
        format!(
            "Calibration: {} ({})",
            result.definition.name, result.definition.id
        ),
        format!(
            "Observed: {} ({})",
            result.observed.name, result.observed.id
        ),
        format!("Objective: {:?}", result.config.objective),
        format!("Failure mode: {:?}", result.config.failure_mode),
        format!("Observed points: {}", result.observed.values.len()),
        format!("Candidates: {}", result.candidates.len()),
        format!("Failures: {}", result.failures.len()),
        format!("Bundle attached: {}", result.bundle.is_some()),
        format!("Best case: {:?}", result.best_case_id),
        format!("Best parameter set: {:?}", result.best_parameter_set_id),
        format!("Best score: {:?}", result.best_score),
    ];

    if let Some(candidate) = result.candidates.first() {
        lines.push(format!(
            "Top candidate: case={}, parameter_set={}, score={:.4}, rmse={:.4}, end_delta={:.4}",
            candidate.case.case_id,
            candidate.parameter_set.id,
            candidate.score,
            candidate.comparison.metrics.rmse,
            candidate.comparison.metrics.end_delta
        ));
    }

    lines.join("\n")
}

pub(crate) fn render_report_markdown(report: &DeterministicReport) -> String {
    let mut lines = vec![
        "# Deterministic Report".into(),
        "".into(),
        "## Scalar Deltas".into(),
        "| Metric | Baseline | Candidate | Delta |".into(),
        "| --- | ---: | ---: | ---: |".into(),
        format!(
            "| Start | {} | {} | {} |",
            markdown_f64(report.start_delta.baseline),
            markdown_f64(report.start_delta.candidate),
            markdown_f64(report.start_delta.delta)
        ),
        format!(
            "| End | {} | {} | {} |",
            markdown_f64(report.end_delta.baseline),
            markdown_f64(report.end_delta.candidate),
            markdown_f64(report.end_delta.delta)
        ),
        format!(
            "| AUC | {} | {} | {} |",
            markdown_f64(report.auc_delta.baseline),
            markdown_f64(report.auc_delta.candidate),
            markdown_f64(report.auc_delta.delta)
        ),
        format!(
            "| Residual damage | {} | {} | {} |",
            markdown_f64(report.residual_damage_delta.baseline),
            markdown_f64(report.residual_damage_delta.candidate),
            markdown_f64(report.residual_damage_delta.delta)
        ),
        "".into(),
        "## Structural Changes".into(),
        "| Metric | Baseline | Candidate | Change |".into(),
        "| --- | --- | --- | --- |".into(),
        format!(
            "| Archetype | {} | {} | {} |",
            markdown_debug(report.archetype_change.baseline),
            markdown_debug(report.archetype_change.candidate),
            if report.archetype_change.changed {
                "changed"
            } else {
                "unchanged"
            }
        ),
        format!(
            "| Break point | {} | {} | {} |",
            markdown_usize(report.break_point_shift.baseline),
            markdown_usize(report.break_point_shift.candidate),
            markdown_isize(report.break_point_shift.shift)
        ),
        format!(
            "| Recovery half-life | {} | {} | {} |",
            markdown_usize(report.recovery_shift.baseline),
            markdown_usize(report.recovery_shift.candidate),
            markdown_isize(report.recovery_shift.shift)
        ),
        format!(
            "| Final percentile band | {} | {} | {} ({:?}) |",
            markdown_f64(report.percentile_band_change.baseline),
            markdown_f64(report.percentile_band_change.candidate),
            markdown_f64(report.percentile_band_change.delta),
            report.percentile_band_change.direction
        ),
        "".into(),
        "## Comparison Snapshot".into(),
    ];

    match &report.comparison {
        Some(comparison) => {
            lines.extend([
                "| Metric | Value |".into(),
                "| --- | ---: |".into(),
                format!("| Mean delta | {:.4} |", comparison.mean_delta),
                format!("| Mean abs delta | {:.4} |", comparison.mean_abs_delta),
                format!("| RMSE | {:.4} |", comparison.rmse),
                format!("| End delta | {:.4} |", comparison.end_delta),
                format!(
                    "| Divergence start | {} |",
                    markdown_usize(comparison.divergence_start_t)
                ),
                format!(
                    "| Divergence end | {} |",
                    markdown_usize(comparison.divergence_end_t)
                ),
                format!(
                    "| Failure shift | {} |",
                    markdown_isize(comparison.failure_shift)
                ),
            ]);
        }
        None => lines.push("No trajectory comparison attached.".into()),
    }

    lines.join("\n")
}

pub(crate) fn render_bundle_markdown(bundle: &ExperimentBundle) -> String {
    let completed = bundle
        .runs
        .iter()
        .filter(|run| run.status == ExperimentRunStatus::Completed)
        .count();
    let failed = bundle
        .runs
        .iter()
        .filter(|run| run.status == ExperimentRunStatus::Failed)
        .count();
    let running = bundle
        .runs
        .iter()
        .filter(|run| run.status == ExperimentRunStatus::Running)
        .count();
    let pending = bundle
        .runs
        .iter()
        .filter(|run| run.status == ExperimentRunStatus::Pending)
        .count();

    let mut lines = vec![
        format!(
            "# Experiment Bundle: {} ({})",
            bundle.spec.name, bundle.spec.id
        ),
        String::new(),
        format!(
            "- Scenario: `{}` (`{}`)",
            bundle.spec.scenario.name, bundle.spec.scenario.id
        ),
        format!(
            "- Description: {}",
            option_to_string(bundle.spec.description.clone())
        ),
        format!("- Time steps: `{}`", bundle.spec.scenario.time_steps),
        format!(
            "- Failure threshold: `{}`",
            markdown_f64(bundle.spec.scenario.failure_threshold)
        ),
        format!(
            "- Default Monte Carlo: `{}`",
            bundle
                .spec
                .default_monte_carlo
                .as_ref()
                .map(format_monte_carlo_config)
                .unwrap_or_else(|| "n/a".into())
        ),
        format!(
            "- Tags: `{}`",
            if bundle.spec.tags.is_empty() {
                "n/a".into()
            } else {
                bundle.spec.tags.join(", ")
            }
        ),
        format!("- Created at (unix s): `{}`", bundle.spec.created_at_unix_s),
        format!("- Parameter sets: `{}`", bundle.parameter_sets.len()),
        format!("- Runs: `{}`", bundle.runs.len()),
        String::new(),
        "## Run States".into(),
        "| Completed | Failed | Running | Pending |".into(),
        "| --- | --- | --- | --- |".into(),
        format!("| {} | {} | {} | {} |", completed, failed, running, pending),
        String::new(),
        "## Parameter Sets".into(),
    ];

    if bundle.parameter_sets.is_empty() {
        lines.push("No parameter sets recorded.".into());
    } else {
        lines.push("| ID | Name | Time Steps | Monte Carlo |".into());
        lines.push("| --- | --- | --- | --- |".into());
        for parameter_set in &bundle.parameter_sets {
            lines.push(format!(
                "| {} | {} | {} | {} |",
                parameter_set.id,
                parameter_set.name,
                parameter_set.scenario.time_steps,
                parameter_set
                    .monte_carlo
                    .as_ref()
                    .map(format_monte_carlo_config)
                    .unwrap_or_else(|| "n/a".into())
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Runs".into());

    if bundle.runs.is_empty() {
        lines.push("No runs recorded.".into());
    } else {
        lines.push(
            "| Run ID | Status | Parameter Set | Seed | Monte Carlo | Composure | Error |".into(),
        );
        lines.push("| --- | --- | --- | --- | --- | --- | --- |".into());
        for run in &bundle.runs {
            let outcome = run.outcome.as_ref();
            lines.push(format!(
                "| {} | {:?} | {} | {} | {} | {} | {} |",
                run.run_id,
                run.status,
                option_to_string(run.parameter_set_id.clone()),
                option_to_string(run.seed),
                bool_cell(
                    outcome
                        .and_then(|outcome| outcome.monte_carlo.as_ref())
                        .is_some()
                ),
                bool_cell(
                    outcome
                        .and_then(|outcome| outcome.composure.as_ref())
                        .is_some()
                ),
                option_to_string(run.error.clone())
            ));
        }
    }

    lines.join("\n")
}

pub(crate) fn render_sweep_summary_markdown(result: &SweepExecutionResult) -> String {
    let best_case_id = result
        .sensitivity
        .as_ref()
        .map(|report| report.objective.best_case_id.as_str())
        .unwrap_or("n/a");
    let best_objective = result
        .samples
        .iter()
        .max_by(|a, b| {
            a.objective
                .partial_cmp(&b.objective)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|sample| sample.objective);

    let mut ranked_samples = result.samples.iter().collect::<Vec<_>>();
    ranked_samples.sort_by(|left, right| {
        right
            .objective
            .partial_cmp(&left.objective)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut lines = vec![
        format!(
            "# Sweep Summary: {} ({})",
            result.definition.name, result.definition.id
        ),
        String::new(),
        format!("- Strategy: `{:?}`", result.definition.strategy),
        format!(
            "- Configured samples: `{}`",
            option_to_string(result.definition.sample_count)
        ),
        format!("- Seed: `{}`", option_to_string(result.definition.seed)),
        format!("- Parameters: `{}`", result.definition.parameters.len()),
        format!("- Executed cases: `{}`", result.executed_cases.len()),
        format!("- Failures: `{}`", result.failures.len()),
        format!("- Scored samples: `{}`", result.samples.len()),
        format!("- Bundle attached: `{}`", result.bundle.is_some()),
        String::new(),
        "## Objective".into(),
        "| Best Case | Best Objective | Worst Case | Mean Objective |".into(),
        "| --- | --- | --- | --- |".into(),
        format!(
            "| {} | {} | {} | {} |",
            best_case_id,
            best_objective
                .map(|value| format!("{value:.4}"))
                .unwrap_or_else(|| "n/a".into()),
            result
                .sensitivity
                .as_ref()
                .map(|report| report.objective.worst_case_id.as_str())
                .unwrap_or("n/a"),
            result
                .sensitivity
                .as_ref()
                .map(|report| format!("{:.4}", report.objective.mean))
                .unwrap_or_else(|| "n/a".into())
        ),
        String::new(),
        "## Top Samples".into(),
    ];

    if ranked_samples.is_empty() {
        lines.push("No scored samples available.".into());
    } else {
        lines.push("| Case | Objective | Run | Parameter Set | Parameters |".into());
        lines.push("| --- | --- | --- | --- | --- |".into());
        for sample in ranked_samples.into_iter().take(5) {
            lines.push(format!(
                "| {} | {:.4} | {} | {} | {} |",
                sample.case_id,
                sample.objective,
                metadata_string(sample.metadata.as_ref(), "run_id"),
                metadata_string(sample.metadata.as_ref(), "parameter_set_id"),
                format_parameter_map(&sample.parameters)
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Sensitivity Rankings".into());

    match &result.sensitivity {
        Some(report) if !report.rankings.is_empty() => {
            lines.push("| Parameter | Direction | Score | Detail |".into());
            lines.push("| --- | --- | --- | --- |".into());
            for ranking in &report.rankings {
                lines.push(format!(
                    "| {} | {:?} | {:.4} | {} |",
                    ranking.parameter,
                    ranking.direction,
                    ranking.score,
                    format_sensitivity_kind(&ranking.kind)
                ));
            }
        }
        _ => lines.push("No sensitivity report attached.".into()),
    }

    if !result.failures.is_empty() {
        lines.push(String::new());
        lines.push("## Failures".into());
        lines.push("| Case | Parameter Set | Error |".into());
        lines.push("| --- | --- | --- |".into());
        for failure in &result.failures {
            lines.push(format!(
                "| {} | {} | {} |",
                failure.case.case_id,
                option_to_string(failure.parameter_set_id.clone()),
                failure.error
            ));
        }
    }

    lines.join("\n")
}

pub(crate) fn render_sweep_csv(result: &SweepExecutionResult) -> String {
    let best_case_id = result
        .sensitivity
        .as_ref()
        .map(|report| report.objective.best_case_id.as_str());
    let parameter_names = collect_parameter_names(
        result
            .samples
            .iter()
            .map(|sample| &sample.parameters)
            .collect::<Vec<_>>()
            .as_slice(),
    );

    let mut rows = Vec::with_capacity(result.samples.len() + 1);
    let mut header = vec![
        "case_id".into(),
        "is_best".into(),
        "objective".into(),
        "run_id".into(),
        "parameter_set_id".into(),
    ];
    header.extend(parameter_names.iter().cloned());
    rows.push(csv_row(&header));

    for sample in &result.samples {
        let mut fields = vec![
            sample.case_id.clone(),
            (best_case_id == Some(sample.case_id.as_str())).to_string(),
            format!("{:.12}", sample.objective),
            metadata_string(sample.metadata.as_ref(), "run_id"),
            metadata_string(sample.metadata.as_ref(), "parameter_set_id"),
        ];
        fields.extend(
            parameter_names
                .iter()
                .map(|name| parameter_string(sample.parameters.get(name))),
        );
        rows.push(csv_row(&fields));
    }

    rows.join("\n")
}

pub(crate) fn render_sweep_markdown(result: &SweepExecutionResult) -> String {
    let best_case_id = result
        .sensitivity
        .as_ref()
        .map(|report| report.objective.best_case_id.as_str());
    let parameter_names = collect_parameter_names(
        result
            .samples
            .iter()
            .map(|sample| &sample.parameters)
            .collect::<Vec<_>>()
            .as_slice(),
    );

    let mut lines = vec![
        format!(
            "# Sweep Samples: {} ({})",
            result.definition.name, result.definition.id
        ),
        "".into(),
        format!("- Strategy: `{:?}`", result.definition.strategy),
        format!("- Scored samples: `{}`", result.samples.len()),
        format!("- Failures: `{}`", result.failures.len()),
        format!("- Bundle attached: `{}`", result.bundle.is_some()),
        "".into(),
        "## Samples".into(),
    ];

    let mut header = vec![
        "Case".into(),
        "Best".into(),
        "Objective".into(),
        "Run".into(),
        "Parameter Set".into(),
    ];
    header.extend(parameter_names.iter().cloned());

    lines.push(markdown_row(&header));
    lines.push(markdown_separator(header.len()));

    for sample in &result.samples {
        let mut row = vec![
            sample.case_id.clone(),
            if best_case_id == Some(sample.case_id.as_str()) {
                "yes".into()
            } else {
                String::new()
            },
            format!("{:.4}", sample.objective),
            metadata_string(sample.metadata.as_ref(), "run_id"),
            metadata_string(sample.metadata.as_ref(), "parameter_set_id"),
        ];
        row.extend(
            parameter_names
                .iter()
                .map(|name| parameter_string(sample.parameters.get(name))),
        );
        lines.push(markdown_row(&row));
    }

    lines.join("\n")
}

pub(crate) fn render_calibration_csv(result: &CalibrationResult) -> String {
    let best_case_id = result.best_case_id.as_deref();
    let best_parameter_set_id = result.best_parameter_set_id.as_deref();
    let parameter_names = collect_parameter_names(
        result
            .candidates
            .iter()
            .map(|candidate| &candidate.case.parameters)
            .collect::<Vec<_>>()
            .as_slice(),
    );

    let mut rows = Vec::with_capacity(result.candidates.len() + 1);
    let mut header = vec![
        "rank".into(),
        "is_best".into(),
        "case_id".into(),
        "parameter_set_id".into(),
        "run_id".into(),
        "score".into(),
        "rmse".into(),
        "mean_abs_delta".into(),
        "end_delta".into(),
        "divergence_start_t".into(),
        "divergence_end_t".into(),
        "failure_shift".into(),
    ];
    header.extend(parameter_names.iter().cloned());
    rows.push(csv_row(&header));

    for (index, candidate) in result.candidates.iter().enumerate() {
        let is_best_case = best_case_id
            .map(|id| id == candidate.case.case_id)
            .unwrap_or(false);
        let is_best_parameter_set = best_parameter_set_id
            .map(|id| id == candidate.parameter_set.id)
            .unwrap_or(false);
        let is_best = match (best_case_id, best_parameter_set_id) {
            (Some(_), Some(_)) => is_best_case && is_best_parameter_set,
            (Some(_), None) => is_best_case,
            (None, Some(_)) => is_best_parameter_set,
            (None, None) => false,
        };

        let mut fields = vec![
            (index + 1).to_string(),
            is_best.to_string(),
            candidate.case.case_id.clone(),
            candidate.parameter_set.id.clone(),
            candidate.run.run_id.clone(),
            format!("{:.12}", candidate.score),
            format!("{:.12}", candidate.comparison.metrics.rmse),
            format!("{:.12}", candidate.comparison.metrics.mean_abs_delta),
            format!("{:.12}", candidate.comparison.metrics.end_delta),
            option_to_string(
                candidate
                    .report
                    .comparison
                    .as_ref()
                    .and_then(|c| c.divergence_start_t),
            ),
            option_to_string(
                candidate
                    .report
                    .comparison
                    .as_ref()
                    .and_then(|c| c.divergence_end_t),
            ),
            option_to_string(
                candidate
                    .report
                    .comparison
                    .as_ref()
                    .and_then(|c| c.failure_shift),
            ),
        ];
        fields.extend(
            parameter_names
                .iter()
                .map(|name| parameter_string(candidate.case.parameters.get(name))),
        );
        rows.push(csv_row(&fields));
    }

    rows.join("\n")
}

pub(crate) fn render_calibration_markdown(result: &CalibrationResult) -> String {
    let best_case_id = result.best_case_id.as_deref();
    let best_parameter_set_id = result.best_parameter_set_id.as_deref();
    let parameter_names = collect_parameter_names(
        result
            .candidates
            .iter()
            .map(|candidate| &candidate.case.parameters)
            .collect::<Vec<_>>()
            .as_slice(),
    );

    let mut lines = vec![
        format!(
            "# Calibration Candidates: {} ({})",
            result.definition.name, result.definition.id
        ),
        "".into(),
        format!("- Objective: `{:?}`", result.config.objective),
        format!("- Failure mode: `{:?}`", result.config.failure_mode),
        format!("- Candidates: `{}`", result.candidates.len()),
        format!("- Failures: `{}`", result.failures.len()),
        format!(
            "- Best case: `{}`",
            result.best_case_id.as_deref().unwrap_or("n/a")
        ),
        format!(
            "- Best parameter set: `{}`",
            result.best_parameter_set_id.as_deref().unwrap_or("n/a")
        ),
        "".into(),
        "## Candidates".into(),
    ];

    let mut header = vec![
        "Rank".into(),
        "Best".into(),
        "Case".into(),
        "Parameter Set".into(),
        "Run".into(),
        "Score".into(),
        "RMSE".into(),
        "Mean Abs Delta".into(),
        "End Delta".into(),
    ];
    header.extend(parameter_names.iter().cloned());

    lines.push(markdown_row(&header));
    lines.push(markdown_separator(header.len()));

    for (index, candidate) in result.candidates.iter().enumerate() {
        let is_best_case = best_case_id
            .map(|id| id == candidate.case.case_id)
            .unwrap_or(false);
        let is_best_parameter_set = best_parameter_set_id
            .map(|id| id == candidate.parameter_set.id)
            .unwrap_or(false);
        let is_best = match (best_case_id, best_parameter_set_id) {
            (Some(_), Some(_)) => is_best_case && is_best_parameter_set,
            (Some(_), None) => is_best_case,
            (None, Some(_)) => is_best_parameter_set,
            (None, None) => false,
        };

        let mut row = vec![
            (index + 1).to_string(),
            if is_best { "yes".into() } else { String::new() },
            candidate.case.case_id.clone(),
            candidate.parameter_set.id.clone(),
            candidate.run.run_id.clone(),
            format!("{:.4}", candidate.score),
            format!("{:.4}", candidate.comparison.metrics.rmse),
            format!("{:.4}", candidate.comparison.metrics.mean_abs_delta),
            format!("{:.4}", candidate.comparison.metrics.end_delta),
        ];
        row.extend(
            parameter_names
                .iter()
                .map(|name| parameter_string(candidate.case.parameters.get(name))),
        );
        lines.push(markdown_row(&row));
    }

    lines.join("\n")
}

pub(crate) fn render_marketing_v2_report_markdown(result: &MarketingSimulationResultV2) -> String {
    let mut lines = vec![
        "# Marketing Simulation Report".into(),
        String::new(),
        format!("Simulation ID: `{}`", result.simulation_id),
        format!("Scenario: `{}`", result.scenario.name),
        format!("Scenario Type: `{:?}`", result.scenario.scenario_type),
        format!("Approaches: {}", result.approach_results.len()),
        String::new(),
        "## Overview".into(),
    ];

    for insight in &result.cross_approach_insights {
        lines.push(format!("- {insight}"));
    }
    for summary in &result.calibration_summary {
        lines.push(format!("- Calibration: {summary}"));
    }
    for recommendation in &result.recommended_next_experiments {
        lines.push(format!("- Next: {recommendation}"));
    }
    lines.push(String::new());

    if let Some(analysis) = &result.llm_analysis {
        lines.push("## LLM Analysis".into());
        lines.push(String::new());
        lines.push(format!(
            "- Model: `{}`{}{}",
            analysis.model,
            analysis
                .provider
                .as_ref()
                .map(|provider| format!(" via `{provider}`"))
                .unwrap_or_default(),
            analysis
                .reasoning_effort
                .as_ref()
                .map(|effort| format!(" (`{effort}` reasoning)"))
                .unwrap_or_default()
        ));
        lines.push(format!(
            "- Evaluator passes: `{}`",
            analysis.evaluator_count
        ));
        for item in &analysis.executive_summary {
            lines.push(format!("- Executive summary: {item}"));
        }
        for item in &analysis.consensus_summary {
            lines.push(format!("- Consensus: {item}"));
        }
        for item in &analysis.strategic_takeaways {
            lines.push(format!("- Strategic takeaway: {item}"));
        }
        for item in &analysis.confidence_notes {
            lines.push(format!("- Confidence note: {item}"));
        }
        for item in &analysis.disagreement_notes {
            lines.push(format!("- Disagreement note: {item}"));
        }
        lines.push(String::new());
    }

    if let Some(trace) = &result.llm_trace {
        lines.push("## LLM Evidence".into());
        lines.push(String::new());
        if let Some(goal) = &trace.analysis_goal {
            lines.push(format!("- Analysis goal: {goal}"));
        }
        lines.push(format!("- Prompt chars: `{}`", trace.prompt_char_count));
        lines.push(format!(
            "- Evaluators captured: `{}`",
            trace.evaluators.len()
        ));
        lines.push(String::new());
        lines.push("### System Prompt".into());
        lines.push(String::new());
        push_code_block(
            &mut lines,
            "text",
            &truncate_for_markdown(&trace.system_prompt, 1600),
        );
        lines.push(String::new());
        lines.push("### User Prompt".into());
        lines.push(String::new());
        push_code_block(
            &mut lines,
            "text",
            &truncate_for_markdown(&trace.user_prompt, 2400),
        );
        lines.push(String::new());

        for evaluator in &trace.evaluators {
            lines.push(format!("### Evaluator {}", evaluator.evaluator_index));
            lines.push(String::new());
            lines.push(format!(
                "- Model: `{}`{}{}",
                evaluator.model,
                evaluator
                    .provider
                    .as_ref()
                    .map(|provider| format!(" via `{provider}`"))
                    .unwrap_or_default(),
                evaluator
                    .reasoning_effort
                    .as_ref()
                    .map(|effort| format!(" (`{effort}` reasoning)"))
                    .unwrap_or_default()
            ));
            lines.push(format!("- Base URL: `{}`", evaluator.base_url));
            lines.push(format!("- Duration: `{}` ms", evaluator.duration_ms));
            if let Some(response_id) = &evaluator.response_id {
                lines.push(format!("- Response ID: `{response_id}`"));
            }
            lines.push(format!(
                "- Stream fallback used: `{}`",
                if evaluator.stream_fallback_used {
                    "yes"
                } else {
                    "no"
                }
            ));
            if let Some(usage) = &evaluator.usage {
                lines.push(format!("- Usage: {}", format_llm_usage(usage)));
            }
            lines.push(String::new());
            lines.push("- Parsed output preview:".into());
            push_code_block(
                &mut lines,
                "json",
                &truncate_for_markdown(
                    &serde_json::to_string_pretty(
                        evaluator
                            .parsed_output
                            .as_ref()
                            .unwrap_or(&serde_json::Value::Null),
                    )
                    .unwrap_or_else(|_| "{}".into()),
                    2400,
                ),
            );
            lines.push(String::new());
            lines.push("- Raw output preview:".into());
            push_code_block(
                &mut lines,
                "text",
                &truncate_for_markdown(&evaluator.raw_output_text, 1800),
            );
            if let Some(raw_stream_text) = evaluator
                .raw_response
                .get("raw_stream_text")
                .and_then(serde_json::Value::as_str)
            {
                lines.push(String::new());
                lines.push("- Raw stream preview:".into());
                push_code_block(
                    &mut lines,
                    "text",
                    &truncate_for_markdown(raw_stream_text, 1800),
                );
            }
            lines.push(String::new());
        }
    }

    if let Some(winner) = result
        .approach_results
        .iter()
        .max_by_key(|approach| approach.primary_scorecard.overall_score)
    {
        lines.push("## Winner".into());
        lines.push(String::new());
        lines.push(format!(
            "- Approach: `{}` with overall score `{}`",
            winner.approach_id, winner.primary_scorecard.overall_score
        ));
        lines.push(format!("- Engagement: `{}`", winner.engagement_score));
        lines.push(format!("- Viral potential: `{}`", winner.viral_potential));
        for reason in &winner.win_reasons {
            lines.push(format!("- Why it won: {reason}"));
        }
        for risk in &winner.loss_risks {
            lines.push(format!("- Main risk: {risk}"));
        }
        lines.push(String::new());
    }

    lines.push("## Approach Leaderboard".into());
    lines.push(String::new());
    lines.push("| Rank | Approach | Overall | Engagement | Viral | Top Metric |".into());
    lines.push("|---|---|---:|---:|---:|---|".into());
    let mut ranked = result.approach_results.iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .primary_scorecard
            .overall_score
            .cmp(&left.primary_scorecard.overall_score)
    });
    for (index, approach) in ranked.iter().enumerate() {
        let top_metric = approach
            .primary_scorecard
            .metrics
            .iter()
            .max_by_key(|metric| metric.score)
            .map(|metric| metric.label.clone())
            .unwrap_or_else(|| "n/a".into());
        lines.push(format!(
            "| {} | `{}` | {} | {} | {} | {} |",
            index + 1,
            approach.approach_id,
            approach.primary_scorecard.overall_score,
            approach.engagement_score,
            approach.viral_potential,
            top_metric
        ));
    }
    lines.push(String::new());

    let persona_rows = build_persona_rows(result);
    if !persona_rows.is_empty() {
        lines.push("## Persona Leaderboard".into());
        lines.push(String::new());
        lines.push("| Rank | Persona | Approach | Score | Weight |".into());
        lines.push("|---|---|---|---:|---:|".into());
        for (index, row) in persona_rows.iter().enumerate() {
            lines.push(format!(
                "| {} | {} | `{}` | {} | {:.2} |",
                index + 1,
                row.0,
                row.1,
                row.2,
                row.3
            ));
        }
        lines.push(String::new());
    }

    let repeated_concerns = build_repeated_concerns(result);
    if !repeated_concerns.is_empty() {
        lines.push("## Repeated Concerns".into());
        lines.push(String::new());
        lines.push("| Concern | Count | Approaches |".into());
        lines.push("|---|---:|---|".into());
        for (concern, count, approaches) in repeated_concerns {
            lines.push(format!(
                "| {} | {} | {} |",
                concern,
                count,
                approaches.join(", ")
            ));
        }
        lines.push(String::new());
    }

    lines.push("## Confidence Notes".into());
    lines.push(String::new());
    for approach in &ranked {
        lines.push(format!("### `{}`", approach.approach_id));
        for note in &approach.confidence_notes {
            lines.push(format!("- {note}"));
        }
    }
    lines.push(String::new());

    if !result.calibration_summary.is_empty()
        || result
            .approach_results
            .iter()
            .any(|approach| !approach.calibration_notes.is_empty())
    {
        lines.push("## Calibration Notes".into());
        lines.push(String::new());
        for item in &result.calibration_summary {
            lines.push(format!("- {item}"));
        }
        for approach in &ranked {
            if !approach.calibration_notes.is_empty() {
                lines.push(format!("### `{}`", approach.approach_id));
                for note in &approach.calibration_notes {
                    lines.push(format!("- {note}"));
                }
            }
        }
        lines.push(String::new());
    }

    lines.push("## Recommended Next Experiments".into());
    lines.push(String::new());
    for item in &result.recommended_next_experiments {
        lines.push(format!("- {item}"));
    }
    lines.push(String::new());

    lines.push("## Approach Notes".into());
    lines.push(String::new());
    for approach in ranked {
        lines.push(format!("### `{}`", approach.approach_id));
        lines.push(format!(
            "- Overall / engagement / viral: `{}` / `{}` / `{}`",
            approach.primary_scorecard.overall_score,
            approach.engagement_score,
            approach.viral_potential
        ));
        if let Some(metric) = approach
            .primary_scorecard
            .metrics
            .iter()
            .find(|metric| metric.metric == MetricKind::ObjectionPressure)
        {
            lines.push(format!("- Objection pressure: `{}`", metric.score));
        }
        for reason in &approach.win_reasons {
            lines.push(format!("- Win reason: {reason}"));
        }
        for risk in &approach.loss_risks {
            lines.push(format!("- Loss risk: {risk}"));
        }
        if let Some(analysis) = &approach.llm_analysis {
            lines.push(format!("- LLM narrative: {}", analysis.narrative));
            for item in &analysis.consensus_summary {
                lines.push(format!("- LLM consensus: {item}"));
            }
            for persona in &analysis.strongest_personas {
                lines.push(format!("- LLM strongest persona: {persona}"));
            }
            for item in &analysis.objections_to_resolve {
                lines.push(format!("- LLM objection to resolve: {item}"));
            }
            for item in &analysis.realism_warnings {
                lines.push(format!("- LLM realism warning: {item}"));
            }
            for item in &analysis.next_experiments {
                lines.push(format!("- LLM next experiment: {item}"));
            }
            for item in &analysis.disagreement_notes {
                lines.push(format!("- LLM disagreement note: {item}"));
            }
        }
        if !approach.persona_results.is_empty() {
            let mut persona_ranked = approach.persona_results.iter().collect::<Vec<_>>();
            persona_ranked.sort_by(|left, right| {
                right
                    .primary_scorecard
                    .overall_score
                    .cmp(&left.primary_scorecard.overall_score)
            });
            let top = persona_ranked[0];
            lines.push(format!(
                "- Best persona: {} at `{}`",
                top.persona_name, top.primary_scorecard.overall_score
            ));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

pub(crate) fn render_marketing_v2_compare_markdown(report: &MarketingV2ComparisonReport) -> String {
    let mut lines = vec![
        "# Marketing V2 Comparison".into(),
        String::new(),
        format!("Comparison ID: `{}`", report.comparison_id),
        format!("Scenarios: {}", report.scenarios.len()),
    ];

    if let Some(model) = &report.model {
        lines.push(format!(
            "Evaluator: `{}`{}{}",
            model,
            report
                .provider
                .as_ref()
                .map(|provider| format!(" via `{provider}`"))
                .unwrap_or_default(),
            report
                .reasoning_effort
                .as_ref()
                .map(|effort| format!(" (`{effort}` reasoning)"))
                .unwrap_or_default()
        ));
    }

    lines.push(String::new());
    if !report.portfolio_recommendation.is_empty() {
        lines.push("## Portfolio Recommendation".into());
        lines.push(String::new());
        for item in &report.portfolio_recommendation {
            lines.push(format!("- {item}"));
        }
        lines.push(String::new());
    }

    if !report.repeated_winner_patterns.is_empty() {
        lines.push("## Repeated Winner Patterns".into());
        lines.push(String::new());
        for item in &report.repeated_winner_patterns {
            lines.push(format!("- {item}"));
        }
        lines.push(String::new());
    }

    lines.push("## Leaderboard".into());
    lines.push(String::new());
    lines.push("| Rank | Scenario | Type | Overall | Winner | Winner Score | Gap | Strongest Metric | Best Delta | Weakest Delta |".into());
    lines.push("|---|---|---|---:|---|---:|---:|---|---|---|".into());
    for (index, scenario) in report.scenarios.iter().enumerate() {
        lines.push(format!(
            "| {} | {} | {} | {} | `{}` | {} | {} | {} | {} | {} |",
            index + 1,
            scenario.scenario_name,
            scenario.scenario_type,
            scenario.overall_score,
            scenario.winner_approach_id,
            scenario.winner_overall_score,
            scenario
                .score_gap_vs_runner_up
                .map(|gap| gap.to_string())
                .unwrap_or_else(|| "n/a".into()),
            scenario.strongest_metric_label.as_deref().unwrap_or("n/a"),
            scenario
                .strongest_positive_delta_metric
                .as_ref()
                .zip(scenario.strongest_positive_delta_value)
                .map(|(metric, delta)| format!("{metric} ({delta:+})"))
                .unwrap_or_else(|| "n/a".into()),
            scenario
                .weakest_delta_metric
                .as_ref()
                .zip(scenario.weakest_delta_value)
                .map(|(metric, delta)| format!("{metric} ({delta:+})"))
                .unwrap_or_else(|| "n/a".into())
        ));
    }
    lines.push(String::new());

    if report
        .scenarios
        .iter()
        .any(|scenario| !scenario.metric_deltas.is_empty())
    {
        let metric_labels = comparison_metric_labels(report);
        let displayed_labels = metric_labels.iter().take(8).cloned().collect::<Vec<_>>();

        if !displayed_labels.is_empty() {
            lines.push("## Cross-Scenario Metric Delta Matrix".into());
            lines.push(String::new());
            lines.push(
                "Each delta is measured against the compare-set average for that metric, using each scenario's aggregate primary scorecard."
                    .into(),
            );
            lines.push(String::new());

            let mut header = vec!["Metric".to_string(), "Max Swing".to_string()];
            header.extend(
                report
                    .scenarios
                    .iter()
                    .map(|scenario| scenario.scenario_name.clone()),
            );
            lines.push(format!("| {} |", header.join(" | ")));
            lines.push(format!(
                "|{}|",
                std::iter::repeat("---")
                    .take(header.len())
                    .collect::<Vec<_>>()
                    .join("|")
            ));

            for label in displayed_labels {
                let values = report
                    .scenarios
                    .iter()
                    .map(|scenario| {
                        scenario
                            .metric_deltas
                            .iter()
                            .find(|metric| metric.label == label)
                            .map(|metric| metric.delta_vs_compare_average)
                    })
                    .collect::<Vec<_>>();
                let max_swing = metric_delta_swing(values.iter().flatten().copied());
                let mut row = vec![label, max_swing.to_string()];
                row.extend(values.into_iter().map(|value| {
                    value
                        .map(markdown_signed_delta)
                        .unwrap_or_else(|| "n/a".into())
                }));
                lines.push(format!("| {} |", row.join(" | ")));
            }
            lines.push(String::new());

            if metric_labels.len() > 8 {
                lines.push(format!(
                    "_Showing the 8 metrics with the largest cross-scenario swings out of {} total._",
                    metric_labels.len()
                ));
                lines.push(String::new());
            }

            lines.push("## Metric Delta Leaders".into());
            lines.push(String::new());
            lines.push("| Metric | Most Above Average | Most Below Average |".into());
            lines.push("|---|---|---|".into());
            for label in metric_labels.iter().take(8) {
                let mut ranked = report
                    .scenarios
                    .iter()
                    .filter_map(|scenario| {
                        scenario
                            .metric_deltas
                            .iter()
                            .find(|metric| metric.label == *label)
                            .map(|metric| {
                                (
                                    scenario.scenario_name.as_str(),
                                    metric.delta_vs_compare_average,
                                )
                            })
                    })
                    .collect::<Vec<_>>();
                ranked
                    .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
                let strongest = ranked
                    .first()
                    .map(|(name, delta)| format!("{name} ({:+})", delta))
                    .unwrap_or_else(|| "n/a".into());
                let weakest = ranked
                    .last()
                    .map(|(name, delta)| format!("{name} ({:+})", delta))
                    .unwrap_or_else(|| "n/a".into());
                lines.push(format!("| {} | {} | {} |", label, strongest, weakest));
            }
            lines.push(String::new());
        }
    }

    lines.push("## Scenario Notes".into());
    lines.push(String::new());
    for scenario in &report.scenarios {
        lines.push(format!("### {}", scenario.scenario_name));
        lines.push(format!("- Request: `{}`", scenario.request_path));
        lines.push(format!("- Scenario type: `{}`", scenario.scenario_type));
        lines.push(format!("- Overall score: `{}`", scenario.overall_score));
        lines.push(format!(
            "- Winner: `{}` at `{}`",
            scenario.winner_approach_id, scenario.winner_overall_score
        ));
        if let Some(runner_up) = &scenario.runner_up_approach_id {
            lines.push(format!(
                "- Runner-up: `{}` at `{}`",
                runner_up,
                scenario
                    .runner_up_overall_score
                    .map(|score| score.to_string())
                    .unwrap_or_else(|| "n/a".into())
            ));
        }
        if let Some(gap) = scenario.score_gap_vs_runner_up {
            lines.push(format!("- Gap vs runner-up: `{}`", gap));
        }
        if let Some(metric) = &scenario.strongest_metric_label {
            lines.push(format!(
                "- Strongest metric: `{}`{}",
                metric,
                scenario
                    .strongest_metric_score
                    .map(|score| format!(" at `{score}`"))
                    .unwrap_or_default()
            ));
        }
        if let (Some(metric), Some(delta)) = (
            &scenario.strongest_positive_delta_metric,
            scenario.strongest_positive_delta_value,
        ) {
            lines.push(format!(
                "- Strongest cross-scenario delta: `{}` at `{:+}` vs compare average",
                metric, delta
            ));
        }
        if let (Some(metric), Some(delta)) =
            (&scenario.weakest_delta_metric, scenario.weakest_delta_value)
        {
            lines.push(format!(
                "- Weakest cross-scenario delta: `{}` at `{:+}` vs compare average",
                metric, delta
            ));
        }
        if !scenario.metric_deltas.is_empty() {
            lines.push("- Metric deltas vs compare set:".into());
            lines.push("| Metric | Score | Vs Avg | Vs Leader | Rank | Leader(s) |".into());
            lines.push("|---|---:|---:|---:|---:|---|".into());
            for metric in scenario.metric_deltas.iter().take(6) {
                lines.push(format!(
                    "| {} | {} | {:+} | {:+} | {}/{} | {} |",
                    metric.label,
                    metric.score,
                    metric.delta_vs_compare_average,
                    metric.delta_vs_compare_leader,
                    metric.compare_set_rank,
                    metric.compare_set_size,
                    if metric.leading_scenarios.is_empty() {
                        "n/a".into()
                    } else {
                        metric.leading_scenarios.join(", ")
                    }
                ));
            }
        }
        for item in &scenario.llm_executive_summary {
            lines.push(format!("- Executive summary: {item}"));
        }
        for item in &scenario.llm_consensus_summary {
            lines.push(format!("- Consensus: {item}"));
        }
        for item in &scenario.recommended_next_experiments {
            lines.push(format!("- Next experiment: {item}"));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

fn comparison_metric_labels(report: &MarketingV2ComparisonReport) -> Vec<String> {
    let mut deltas_by_metric = std::collections::BTreeMap::<String, Vec<i32>>::new();
    for scenario in &report.scenarios {
        for metric in &scenario.metric_deltas {
            deltas_by_metric
                .entry(metric.label.clone())
                .or_default()
                .push(metric.delta_vs_compare_average);
        }
    }

    let mut metrics = deltas_by_metric
        .into_iter()
        .map(|(label, deltas)| (label, metric_delta_swing(deltas.into_iter())))
        .collect::<Vec<_>>();
    metrics.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    metrics.into_iter().map(|(label, _)| label).collect()
}

fn metric_delta_swing(deltas: impl IntoIterator<Item = i32>) -> i32 {
    let mut min_delta: Option<i32> = None;
    let mut max_delta: Option<i32> = None;

    for delta in deltas {
        min_delta = Some(min_delta.map_or(delta, |current| current.min(delta)));
        max_delta = Some(max_delta.map_or(delta, |current| current.max(delta)));
    }

    match (min_delta, max_delta) {
        (Some(min_delta), Some(max_delta)) => max_delta - min_delta,
        _ => 0,
    }
}

fn build_persona_rows(result: &MarketingSimulationResultV2) -> Vec<(String, String, u32, f64)> {
    let mut rows = result
        .approach_results
        .iter()
        .flat_map(|approach| {
            approach.persona_results.iter().map(|persona| {
                (
                    persona.persona_name.clone(),
                    approach.approach_id.clone(),
                    persona.primary_scorecard.overall_score,
                    persona.audience_weight,
                )
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| left.0.cmp(&right.0)));
    rows.truncate(10);
    rows
}

fn markdown_signed_delta(delta: i32) -> String {
    if delta > 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    }
}

fn build_repeated_concerns(
    result: &MarketingSimulationResultV2,
) -> Vec<(String, usize, Vec<String>)> {
    let mut buckets = std::collections::BTreeMap::<String, (usize, Vec<String>)>::new();
    for approach in &result.approach_results {
        for concern in &approach.concerns {
            let entry = buckets.entry(concern.clone()).or_insert((0, Vec::new()));
            entry.0 += 1;
            if !entry.1.contains(&approach.approach_id) {
                entry.1.push(approach.approach_id.clone());
            }
        }
    }
    let mut rows = buckets
        .into_iter()
        .map(|(concern, (count, approaches))| (concern, count, approaches))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    rows
}

fn format_delta(delta: &composure_core::SummaryDelta) -> String {
    format!(
        "baseline={:?}, candidate={:?}, delta={:?}",
        delta.baseline, delta.candidate, delta.delta
    )
}

fn collect_parameter_names(
    parameter_sets: &[&std::collections::BTreeMap<String, ParameterValue>],
) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    for parameters in parameter_sets {
        names.extend(parameters.keys().cloned());
    }
    names.into_iter().collect()
}

fn metadata_string(metadata: Option<&serde_json::Value>, key: &str) -> String {
    metadata
        .and_then(|value| value.get(key))
        .map(json_cell)
        .unwrap_or_default()
}

fn parameter_string(value: Option<&ParameterValue>) -> String {
    match value {
        Some(ParameterValue::Bool(value)) => value.to_string(),
        Some(ParameterValue::Int(value)) => value.to_string(),
        Some(ParameterValue::Float(value)) => value.clone(),
        Some(ParameterValue::Text(value)) => value.clone(),
        None => String::new(),
    }
}

fn format_parameter_map(parameters: &std::collections::BTreeMap<String, ParameterValue>) -> String {
    parameters
        .iter()
        .map(|(name, value)| format!("{name}={}", parameter_string(Some(value))))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_monte_carlo_config(config: &composure_core::MonteCarloConfig) -> String {
    format!(
        "{} paths / {} steps / seed {}",
        config.num_paths, config.time_steps, config.seed_base
    )
}

fn format_sensitivity_kind(kind: &SensitivityKind) -> String {
    match kind {
        SensitivityKind::Numeric(stats) => {
            format!(
                "numeric corr={:.4}, slope={:.4}",
                stats.correlation, stats.slope
            )
        }
        SensitivityKind::Categorical(stats) => {
            format!(
                "categorical range={:.4}, buckets={}",
                stats.range,
                stats.buckets.len()
            )
        }
    }
}

fn bool_cell(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn csv_row(fields: &[String]) -> String {
    fields
        .iter()
        .map(|field| csv_escape(field))
        .collect::<Vec<_>>()
        .join(",")
}

fn csv_escape(field: &str) -> String {
    if field.contains([',', '"', '\n']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn markdown_row(fields: &[String]) -> String {
    format!("| {} |", fields.join(" | "))
}

fn markdown_separator(columns: usize) -> String {
    format!("|{}|", vec![" --- "; columns].join("|"))
}

fn json_cell(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn option_to_string<T>(value: Option<T>) -> String
where
    T: ToString,
{
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn markdown_f64(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "n/a".into())
}

fn markdown_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "n/a".into())
}

fn markdown_isize(value: Option<isize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "n/a".into())
}

fn markdown_debug<T>(value: Option<T>) -> String
where
    T: std::fmt::Debug,
{
    value
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|| "n/a".into())
}

fn format_llm_usage(usage: &composure_marketing::LlmUsage) -> String {
    let mut parts = Vec::new();
    if let Some(input) = usage.input_tokens {
        parts.push(format!("input `{input}`"));
    }
    if let Some(output) = usage.output_tokens {
        parts.push(format!("output `{output}`"));
    }
    if let Some(reasoning) = usage.reasoning_tokens {
        parts.push(format!("reasoning `{reasoning}`"));
    }
    if let Some(total) = usage.total_tokens {
        parts.push(format!("total `{total}`"));
    }
    if parts.is_empty() {
        "n/a".into()
    } else {
        parts.join(", ")
    }
}

fn truncate_for_markdown(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    let truncated = text.chars().take(max_chars).collect::<String>();
    format!(
        "{truncated}\n\n...[truncated {} chars]",
        char_count - max_chars
    )
}

fn push_code_block(lines: &mut Vec<String>, language: &str, body: &str) {
    lines.push(format!("```{language}"));
    lines.push(body.to_string());
    lines.push("```".into());
}

pub(crate) fn render_market_report_markdown(result: &MarketSimulationResult) -> String {
    let mt = &result.market_totals;

    let mut lines = vec![
        "# Market Simulation Report".into(),
        String::new(),
        format!("Config digest: `{}`", result.config_digest),
        format!("Variants simulated: {}", result.variant_count),
        format!("Time steps: {}", result.time_steps),
        format!("Buyers sampled: {}", result.buyers.len()),
        format!("Cohorts: {}", result.cohorts.len()),
        String::new(),
        "## Market Totals".into(),
        format!("| Metric | Value |"),
        format!("| --- | --- |"),
        format!("| Total buyers | {} |", mt.total_buyers),
        format!("| Total signups | {} |", mt.total_signups),
        format!("| Total activations | {} |", mt.total_activations),
        format!("| Total churns | {} |", mt.total_churns),
        format!("| Total referrals | {} |", mt.total_referrals),
        format!("| Total revenue (cents) | {:.2} |", mt.total_revenue_cents),
        format!("| Market CTR | {:.4} |", mt.market_ctr),
        format!("| Market CVR | {:.4} |", mt.market_cvr),
        format!("| Average LTV (cents) | {:.2} |", mt.market_ltv),
        String::new(),
    ];

    if !result.cohorts.is_empty() {
        lines.push("## Cohort Breakdown".into());
        lines.push(String::new());
        lines.push("| Cohort | Archetype | Buyers | Signup% | Activation% | Churn% | Avg LTV |".into());
        lines.push("| --- | --- | --- | --- | --- | --- | --- |".into());

        for cohort in &result.cohorts {
            lines.push(format!(
                "| {} | {:?} | {} | {:.2} | {:.2} | {:.2} | {:.2} |",
                cohort.segment_key,
                cohort.archetype,
                cohort.buyer_count,
                cohort.signup_rate,
                cohort.activation_rate,
                cohort.churn_rate,
                cohort.avg_ltv_cents
            ));
        }
        lines.push(String::new());
    }

    if !result.buyers.is_empty() {
        lines.push("## Sampled Buyers (sample_rate controlled output)".into());
        lines.push(String::new());
        lines.push("| ID | Archetype | Signup | Activated | Churned | Referrals | LTV |".into());
        lines.push("| --- | --- | --- | --- | --- | --- | --- |".into());

        for buyer in result.buyers.iter().take(20) {
            lines.push(format!(
                "| {} | {:?} | {} | {} | {} | {} | {:.2} |",
                buyer.buyer_id,
                buyer.archetype,
                buyer.reached_signup,
                buyer.reached_activation,
                buyer.churned,
                buyer.referral_count,
                buyer.lifetime_value_cents
            ));
        }

        if result.buyers.len() > 20 {
            lines.push(String::new());
            lines.push(format!("... ({} more buyers)", result.buyers.len() - 20));
        }
    }

    lines.join("\n")
}
