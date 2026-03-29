use std::{env, fs};

use composure_core::{
    ExperimentBundle, ExperimentRunStatus, RunSummary, SensitivityKind, SweepExecutionResult,
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
        [_bin, command, path] if command == "inspect-bundle" => {
            let bundle = read_json::<ExperimentBundle>(path)?;
            Ok(format_bundle(&bundle))
        }
        [_bin, command, path] if command == "inspect-sweep" => {
            let result = read_json::<SweepExecutionResult>(path)?;
            Ok(format_sweep(&result))
        }
        [_bin, command, path] if command == "inspect-summary" => {
            let summary = read_json::<RunSummary>(path)?;
            Ok(format_summary(&summary))
        }
        [bin, ..] => Err(CliError::UnknownCommand {
            bin: bin.clone(),
            usage: usage(),
        }),
        [] => Err(CliError::Usage(usage())),
    }
}

fn usage() -> String {
    [
        "Usage:",
        "  composure inspect-bundle <path>",
        "  composure inspect-sweep <path>",
        "  composure inspect-summary <path>",
        "",
        "Commands:",
        "  inspect-bundle   Read an ExperimentBundle JSON artifact and print a summary",
        "  inspect-sweep    Read a SweepExecutionResult JSON artifact and print a summary",
        "  inspect-summary  Read a RunSummary JSON artifact and print a summary",
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

fn format_bundle(bundle: &ExperimentBundle) -> String {
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

fn format_sweep(result: &SweepExecutionResult) -> String {
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

fn format_summary(summary: &RunSummary) -> String {
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

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Usage(String),
    #[error("unknown command\n\n{usage}")]
    UnknownCommand { bin: String, usage: String },
    #[error("failed to read {path}: {source}")]
    ReadFile {
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
}

#[cfg(test)]
mod tests {
    use composure_core::{
        ComposureSummary, ExperimentExecutionConfig, ExperimentParameterSet, ExperimentSpec,
        MonteCarloConfig, MonteCarloSummary, Scenario, SensitivityConfig, SensitivityDirection,
        SweepCase, SweepDefinition, SweepExecutionResult, SweepFailureMode, SweepRunnerConfig,
    };
    use std::collections::BTreeMap;

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

    #[test]
    fn test_format_bundle() {
        let output = format_bundle(&sample_bundle());
        assert!(output.contains("Bundle: Baseline (exp-1)"));
        assert!(output.contains("Parameter sets: 1"));
        assert!(output.contains("completed=1"));
    }

    #[test]
    fn test_format_sweep() {
        let output = format_sweep(&SweepExecutionResult {
            definition: SweepDefinition {
                id: "sweep-1".into(),
                name: "Sweep".into(),
                parameters: vec![],
                strategy: composure_core::SweepStrategy::Random,
                sample_count: Some(6),
                seed: Some(7),
                metadata: None,
            },
            bundle: None,
            executed_cases: vec![composure_core::ExecutedSweepCase {
                case: SweepCase {
                    case_id: "sweep-1-1".into(),
                    parameters: BTreeMap::new(),
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
                sample: None,
            }],
            failures: vec![],
            samples: vec![],
            sensitivity: Some(composure_core::SensitivityReport {
                sample_count: 3,
                objective: composure_core::ObjectiveSummary {
                    min: 0.1,
                    max: 0.8,
                    mean: 0.4,
                    best_case_id: "best".into(),
                    worst_case_id: "worst".into(),
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
        });

        assert!(output.contains("Strategy: Random"));
        assert!(output.contains("Top sensitivity: dose"));
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
    fn test_run_help() {
        let output = run(&["composure".into(), "help".into()]).unwrap();
        assert!(output.contains("inspect-bundle"));
    }
}
