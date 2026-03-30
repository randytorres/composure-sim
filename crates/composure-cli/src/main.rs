mod render;

use std::{env, fs};

use composure_calibration::CalibrationResult;
use composure_core::{
    build_deterministic_report, compare_monte_carlo_results, summarize_run, ComparisonConfig,
    DeterministicReport, ExperimentBundle, ExperimentExecutionConfig, MonteCarloResult, RunSummary,
    SweepExecutionResult, TrajectoryComparison,
};
use composure_runtime::{default_run_id, load_pack, load_pack_for_run, run_pack};
use render::{
    format_bundle, format_calibration, format_comparison, format_report, format_summary,
    format_sweep, render_bundle_markdown, render_calibration_csv, render_calibration_markdown,
    render_report_markdown, render_sweep_csv, render_sweep_markdown, render_sweep_summary_markdown,
};
use thiserror::Error;

fn main() {
    if let Err(err) = real_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), CliError> {
    let args: Vec<String> = env::args().collect();
    let output = run(&args)?;
    println!("{output}");
    Ok(())
}

fn run(args: &[String]) -> Result<String, CliError> {
    match args {
        [_bin] => Err(CliError::Usage(usage())),
        [_bin, command] if command == "help" || command == "--help" || command == "-h" => {
            Ok(usage())
        }
        [_bin, command, path] if command == "inspect-pack" => {
            let pack = load_pack(path).map_err(CliError::Pack)?;
            Ok(pack.summary())
        }
        [_bin, command, path] if command == "validate-pack" => {
            let pack = load_pack(path).map_err(CliError::Pack)?;
            Ok(format!(
                "Pack valid: {} ({})",
                pack.definition.name, pack.definition.id
            ))
        }
        [_bin, command, path, tail @ ..] if command == "run-pack" => {
            let pack = load_pack_for_run(path).map_err(CliError::Pack)?;
            let bundle = run_pack(
                &pack,
                default_run_id(&pack),
                &ExperimentExecutionConfig::default(),
            )
            .map_err(CliError::PackRun)?;
            let output = serde_json::to_string_pretty(&bundle).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-bundle" => {
            let bundle = read_json::<ExperimentBundle>(path)?;
            Ok(format_bundle(&bundle))
        }
        [_bin, command, path, tail @ ..] if command == "export-bundle-markdown" => {
            let bundle = read_json::<ExperimentBundle>(path)?;
            let output = render_bundle_markdown(&bundle);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-sweep" => {
            let result = read_json::<SweepExecutionResult>(path)?;
            Ok(format_sweep(&result))
        }
        [_bin, command, path, tail @ ..] if command == "export-sweep-summary-markdown" => {
            let result = read_json::<SweepExecutionResult>(path)?;
            let output = render_sweep_summary_markdown(&result);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-summary" => {
            let summary = read_json::<RunSummary>(path)?;
            Ok(format_summary(&summary))
        }
        [_bin, command, path] if command == "inspect-report" => {
            let report = read_json::<DeterministicReport>(path)?;
            Ok(format_report(&report))
        }
        [_bin, command, path, tail @ ..] if command == "export-report-markdown" => {
            let report = read_json::<DeterministicReport>(path)?;
            let output = render_report_markdown(&report);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "summarize-monte-carlo" => {
            let result = read_json::<MonteCarloResult>(path)?;
            let summary = summarize_run(Some(&result), None);
            let output = serde_json::to_string_pretty(&summary).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-calibration" => {
            let calibration = read_json::<CalibrationResult>(path)?;
            Ok(format_calibration(&calibration))
        }
        [_bin, command, path, tail @ ..]
            if command == "export-calibration-candidates"
                || command == "export-calibration-csv" =>
        {
            let calibration = read_json::<CalibrationResult>(path)?;
            let output = render_calibration_csv(&calibration);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "export-calibration-candidates-markdown" => {
            let calibration = read_json::<CalibrationResult>(path)?;
            let output = render_calibration_markdown(&calibration);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path] if command == "inspect-compare" => {
            let comparison = read_json::<TrajectoryComparison>(path)?;
            Ok(format_comparison(&comparison))
        }
        [_bin, command, path, tail @ ..]
            if command == "export-sweep-samples" || command == "export-sweep-csv" =>
        {
            let result = read_json::<SweepExecutionResult>(path)?;
            let output = render_sweep_csv(&result);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, path, tail @ ..] if command == "export-sweep-samples-markdown" => {
            let result = read_json::<SweepExecutionResult>(path)?;
            let output = render_sweep_markdown(&result);
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, bundle_path, run_id, tail @ ..] if command == "summarize-bundle-run" => {
            let bundle = read_json::<ExperimentBundle>(bundle_path)?;
            let run = bundle
                .runs
                .iter()
                .find(|run| run.run_id == *run_id)
                .ok_or_else(|| CliError::RunNotFound {
                    bundle_path: bundle_path.clone(),
                    run_id: run_id.clone(),
                })?;
            let outcome = run
                .outcome
                .as_ref()
                .ok_or_else(|| CliError::MissingRunOutcome {
                    bundle_path: bundle_path.clone(),
                    run_id: run_id.clone(),
                })?;
            let summary = summarize_run(outcome.monte_carlo.as_ref(), outcome.composure.as_ref());
            let output = serde_json::to_string_pretty(&summary).map_err(CliError::SerializeJson)?;
            write_output(output, parse_output_flag(tail)?)
        }
        [_bin, command, baseline_path, candidate_path, tail @ ..]
            if command == "compare-monte-carlo" =>
        {
            let baseline = read_json::<MonteCarloResult>(baseline_path)?;
            let candidate = read_json::<MonteCarloResult>(candidate_path)?;
            let options = parse_compare_options(tail)?;
            let comparison = compare_monte_carlo_results(&baseline, &candidate, &options.config)
                .map_err(CliError::Compare)?;
            let output =
                serde_json::to_string_pretty(&comparison).map_err(CliError::SerializeJson)?;
            write_output(output, options.output_path.as_deref())
        }
        [_bin, command, baseline_path, candidate_path, tail @ ..] if command == "build-report" => {
            let baseline = read_json::<RunSummary>(baseline_path)?;
            let candidate = read_json::<RunSummary>(candidate_path)?;
            let options = parse_build_report_options(tail)?;
            let comparison = match options.comparison_path.as_deref() {
                Some(path) => Some(read_json::<TrajectoryComparison>(path)?),
                None => None,
            };
            let report = build_deterministic_report(&baseline, &candidate, comparison.as_ref());
            let output = serde_json::to_string_pretty(&report).map_err(CliError::SerializeJson)?;
            write_output(output, options.output_path.as_deref())
        }
        [_bin, ..] => Err(CliError::UnknownCommand { usage: usage() }),
        [] => Err(CliError::Usage(usage())),
    }
}

fn usage() -> String {
    [
        "Usage:",
        "  composure inspect-pack <path>",
        "  composure validate-pack <path>",
        "  composure run-pack <path> [--output <path>]",
        "  composure inspect-bundle <path>",
        "  composure export-bundle-markdown <path> [--output <path>]",
        "  composure inspect-sweep <path>",
        "  composure export-sweep-summary-markdown <path> [--output <path>]",
        "  composure inspect-summary <path>",
        "  composure inspect-report <path>",
        "  composure export-report-markdown <path> [--output <path>]",
        "  composure summarize-monte-carlo <path> [--output <path>]",
        "  composure summarize-bundle-run <bundle-path> <run-id> [--output <path>]",
        "  composure inspect-calibration <path>",
        "  composure export-calibration-candidates <path> [--output <path>]",
        "  composure export-calibration-candidates-markdown <path> [--output <path>]",
        "  composure inspect-compare <path>",
        "  composure export-sweep-samples <path> [--output <path>]",
        "  composure export-sweep-samples-markdown <path> [--output <path>]",
        "  composure compare-monte-carlo <baseline-path> <candidate-path> [flags] [--output <path>]",
        "  composure build-report <baseline-summary-path> <candidate-summary-path> [--comparison <path>] [--output <path>]",
        "",
        "Commands:",
        "  inspect-pack   Read a pack directory or manifest and print a compiled summary",
        "  validate-pack  Validate a pack directory or manifest and its referenced artifacts",
        "  run-pack  Execute a pack with its built-in runtime model and emit an ExperimentBundle artifact",
        "  inspect-bundle   Read an ExperimentBundle JSON artifact and print a summary",
        "  export-bundle-markdown  Convert an ExperimentBundle JSON artifact into markdown",
        "  inspect-sweep    Read a SweepExecutionResult JSON artifact and print a summary",
        "  export-sweep-summary-markdown  Convert a SweepExecutionResult JSON artifact into markdown summary",
        "  inspect-summary  Read a RunSummary JSON artifact and print a summary",
        "  inspect-report   Read a DeterministicReport JSON artifact and print a summary",
        "  export-report-markdown  Convert a DeterministicReport JSON artifact into markdown",
        "  summarize-monte-carlo  Convert a MonteCarloResult JSON artifact into a RunSummary JSON artifact",
        "  summarize-bundle-run   Extract and summarize one run from an ExperimentBundle JSON artifact",
        "  inspect-calibration  Read a CalibrationResult JSON artifact and print a summary",
        "  export-calibration-candidates  Convert a CalibrationResult JSON artifact into CSV",
        "  export-calibration-candidates-markdown  Convert a CalibrationResult JSON artifact into markdown",
        "  inspect-compare  Read a TrajectoryComparison JSON artifact and print a summary",
        "  export-sweep-samples  Convert a SweepExecutionResult JSON artifact into CSV",
        "  export-sweep-samples-markdown  Convert a SweepExecutionResult JSON artifact into markdown",
        "  compare-monte-carlo  Compare two MonteCarloResult JSON artifacts and emit JSON",
        "  build-report   Build a DeterministicReport JSON artifact from two RunSummary artifacts",
        "",
        "Compare/build flags:",
        "  --divergence-threshold <float>",
        "  --sustained-steps <usize>",
        "  --equality-epsilon <float>",
        "  --failure-threshold <float>",
        "  --comparison <path>",
        "  --output <path>",
    ]
    .join("\n")
}

fn read_json<T>(path: &str) -> Result<T, CliError>
where
    T: serde::de::DeserializeOwned,
{
    let raw = fs::read_to_string(path).map_err(|source| CliError::ReadFile {
        path: path.into(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| CliError::ParseJson {
        path: path.into(),
        source,
    })
}

#[derive(Debug)]
struct CompareOptions {
    config: ComparisonConfig,
    output_path: Option<String>,
}

#[derive(Debug)]
struct BuildReportOptions {
    comparison_path: Option<String>,
    output_path: Option<String>,
}

fn parse_compare_options(args: &[String]) -> Result<CompareOptions, CliError> {
    let mut config = ComparisonConfig::default();
    let mut output_path = None;
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];
        let value = args
            .get(index + 1)
            .ok_or_else(|| CliError::MissingFlagValue(flag.clone()))?;

        match flag.as_str() {
            "--divergence-threshold" => {
                config.divergence_threshold = parse_flag(value, flag)?;
            }
            "--sustained-steps" => {
                config.sustained_steps = parse_flag(value, flag)?;
            }
            "--equality-epsilon" => {
                config.equality_epsilon = parse_flag(value, flag)?;
            }
            "--failure-threshold" => {
                config.failure_threshold = Some(parse_flag(value, flag)?);
            }
            "--output" => {
                output_path = Some(value.clone());
            }
            _ => return Err(CliError::UnknownFlag(flag.clone())),
        }

        index += 2;
    }

    config.validate().map_err(CliError::Compare)?;
    Ok(CompareOptions {
        config,
        output_path,
    })
}

fn parse_build_report_options(args: &[String]) -> Result<BuildReportOptions, CliError> {
    let mut comparison_path = None;
    let mut output_path = None;
    let mut index = 0;

    while index < args.len() {
        let flag = &args[index];
        let value = args
            .get(index + 1)
            .ok_or_else(|| CliError::MissingFlagValue(flag.clone()))?;

        match flag.as_str() {
            "--comparison" => {
                comparison_path = Some(value.clone());
            }
            "--output" => {
                output_path = Some(value.clone());
            }
            _ => return Err(CliError::UnknownFlag(flag.clone())),
        }

        index += 2;
    }

    Ok(BuildReportOptions {
        comparison_path,
        output_path,
    })
}

fn parse_output_flag(args: &[String]) -> Result<Option<&str>, CliError> {
    match args {
        [] => Ok(None),
        [flag, path] if flag == "--output" => Ok(Some(path.as_str())),
        [flag] => Err(CliError::MissingFlagValue(flag.clone())),
        [flag, ..] => Err(CliError::UnknownFlag(flag.clone())),
    }
}

fn parse_flag<T>(value: &str, flag: &str) -> Result<T, CliError>
where
    T: std::str::FromStr,
{
    value.parse::<T>().map_err(|_| CliError::InvalidFlagValue {
        flag: flag.into(),
        value: value.into(),
    })
}

fn write_output(output: String, output_path: Option<&str>) -> Result<String, CliError> {
    match output_path {
        Some(path) => {
            fs::write(path, &output).map_err(|source| CliError::WriteFile {
                path: path.into(),
                source,
            })?;
            Ok(format!("Wrote artifact to {path}"))
        }
        None => Ok(output),
    }
}

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Usage(String),
    #[error("unknown command\n\n{usage}")]
    UnknownCommand { usage: String },
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse JSON from {path}: {source}")]
    ParseJson {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("run {run_id} was not found in bundle {bundle_path}")]
    RunNotFound { bundle_path: String, run_id: String },
    #[error("run {run_id} in bundle {bundle_path} does not have an outcome")]
    MissingRunOutcome { bundle_path: String, run_id: String },
    #[error("missing value for flag {0}")]
    MissingFlagValue(String),
    #[error("unknown flag {0}")]
    UnknownFlag(String),
    #[error("invalid value {value} for flag {flag}")]
    InvalidFlagValue { flag: String, value: String },
    #[error("comparison failed: {0}")]
    Compare(composure_core::CompareError),
    #[error("pack error: {0}")]
    Pack(composure_runtime::PackError),
    #[error("pack execution error: {0}")]
    PackRun(composure_runtime::PackRunError),
    #[error("failed to serialize JSON output: {0}")]
    SerializeJson(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        time::{SystemTime, UNIX_EPOCH},
    };

    use composure_calibration::{
        CalibrationCandidate, CalibrationCaseFailure, CalibrationConfig, CalibrationFailureMode,
        CalibrationObjective, CalibrationResult, ObservedTrajectory,
    };
    use composure_core::monte_carlo::PercentileBands;
    use composure_core::{
        ComposureSummary, ExperimentExecutionConfig, ExperimentOutcome, ExperimentParameterSet,
        ExperimentSpec, MonteCarloConfig, MonteCarloSummary, ParameterValue, Scenario,
        SensitivityConfig, SensitivityDirection, SensitivityKind, SweepCase, SweepDefinition,
        SweepExecutionResult, SweepFailureMode, SweepParameter, SweepRunnerConfig, SweepStrategy,
    };

    use super::*;

    fn sample_bundle() -> ExperimentBundle {
        let mut spec = ExperimentSpec::new(
            "exp-1",
            "Baseline",
            Scenario::new(
                "baseline",
                "Baseline",
                composure_core::SimState::zeros(1),
                5,
            ),
        );
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(10, 5, 42));
        let mut bundle = ExperimentBundle::new(spec);
        let mut parameter_set = ExperimentParameterSet::new(
            "variant-a",
            "Variant A",
            Scenario::new(
                "variant-a",
                "Variant A",
                composure_core::SimState::zeros(1),
                5,
            ),
        );
        parameter_set.monte_carlo = Some(MonteCarloConfig::with_seed(10, 5, 7));
        bundle.add_parameter_set(parameter_set).unwrap();
        bundle
            .record_run(
                composure_core::ExperimentRunRecord::running("run-1", Some("variant-a"), Some(7))
                    .mark_completed(composure_core::ExperimentOutcome {
                        monte_carlo: None,
                        composure: None,
                        replay: None,
                        metadata: None,
                    }),
            )
            .unwrap();
        bundle
    }

    fn sample_monte_carlo(seed: u64, values: [f64; 3]) -> MonteCarloResult {
        MonteCarloResult {
            paths: vec![],
            percentiles: PercentileBands {
                p10: vec![values[0] - 0.04, values[1] - 0.04, values[2] - 0.04],
                p25: vec![values[0] - 0.02, values[1] - 0.02, values[2] - 0.02],
                p50: vec![values[0], values[1], values[2]],
                p75: vec![values[0] + 0.02, values[1] + 0.02, values[2] + 0.02],
                p90: vec![values[0] + 0.04, values[1] + 0.04, values[2] + 0.04],
            },
            mean_trajectory: values.into(),
            config: MonteCarloConfig::with_seed(10, 3, seed),
        }
    }

    fn sample_baseline_summary() -> RunSummary {
        RunSummary {
            monte_carlo: Some(MonteCarloSummary {
                time_steps: 3,
                num_paths: 10,
                start: Some(0.84),
                end: Some(0.64),
                min: Some(0.64),
                max: Some(0.84),
                mean: Some(0.74),
                auc: Some(1.48),
                p10_end: Some(0.6),
                p50_end: Some(0.64),
                p90_end: Some(0.68),
                final_band_width: Some(0.08),
            }),
            composure: Some(ComposureSummary {
                archetype: composure_core::Archetype::CliffFaller,
                slope: -0.1,
                variance: 0.01,
                peak: 0.84,
                trough: 0.64,
                recovery_half_life: Some(2),
                residual_damage: 0.2,
                break_point: Some(1),
            }),
        }
    }

    fn sample_candidate_summary() -> RunSummary {
        RunSummary {
            monte_carlo: Some(MonteCarloSummary {
                time_steps: 3,
                num_paths: 10,
                start: Some(0.86),
                end: Some(0.69),
                min: Some(0.69),
                max: Some(0.86),
                mean: Some(0.7733333333333333),
                auc: Some(1.545),
                p10_end: Some(0.65),
                p50_end: Some(0.69),
                p90_end: Some(0.73),
                final_band_width: Some(0.08),
            }),
            composure: Some(ComposureSummary {
                archetype: composure_core::Archetype::Phoenix,
                slope: -0.085,
                variance: 0.004822222222222221,
                peak: 0.86,
                trough: 0.69,
                recovery_half_life: Some(1),
                residual_damage: 0.17,
                break_point: Some(2),
            }),
        }
    }

    fn sample_comparison() -> TrajectoryComparison {
        composure_core::compare_trajectories(
            &[0.84, 0.74, 0.64],
            &[0.86, 0.77, 0.69],
            &ComparisonConfig::default(),
        )
        .unwrap()
    }

    fn sample_report() -> DeterministicReport {
        let baseline = sample_baseline_summary();
        let candidate = sample_candidate_summary();
        let comparison = sample_comparison();
        build_deterministic_report(&baseline, &candidate, Some(&comparison))
    }

    fn sample_calibration_result() -> CalibrationResult {
        let comparison = sample_comparison();
        let report = build_deterministic_report(
            &sample_baseline_summary(),
            &sample_candidate_summary(),
            Some(&comparison),
        );
        let mut parameters = BTreeMap::new();
        parameters.insert("dose".into(), ParameterValue::Int(3));
        let case = SweepCase {
            case_id: "dose-sweep-1".into(),
            parameters: parameters.clone(),
        };

        let mut parameter_set = ExperimentParameterSet::new(
            "ps-dose-sweep-1",
            "Dose Sweep 1",
            Scenario::new(
                "scenario-dose-sweep-1",
                "Scenario Dose Sweep 1",
                composure_core::SimState::zeros(1),
                3,
            ),
        );
        parameter_set.monte_carlo = Some(MonteCarloConfig::with_seed(10, 3, 7));

        let run = composure_core::ExperimentRunRecord::running(
            "calibration-run-1",
            Some("ps-dose-sweep-1"),
            Some(7),
        )
        .mark_completed(ExperimentOutcome {
            monte_carlo: Some(sample_monte_carlo(7, [0.86, 0.77, 0.69])),
            composure: None,
            replay: None,
            metadata: None,
        });

        let mut failure_parameters = BTreeMap::new();
        failure_parameters.insert("dose".into(), ParameterValue::Int(2));

        CalibrationResult {
            definition: SweepDefinition {
                id: "dose-sweep".into(),
                name: "Dose Sweep".into(),
                parameters: vec![SweepParameter {
                    name: "dose".into(),
                    values: vec![
                        ParameterValue::Int(1),
                        ParameterValue::Int(2),
                        ParameterValue::Int(3),
                    ],
                }],
                strategy: SweepStrategy::Grid,
                sample_count: None,
                seed: None,
                metadata: None,
            },
            observed: ObservedTrajectory::new("obs-1", "Observed Recovery", vec![0.84, 0.74, 0.64]),
            observed_summary: sample_baseline_summary(),
            bundle: None,
            candidates: vec![CalibrationCandidate {
                case,
                parameter_set,
                run,
                summary: sample_candidate_summary(),
                comparison: comparison.clone(),
                report,
                score: comparison.metrics.rmse,
            }],
            failures: vec![CalibrationCaseFailure {
                case: SweepCase {
                    case_id: "dose-sweep-2".into(),
                    parameters: failure_parameters,
                },
                parameter_set_id: Some("ps-dose-sweep-2".into()),
                error: "dose rejected".into(),
            }],
            best_case_id: Some("dose-sweep-1".into()),
            best_parameter_set_id: Some("ps-dose-sweep-1".into()),
            best_score: Some(comparison.metrics.rmse),
            config: CalibrationConfig {
                objective: CalibrationObjective::Rmse,
                failure_mode: CalibrationFailureMode::Continue,
                ..CalibrationConfig::default()
            },
        }
    }

    fn sample_sweep_result() -> SweepExecutionResult {
        let mut parameters = BTreeMap::new();
        parameters.insert("dose".into(), ParameterValue::Int(2));

        SweepExecutionResult {
            definition: SweepDefinition {
                id: "sweep-1".into(),
                name: "Sweep".into(),
                parameters: vec![SweepParameter {
                    name: "dose".into(),
                    values: vec![
                        ParameterValue::Int(1),
                        ParameterValue::Int(2),
                        ParameterValue::Int(3),
                    ],
                }],
                strategy: composure_core::SweepStrategy::Random,
                sample_count: Some(6),
                seed: Some(7),
                metadata: None,
            },
            bundle: None,
            executed_cases: vec![composure_core::ExecutedSweepCase {
                case: SweepCase {
                    case_id: "sweep-1-1".into(),
                    parameters: parameters.clone(),
                },
                parameter_set: ExperimentParameterSet::new(
                    "variant-a",
                    "Variant A",
                    Scenario::new(
                        "variant-a",
                        "Variant A",
                        composure_core::SimState::zeros(1),
                        5,
                    ),
                ),
                run: composure_core::ExperimentRunRecord::running(
                    "run-1",
                    Some("variant-a"),
                    Some(7),
                )
                .mark_completed(composure_core::ExperimentOutcome {
                    monte_carlo: None,
                    composure: None,
                    replay: None,
                    metadata: None,
                }),
                summary: RunSummary {
                    monte_carlo: None,
                    composure: None,
                },
                sample: Some(composure_core::SweepSample {
                    case_id: "sweep-1-1".into(),
                    parameters: parameters.clone(),
                    objective: 0.69,
                    metadata: Some(serde_json::json!({
                        "run_id": "run-1",
                        "parameter_set_id": "variant-a"
                    })),
                }),
            }],
            failures: vec![],
            samples: vec![composure_core::SweepSample {
                case_id: "sweep-1-1".into(),
                parameters,
                objective: 0.69,
                metadata: Some(serde_json::json!({
                    "run_id": "run-1",
                    "parameter_set_id": "variant-a"
                })),
            }],
            sensitivity: Some(composure_core::SensitivityReport {
                sample_count: 3,
                objective: composure_core::ObjectiveSummary {
                    min: 0.69,
                    max: 0.69,
                    mean: 0.69,
                    best_case_id: "sweep-1-1".into(),
                    worst_case_id: "sweep-1-1".into(),
                },
                rankings: vec![composure_core::ParameterSensitivity {
                    parameter: "dose".into(),
                    score: 0.9,
                    direction: SensitivityDirection::Positive,
                    kind: SensitivityKind::Numeric(composure_core::NumericSensitivityStats {
                        correlation: 0.9,
                        slope: 0.5,
                    }),
                }],
                config: SensitivityConfig::default(),
            }),
            config: SweepRunnerConfig {
                run_id_prefix: "run".into(),
                execution: ExperimentExecutionConfig::default(),
                sensitivity: SensitivityConfig::default(),
                failure_mode: SweepFailureMode::Continue,
            },
        }
    }

    #[test]
    fn test_format_bundle() {
        let output = format_bundle(&sample_bundle());
        assert!(output.contains("Bundle: Baseline (exp-1)"));
        assert!(output.contains("Parameter sets: 1"));
        assert!(output.contains("completed=1"));
    }

    #[test]
    fn test_render_bundle_markdown() {
        let output = render_bundle_markdown(&sample_bundle());
        assert!(output.contains("# Experiment Bundle: Baseline (exp-1)"));
        assert!(output.contains("- Default Monte Carlo: `10 paths / 5 steps / seed 42`"));
        assert!(output.contains("## Parameter Sets"));
        assert!(output.contains("| variant-a | Variant A | 5 | 10 paths / 5 steps / seed 7 |"));
        assert!(output.contains("## Runs"));
        assert!(output.contains("| run-1 | Completed | variant-a | 7 | no | no |  |"));
    }

    #[test]
    fn test_format_sweep() {
        let output = format_sweep(&sample_sweep_result());

        assert!(output.contains("Strategy: Random"));
        assert!(output.contains("Top sensitivity: dose"));
    }

    #[test]
    fn test_render_sweep_summary_markdown() {
        let output = render_sweep_summary_markdown(&sample_sweep_result());
        assert!(output.contains("# Sweep Summary: Sweep (sweep-1)"));
        assert!(output.contains("| Best Case | Best Objective | Worst Case | Mean Objective |"));
        assert!(output.contains("| sweep-1-1 | 0.6900 | sweep-1-1 | 0.6900 |"));
        assert!(output.contains("| sweep-1-1 | 0.6900 | run-1 | variant-a | dose=2 |"));
        assert!(output.contains("| dose | Positive | 0.9000 | numeric corr=0.9000, slope=0.5000 |"));
    }

    #[test]
    fn test_format_summary() {
        let output = format_summary(&RunSummary {
            monte_carlo: Some(MonteCarloSummary {
                time_steps: 5,
                num_paths: 100,
                start: Some(0.8),
                end: Some(0.4),
                min: Some(0.4),
                max: Some(0.8),
                mean: Some(0.6),
                auc: Some(2.4),
                p10_end: Some(0.3),
                p50_end: Some(0.4),
                p90_end: Some(0.5),
                final_band_width: Some(0.2),
            }),
            composure: Some(ComposureSummary {
                archetype: composure_core::Archetype::Phoenix,
                slope: -0.2,
                variance: 0.1,
                peak: 0.9,
                trough: 0.3,
                recovery_half_life: Some(2),
                residual_damage: 0.1,
                break_point: Some(1),
            }),
        });

        assert!(output.contains("Monte Carlo: present"));
        assert!(output.contains("final mean: Some(0.4)"));
        assert!(output.contains("Composure: present"));
        assert!(output.contains("archetype: Phoenix"));
    }

    #[test]
    fn test_format_report() {
        let output = format_report(&sample_report());
        assert!(output.contains("Start delta:"));
        assert!(output.contains("Archetype change:"));
        assert!(output.contains("Phoenix"));
        assert!(output.contains("Comparison: rmse="));
    }

    #[test]
    fn test_render_report_markdown() {
        let output = render_report_markdown(&sample_report());
        assert!(output.contains("# Deterministic Report"));
        assert!(output.contains("| Start | 0.8400 | 0.8600 |"));
        assert!(output.contains("## Comparison Snapshot"));
    }

    #[test]
    fn test_format_calibration() {
        let output = format_calibration(&sample_calibration_result());
        assert!(output.contains("Calibration: Dose Sweep (dose-sweep)"));
        assert!(output.contains("Observed: Observed Recovery (obs-1)"));
        assert!(output.contains("Failures: 1"));
        assert!(output.contains("Top candidate: case=dose-sweep-1"));
    }

    #[test]
    fn test_render_sweep_csv() {
        let output = render_sweep_csv(&sample_sweep_result());
        assert!(output.contains("case_id,is_best,objective,run_id,parameter_set_id,dose"));
        assert!(output.contains("sweep-1-1,true,0.690000000000,run-1,variant-a,2"));
    }

    #[test]
    fn test_render_sweep_markdown() {
        let output = render_sweep_markdown(&sample_sweep_result());
        assert!(output.contains("# Sweep Samples: Sweep (sweep-1)"));
        assert!(output.contains("| Case | Best | Objective | Run | Parameter Set | dose |"));
        assert!(output.contains("| sweep-1-1 | yes | 0.6900 | run-1 | variant-a | 2 |"));
    }

    #[test]
    fn test_render_calibration_csv() {
        let output = render_calibration_csv(&sample_calibration_result());
        assert!(output.contains("rank,is_best,case_id,parameter_set_id,run_id,score"));
        assert!(output.contains("1,true,dose-sweep-1,ps-dose-sweep-1,calibration-run-1"));
        assert!(output.contains(",3"));
    }

    #[test]
    fn test_render_calibration_markdown() {
        let output = render_calibration_markdown(&sample_calibration_result());
        assert!(output.contains("# Calibration Candidates: Dose Sweep (dose-sweep)"));
        assert!(output.contains("| Rank | Best | Case | Parameter Set | Run | Score | RMSE | Mean Abs Delta | End Delta | dose |"));
        assert!(output.contains("| 1 | yes | dose-sweep-1 | ps-dose-sweep-1 | calibration-run-1 | 0.0356 | 0.0356 | 0.0333 | 0.0500 | 3 |"));
    }

    #[test]
    fn test_run_help() {
        let output = run(&["composure".into(), "help".into()]).unwrap();
        assert!(output.contains("inspect-pack"));
        assert!(output.contains("validate-pack"));
        assert!(output.contains("run-pack"));
        assert!(output.contains("inspect-report"));
        assert!(output.contains("inspect-calibration"));
        assert!(output.contains("export-bundle-markdown"));
        assert!(output.contains("export-report-markdown"));
        assert!(output.contains("export-sweep-summary-markdown"));
        assert!(output.contains("export-sweep-samples"));
        assert!(output.contains("export-sweep-samples-markdown"));
        assert!(output.contains("export-calibration-candidates"));
        assert!(output.contains("export-calibration-candidates-markdown"));
        assert!(output.contains("summarize-bundle-run"));
        assert!(output.contains("build-report"));
        assert!(output.contains("--output <path>"));
    }

    #[test]
    fn test_format_comparison() {
        let comparison = composure_core::compare_trajectories(
            &[0.9, 0.8, 0.7],
            &[0.9, 0.82, 0.6],
            &composure_core::ComparisonConfig {
                failure_threshold: Some(0.65),
                ..composure_core::ComparisonConfig::default()
            },
        )
        .unwrap();

        let output = format_comparison(&comparison);
        assert!(output.contains("Series length: 3"));
        assert!(output.contains("Failure comparison:"));
    }

    #[test]
    fn test_parse_compare_options_overrides_defaults() {
        let options = parse_compare_options(&[
            "--divergence-threshold".into(),
            "0.2".into(),
            "--sustained-steps".into(),
            "3".into(),
            "--equality-epsilon".into(),
            "0.01".into(),
            "--failure-threshold".into(),
            "0.4".into(),
        ])
        .unwrap();

        assert_eq!(options.config.divergence_threshold, 0.2);
        assert_eq!(options.config.sustained_steps, 3);
        assert_eq!(options.config.equality_epsilon, 0.01);
        assert_eq!(options.config.failure_threshold, Some(0.4));
        assert!(options.output_path.is_none());
    }

    #[test]
    fn test_parse_compare_options_accepts_output() {
        let options = parse_compare_options(&[
            "--divergence-threshold".into(),
            "0.2".into(),
            "--output".into(),
            "/tmp/comparison.json".into(),
        ])
        .unwrap();

        assert_eq!(options.config.divergence_threshold, 0.2);
        assert_eq!(options.output_path.as_deref(), Some("/tmp/comparison.json"));
    }

    #[test]
    fn test_parse_compare_options_rejects_unknown_flag() {
        let err = parse_compare_options(&["--unknown".into(), "1".into()]).unwrap_err();
        assert!(matches!(err, CliError::UnknownFlag(flag) if flag == "--unknown"));
    }

    #[test]
    fn test_parse_build_report_options_accepts_flags() {
        let options = parse_build_report_options(&[
            "--comparison".into(),
            "/tmp/comparison.json".into(),
            "--output".into(),
            "/tmp/report.json".into(),
        ])
        .unwrap();

        assert_eq!(
            options.comparison_path.as_deref(),
            Some("/tmp/comparison.json")
        );
        assert_eq!(options.output_path.as_deref(), Some("/tmp/report.json"));
    }

    #[test]
    fn test_parse_build_report_options_rejects_unknown_flag() {
        let err = parse_build_report_options(&["--unknown".into(), "1".into()]).unwrap_err();
        assert!(matches!(err, CliError::UnknownFlag(flag) if flag == "--unknown"));
    }

    #[test]
    fn test_run_compare_monte_carlo_outputs_json() {
        let temp_dir = std::env::temp_dir();
        let baseline_path = temp_dir.join("composure-cli-baseline.json");
        let candidate_path = temp_dir.join("composure-cli-candidate.json");

        let baseline = sample_monte_carlo(1, [0.84, 0.74, 0.64]);
        let candidate = sample_monte_carlo(2, [0.86, 0.77, 0.69]);

        fs::write(&baseline_path, serde_json::to_string(&baseline).unwrap()).unwrap();
        fs::write(&candidate_path, serde_json::to_string(&candidate).unwrap()).unwrap();

        let output = run(&[
            "composure".into(),
            "compare-monte-carlo".into(),
            baseline_path.display().to_string(),
            candidate_path.display().to_string(),
        ])
        .unwrap();

        let comparison: TrajectoryComparison = serde_json::from_str(&output).unwrap();
        assert_eq!(comparison.series_len, 3);
        assert!(comparison.metrics.mean_delta > 0.0);

        let _ = fs::remove_file(baseline_path);
        let _ = fs::remove_file(candidate_path);
    }

    #[test]
    fn test_run_compare_monte_carlo_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let baseline_path = temp_dir.join("composure-cli-baseline-output.json");
        let candidate_path = temp_dir.join("composure-cli-candidate-output.json");
        let output_path = temp_dir.join("composure-cli-comparison-output.json");

        fs::write(
            &baseline_path,
            serde_json::to_string(&sample_monte_carlo(1, [0.84, 0.74, 0.64])).unwrap(),
        )
        .unwrap();
        fs::write(
            &candidate_path,
            serde_json::to_string(&sample_monte_carlo(2, [0.86, 0.77, 0.69])).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "compare-monte-carlo".into(),
            baseline_path.display().to_string(),
            candidate_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let comparison: TrajectoryComparison = serde_json::from_str(&written).unwrap();
        assert_eq!(comparison.series_len, 3);

        let _ = fs::remove_file(baseline_path);
        let _ = fs::remove_file(candidate_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_build_report_outputs_json() {
        let temp_dir = std::env::temp_dir();
        let baseline_path = temp_dir.join("composure-cli-baseline-summary.json");
        let candidate_path = temp_dir.join("composure-cli-candidate-summary.json");
        let comparison_path = temp_dir.join("composure-cli-build-report-comparison.json");

        fs::write(
            &baseline_path,
            serde_json::to_string(&sample_baseline_summary()).unwrap(),
        )
        .unwrap();
        fs::write(
            &candidate_path,
            serde_json::to_string(&sample_candidate_summary()).unwrap(),
        )
        .unwrap();
        fs::write(
            &comparison_path,
            serde_json::to_string(&sample_comparison()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "build-report".into(),
            baseline_path.display().to_string(),
            candidate_path.display().to_string(),
            "--comparison".into(),
            comparison_path.display().to_string(),
        ])
        .unwrap();

        let report: DeterministicReport = serde_json::from_str(&output).unwrap();
        assert_eq!(report.end_delta.delta, Some(0.04999999999999993));
        assert!(report.comparison.is_some());

        let _ = fs::remove_file(baseline_path);
        let _ = fs::remove_file(candidate_path);
        let _ = fs::remove_file(comparison_path);
    }

    #[test]
    fn test_run_build_report_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let baseline_path = temp_dir.join("composure-cli-baseline-summary-output.json");
        let candidate_path = temp_dir.join("composure-cli-candidate-summary-output.json");
        let output_path = temp_dir.join("composure-cli-report-output.json");

        fs::write(
            &baseline_path,
            serde_json::to_string(&sample_baseline_summary()).unwrap(),
        )
        .unwrap();
        fs::write(
            &candidate_path,
            serde_json::to_string(&sample_candidate_summary()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "build-report".into(),
            baseline_path.display().to_string(),
            candidate_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let report: DeterministicReport = serde_json::from_str(&written).unwrap();
        assert!(report.comparison.is_none());

        let _ = fs::remove_file(baseline_path);
        let _ = fs::remove_file(candidate_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_report_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-report-markdown-input.json");
        let output_path = temp_dir.join("composure-cli-report-markdown-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_report()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-report-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Deterministic Report"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    fn temp_pack_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("composure-cli-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_sample_pack(dir: &std::path::Path) {
        let mut scenario = Scenario::new(
            "pack-scenario",
            "Pack Scenario",
            composure_core::SimState::zeros(2),
            4,
        );
        scenario.initial_state =
            composure_core::SimState::new(vec![0.4, 0.5], vec![0.1, 0.2], vec![0.2, 0.2]);
        scenario.failure_threshold = Some(0.45);
        scenario.metadata = Some(serde_json::json!({
            "dimension_labels": ["sleep", "readiness"]
        }));

        let mut spec = ExperimentSpec::new("pack-exp", "Pack Experiment", scenario.clone());
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(8, 4, 7));

        let observed = ObservedTrajectory::new("obs-1", "Observed", vec![0.4, 0.45, 0.5, 0.55]);

        let mut sweep = SweepDefinition::new("sweep-1", "Sweep");
        sweep.parameters.push(SweepParameter {
            name: "dose".into(),
            values: vec![ParameterValue::Int(1)],
        });

        fs::write(
            dir.join("scenario.json"),
            serde_json::to_string_pretty(&scenario).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("experiment-spec.json"),
            serde_json::to_string_pretty(&spec).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("sweep-definition.json"),
            serde_json::to_string_pretty(&sweep).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("observed-trajectory.json"),
            serde_json::to_string_pretty(&observed).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.join("pack.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "id": "health-pack",
                "name": "Health Pack",
                "scenario": "scenario.json",
                "experiment_spec": "experiment-spec.json",
                "sweep_definition": "sweep-definition.json",
                "observed_trajectory": "observed-trajectory.json",
                "runtime_model": {
                    "kind": "linear",
                    "dimensions": [
                        {
                            "drift": 0.01,
                            "action_gain": 0.08,
                            "memory_decay": 0.1,
                            "action_to_memory": 0.06,
                            "memory_to_state": 0.04,
                            "uncertainty_decay": 0.05,
                            "action_to_uncertainty": 0.2,
                            "min_value": 0.0,
                            "max_value": 1.0
                        },
                        {
                            "drift": 0.015,
                            "action_gain": 0.06,
                            "memory_decay": 0.08,
                            "action_to_memory": 0.05,
                            "memory_to_state": 0.03,
                            "uncertainty_decay": 0.05,
                            "action_to_uncertainty": 0.2,
                            "min_value": 0.0,
                            "max_value": 1.0
                        }
                    ],
                    "action_type_scales": {
                        "intervention": 1.0,
                        "stressor_onset": 1.0,
                        "stressor_removal": 0.8,
                        "hold": 0.0,
                        "custom": {}
                    },
                    "noise_scale": 0.01,
                    "aggregate_weights": [0.7, 0.3]
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_run_inspect_pack_outputs_summary() {
        let dir = temp_pack_dir("inspect-pack");
        write_sample_pack(&dir);

        let output = run(&[
            "composure".into(),
            "inspect-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Pack: Health Pack (health-pack)"));
        assert!(output.contains("Scenario: Pack Scenario (pack-scenario)"));
        assert!(output.contains("Experiment spec: yes"));
    }

    #[test]
    fn test_run_validate_pack_outputs_success() {
        let dir = temp_pack_dir("validate-pack");
        write_sample_pack(&dir);

        let output = run(&[
            "composure".into(),
            "validate-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        assert_eq!(output, "Pack valid: Health Pack (health-pack)");
    }

    #[test]
    fn test_run_run_pack_outputs_bundle_json() {
        let dir = temp_pack_dir("run-pack");
        write_sample_pack(&dir);

        let output = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        let bundle: ExperimentBundle = serde_json::from_str(&output).unwrap();
        assert_eq!(bundle.spec.id, "pack-exp");
        assert_eq!(bundle.runs.len(), 1);
        assert_eq!(bundle.runs[0].run_id, "health-pack-run-1");
    }

    #[test]
    fn test_run_run_pack_accepts_manifest_path() {
        let dir = temp_pack_dir("run-pack-manifest");
        write_sample_pack(&dir);

        let output = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.join("pack.json").display().to_string(),
        ])
        .unwrap();

        let bundle: ExperimentBundle = serde_json::from_str(&output).unwrap();
        assert_eq!(bundle.runs.len(), 1);
    }

    #[test]
    fn test_run_run_pack_writes_output_file() {
        let dir = temp_pack_dir("run-pack-output");
        write_sample_pack(&dir);
        let output_path = dir.join("bundle-output.json");

        let output = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let bundle: ExperimentBundle = serde_json::from_str(&written).unwrap();
        assert_eq!(bundle.runs.len(), 1);
    }

    #[test]
    fn test_run_run_pack_ignores_invalid_observed_and_sweep_inputs() {
        let dir = temp_pack_dir("run-pack-runtime-only");
        write_sample_pack(&dir);
        fs::write(dir.join("observed-trajectory.json"), "{not-json").unwrap();
        fs::write(dir.join("sweep-definition.json"), "{not-json").unwrap();

        let output = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap();

        let bundle: ExperimentBundle = serde_json::from_str(&output).unwrap();
        assert_eq!(bundle.runs.len(), 1);
    }

    #[test]
    fn test_run_run_pack_requires_runtime_model() {
        let dir = temp_pack_dir("run-pack-missing-runtime");
        write_sample_pack(&dir);

        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("pack.json")).unwrap()).unwrap();
        manifest
            .as_object_mut()
            .unwrap()
            .remove("runtime_model")
            .unwrap();
        fs::write(
            dir.join("pack.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let err = run(&[
            "composure".into(),
            "run-pack".into(),
            dir.display().to_string(),
        ])
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("pack does not define a runtime_model"));
    }

    #[test]
    fn test_run_export_bundle_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-bundle-markdown-input.json");
        let output_path = temp_dir.join("composure-cli-bundle-markdown-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_bundle()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-bundle-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Experiment Bundle: Baseline (exp-1)"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_sweep_summary_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-sweep-summary-md-input.json");
        let output_path = temp_dir.join("composure-cli-sweep-summary-md-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_sweep_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-sweep-summary-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Sweep Summary: Sweep (sweep-1)"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_sweep_csv_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-sweep-csv-input.json");
        let output_path = temp_dir.join("composure-cli-sweep-csv-output.csv");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_sweep_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-sweep-samples".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("case_id,is_best,objective,run_id,parameter_set_id,dose"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_sweep_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-sweep-md-input.json");
        let output_path = temp_dir.join("composure-cli-sweep-md-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_sweep_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-sweep-samples-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Sweep Samples: Sweep (sweep-1)"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_calibration_csv_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-calibration-csv-input.json");
        let output_path = temp_dir.join("composure-cli-calibration-csv-output.csv");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_calibration_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-calibration-candidates".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("rank,is_best,case_id,parameter_set_id,run_id,score"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_export_calibration_markdown_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-calibration-md-input.json");
        let output_path = temp_dir.join("composure-cli-calibration-md-output.md");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_calibration_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "export-calibration-candidates-markdown".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        assert!(written.contains("# Calibration Candidates: Dose Sweep (dose-sweep)"));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_summarize_monte_carlo_outputs_summary_json() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("composure-cli-summary-monte-carlo.json");

        fs::write(
            &path,
            serde_json::to_string(&sample_monte_carlo(1, [0.84, 0.74, 0.64])).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "summarize-monte-carlo".into(),
            path.display().to_string(),
        ])
        .unwrap();

        let summary: RunSummary = serde_json::from_str(&output).unwrap();
        assert_eq!(summary.monte_carlo.as_ref().unwrap().time_steps, 3);
        assert!(summary.composure.is_none());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_run_summarize_monte_carlo_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("composure-cli-summary-input.json");
        let output_path = temp_dir.join("composure-cli-summary-output.json");

        fs::write(
            &input_path,
            serde_json::to_string(&sample_monte_carlo(1, [0.84, 0.74, 0.64])).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "summarize-monte-carlo".into(),
            input_path.display().to_string(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let summary: RunSummary = serde_json::from_str(&written).unwrap();
        assert_eq!(summary.monte_carlo.as_ref().unwrap().end, Some(0.64));

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_summarize_bundle_run_outputs_summary_json() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("composure-cli-bundle.json");
        let bundle = sample_bundle();

        fs::write(&path, serde_json::to_string(&bundle).unwrap()).unwrap();

        let output = run(&[
            "composure".into(),
            "summarize-bundle-run".into(),
            path.display().to_string(),
            "run-1".into(),
        ])
        .unwrap();

        let summary: RunSummary = serde_json::from_str(&output).unwrap();
        assert!(summary.monte_carlo.is_none());
        assert!(summary.composure.is_none());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_run_summarize_bundle_run_writes_output_file() {
        let temp_dir = std::env::temp_dir();
        let bundle_path = temp_dir.join("composure-cli-bundle-output.json");
        let output_path = temp_dir.join("composure-cli-bundle-summary-output.json");
        let bundle = sample_bundle();

        fs::write(&bundle_path, serde_json::to_string(&bundle).unwrap()).unwrap();

        let output = run(&[
            "composure".into(),
            "summarize-bundle-run".into(),
            bundle_path.display().to_string(),
            "run-1".into(),
            "--output".into(),
            output_path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Wrote artifact"));
        let written = fs::read_to_string(&output_path).unwrap();
        let summary: RunSummary = serde_json::from_str(&written).unwrap();
        assert!(summary.monte_carlo.is_none());

        let _ = fs::remove_file(bundle_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn test_run_inspect_report_outputs_summary() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("composure-cli-report-inspect.json");

        fs::write(&path, serde_json::to_string(&sample_report()).unwrap()).unwrap();

        let output = run(&[
            "composure".into(),
            "inspect-report".into(),
            path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Archetype change:"));
        assert!(output.contains("Comparison: rmse="));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_run_inspect_calibration_outputs_summary() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("composure-cli-calibration-inspect.json");

        fs::write(
            &path,
            serde_json::to_string(&sample_calibration_result()).unwrap(),
        )
        .unwrap();

        let output = run(&[
            "composure".into(),
            "inspect-calibration".into(),
            path.display().to_string(),
        ])
        .unwrap();

        assert!(output.contains("Calibration: Dose Sweep (dose-sweep)"));
        assert!(output.contains("Best case: Some(\"dose-sweep-1\")"));

        let _ = fs::remove_file(path);
    }
}
