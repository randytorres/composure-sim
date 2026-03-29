use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::{
    analyze_sensitivity, execute_parameter_set, generate_sweep_cases, summarize_run,
    ExperimentBundle, ExperimentError, ExperimentExecutionConfig, ExperimentExecutionError,
    ExperimentParameterSet, ExperimentRunRecord, ExperimentSpec, RunSummary, SensitivityConfig,
    SensitivityError, SensitivityReport, Simulator, SweepCase, SweepDefinition, SweepSample,
};

/// Configuration for executing a sweep and analyzing the resulting samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepRunnerConfig {
    pub run_id_prefix: String,
    pub execution: ExperimentExecutionConfig,
    pub sensitivity: SensitivityConfig,
    pub failure_mode: SweepFailureMode,
}

impl Default for SweepRunnerConfig {
    fn default() -> Self {
        Self {
            run_id_prefix: "sweep-run".into(),
            execution: ExperimentExecutionConfig::default(),
            sensitivity: SensitivityConfig::default(),
            failure_mode: SweepFailureMode::FailFast,
        }
    }
}

impl SweepRunnerConfig {
    pub fn validate(&self) -> Result<(), SweepRunnerError> {
        if self.run_id_prefix.trim().is_empty() {
            return Err(SweepRunnerError::EmptyRunIdPrefix);
        }
        self.sensitivity
            .validate()
            .map_err(SweepRunnerError::InvalidSensitivityConfig)?;
        Ok(())
    }
}

/// Controls whether sweep execution stops on first error or accumulates case failures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SweepFailureMode {
    FailFast,
    Continue,
}

/// Fully executed record for one generated sweep case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedSweepCase {
    pub case: SweepCase,
    pub parameter_set: ExperimentParameterSet,
    pub run: ExperimentRunRecord,
    pub summary: RunSummary,
    pub sample: Option<SweepSample>,
}

/// Recorded failure for a sweep case that could not be fully processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepCaseFailure {
    pub case: SweepCase,
    pub parameter_set_id: Option<String>,
    pub error: String,
}

/// Portable artifact bundle produced by the sweep runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepExecutionResult {
    pub definition: SweepDefinition,
    pub bundle: Option<ExperimentBundle>,
    pub executed_cases: Vec<ExecutedSweepCase>,
    pub failures: Vec<SweepCaseFailure>,
    pub samples: Vec<SweepSample>,
    pub sensitivity: Option<SensitivityReport>,
    pub config: SweepRunnerConfig,
}

/// Execute every case in a sweep definition and analyze parameter sensitivity.
///
/// The caller provides:
/// - `build_parameter_set`: how a generated [`SweepCase`] maps onto an
///   [`ExperimentParameterSet`] for the caller's domain
/// - `extract_objective`: how to derive a scalar objective from the completed run summary
pub fn execute_sweep<S, B, O>(
    sim: &S,
    definition: &SweepDefinition,
    config: &SweepRunnerConfig,
    mut build_parameter_set: B,
    mut extract_objective: O,
) -> Result<SweepExecutionResult, SweepRunnerError>
where
    S: Simulator,
    B: FnMut(&SweepCase) -> Result<ExperimentParameterSet, String>,
    O: FnMut(
        &SweepCase,
        &ExperimentParameterSet,
        &ExperimentRunRecord,
        &RunSummary,
    ) -> Result<f64, String>,
{
    execute_sweep_internal(
        sim,
        None,
        definition,
        config,
        |_, case| build_parameter_set(case),
        |case, parameter_set, run, summary| {
            extract_objective(case, parameter_set, run, summary).map(Some)
        },
    )
}

/// Execute a sweep against an experiment spec and persist successful cases into a bundle.
///
/// If a built parameter set omits its Monte Carlo config, the runner inherits
/// `spec.default_monte_carlo` before validation and execution.
pub fn execute_experiment_sweep<S, B, O>(
    sim: &S,
    spec: &ExperimentSpec,
    definition: &SweepDefinition,
    config: &SweepRunnerConfig,
    mut build_parameter_set: B,
    extract_objective: O,
) -> Result<SweepExecutionResult, SweepRunnerError>
where
    S: Simulator,
    B: FnMut(&ExperimentSpec, &SweepCase) -> Result<ExperimentParameterSet, String>,
    O: FnMut(
        &SweepCase,
        &ExperimentParameterSet,
        &ExperimentRunRecord,
        &RunSummary,
    ) -> Result<Option<f64>, String>,
{
    spec.validate()
        .map_err(SweepRunnerError::InvalidExperimentSpec)?;

    execute_sweep_internal(
        sim,
        Some(spec),
        definition,
        config,
        |spec, case| build_parameter_set(spec.expect("experiment spec is provided"), case),
        extract_objective,
    )
}

fn execute_sweep_internal<S, B, O>(
    sim: &S,
    spec: Option<&ExperimentSpec>,
    definition: &SweepDefinition,
    config: &SweepRunnerConfig,
    mut build_parameter_set: B,
    mut extract_objective: O,
) -> Result<SweepExecutionResult, SweepRunnerError>
where
    S: Simulator,
    B: FnMut(Option<&ExperimentSpec>, &SweepCase) -> Result<ExperimentParameterSet, String>,
    O: FnMut(
        &SweepCase,
        &ExperimentParameterSet,
        &ExperimentRunRecord,
        &RunSummary,
    ) -> Result<Option<f64>, String>,
{
    config.validate()?;

    let cases = generate_sweep_cases(definition).map_err(SweepRunnerError::InvalidSweep)?;
    let mut bundle = spec.cloned().map(ExperimentBundle::new);
    let mut executed_cases = Vec::with_capacity(cases.len());
    let mut failures = Vec::new();
    let mut samples = Vec::with_capacity(cases.len());

    for (index, case) in cases.into_iter().enumerate() {
        let mut parameter_set = match build_parameter_set(spec, &case) {
            Ok(parameter_set) => parameter_set,
            Err(message) => {
                let error = SweepRunnerError::BuildParameterSet {
                    case_id: case.case_id.clone(),
                    message,
                };
                if should_continue_after_failure(&config.failure_mode) {
                    failures.push(SweepCaseFailure {
                        case,
                        parameter_set_id: None,
                        error: error.to_string(),
                    });
                    continue;
                }
                return Err(error);
            }
        };

        inherit_defaults_from_spec(&mut parameter_set, spec);

        if let Err(error) = validate_parameter_set_for_case(&case, &parameter_set) {
            if should_continue_after_failure(&config.failure_mode) {
                failures.push(SweepCaseFailure {
                    case,
                    parameter_set_id: Some(parameter_set.id.clone()),
                    error: error.to_string(),
                });
                continue;
            }
            return Err(error);
        }

        if let Some(bundle) = bundle.as_mut() {
            if let Err(source) = bundle.add_parameter_set(parameter_set.clone()) {
                let error = SweepRunnerError::BundleParameterSet {
                    case_id: case.case_id.clone(),
                    source,
                };
                if should_continue_after_failure(&config.failure_mode) {
                    failures.push(SweepCaseFailure {
                        case,
                        parameter_set_id: Some(parameter_set.id.clone()),
                        error: error.to_string(),
                    });
                    continue;
                }
                return Err(error);
            }
        }

        let run = match execute_parameter_set(
            format!("{}-{}", config.run_id_prefix, index + 1),
            sim,
            &parameter_set,
            &config.execution,
        ) {
            Ok(run) => run,
            Err(source) => {
                let error = SweepRunnerError::ExecuteCase {
                    case_id: case.case_id.clone(),
                    source,
                };
                if should_continue_after_failure(&config.failure_mode) {
                    failures.push(SweepCaseFailure {
                        case,
                        parameter_set_id: Some(parameter_set.id.clone()),
                        error: error.to_string(),
                    });
                    continue;
                }
                return Err(error);
            }
        };

        if let Some(bundle) = bundle.as_mut() {
            if let Err(source) = bundle.record_run(run.clone()) {
                let error = SweepRunnerError::BundleRun {
                    case_id: case.case_id.clone(),
                    source,
                };
                if should_continue_after_failure(&config.failure_mode) {
                    failures.push(SweepCaseFailure {
                        case,
                        parameter_set_id: Some(parameter_set.id.clone()),
                        error: error.to_string(),
                    });
                    continue;
                }
                return Err(error);
            }
        }

        let outcome = match run.outcome.as_ref() {
            Some(outcome) => outcome,
            None => {
                let error = SweepRunnerError::MissingOutcome {
                    case_id: case.case_id.clone(),
                };
                if should_continue_after_failure(&config.failure_mode) {
                    failures.push(SweepCaseFailure {
                        case,
                        parameter_set_id: Some(parameter_set.id.clone()),
                        error: error.to_string(),
                    });
                    continue;
                }
                return Err(error);
            }
        };
        let summary = summarize_run(outcome.monte_carlo.as_ref(), outcome.composure.as_ref());

        let sample = match extract_objective(&case, &parameter_set, &run, &summary) {
            Ok(Some(objective)) => {
                if !objective.is_finite() {
                    let error = SweepRunnerError::NonFiniteObjective {
                        case_id: case.case_id.clone(),
                        objective,
                    };
                    if should_continue_after_failure(&config.failure_mode) {
                        failures.push(SweepCaseFailure {
                            case: case.clone(),
                            parameter_set_id: Some(parameter_set.id.clone()),
                            error: error.to_string(),
                        });
                        None
                    } else {
                        return Err(error);
                    }
                } else {
                    Some(SweepSample {
                        case_id: case.case_id.clone(),
                        parameters: case.parameters.clone(),
                        objective,
                        metadata: Some(json!({
                            "run_id": run.run_id,
                            "parameter_set_id": parameter_set.id,
                        })),
                    })
                }
            }
            Ok(None) => None,
            Err(message) => {
                let error = SweepRunnerError::ObjectiveExtraction {
                    case_id: case.case_id.clone(),
                    message,
                };
                if should_continue_after_failure(&config.failure_mode) {
                    failures.push(SweepCaseFailure {
                        case: case.clone(),
                        parameter_set_id: Some(parameter_set.id.clone()),
                        error: error.to_string(),
                    });
                    None
                } else {
                    return Err(error);
                }
            }
        };

        if let Some(sample) = sample.clone() {
            samples.push(sample);
        }

        executed_cases.push(ExecutedSweepCase {
            case,
            parameter_set,
            run,
            summary,
            sample,
        });
    }

    let sensitivity = if samples.is_empty() {
        None
    } else {
        Some(
            analyze_sensitivity(&samples, &config.sensitivity)
                .map_err(SweepRunnerError::AnalyzeSensitivity)?,
        )
    };

    Ok(SweepExecutionResult {
        definition: definition.clone(),
        bundle,
        executed_cases,
        failures,
        samples,
        sensitivity,
        config: config.clone(),
    })
}

fn inherit_defaults_from_spec(
    parameter_set: &mut ExperimentParameterSet,
    spec: Option<&ExperimentSpec>,
) {
    if parameter_set.monte_carlo.is_none() {
        if let Some(spec) = spec {
            parameter_set.monte_carlo = spec.default_monte_carlo.clone();
        }
    }
}

fn validate_parameter_set_for_case(
    case: &SweepCase,
    parameter_set: &ExperimentParameterSet,
) -> Result<(), SweepRunnerError> {
    parameter_set
        .validate()
        .map_err(|source| SweepRunnerError::InvalidParameterSet {
            case_id: case.case_id.clone(),
            source,
        })?;

    if let Some(monte_carlo) = parameter_set.monte_carlo.as_ref() {
        if parameter_set.scenario.time_steps != monte_carlo.time_steps {
            return Err(SweepRunnerError::InconsistentTimeSteps {
                case_id: case.case_id.clone(),
                scenario_time_steps: parameter_set.scenario.time_steps,
                monte_carlo_time_steps: monte_carlo.time_steps,
            });
        }
    }

    Ok(())
}

fn should_continue_after_failure(mode: &SweepFailureMode) -> bool {
    matches!(mode, SweepFailureMode::Continue)
}

#[derive(Debug, Error)]
pub enum SweepRunnerError {
    #[error("sweep run ID prefix cannot be empty")]
    EmptyRunIdPrefix,
    #[error("invalid experiment spec: {0}")]
    InvalidExperimentSpec(ExperimentError),
    #[error("invalid sensitivity configuration: {0}")]
    InvalidSensitivityConfig(SensitivityError),
    #[error("invalid sweep definition: {0}")]
    InvalidSweep(SensitivityError),
    #[error("failed to build parameter set for case {case_id}: {message}")]
    BuildParameterSet { case_id: String, message: String },
    #[error("invalid parameter set for case {case_id}: {source}")]
    InvalidParameterSet {
        case_id: String,
        source: ExperimentError,
    },
    #[error("failed to execute case {case_id}: {source}")]
    ExecuteCase {
        case_id: String,
        source: ExperimentExecutionError,
    },
    #[error(
        "scenario time_steps ({scenario_time_steps}) and Monte Carlo time_steps ({monte_carlo_time_steps}) differ for case {case_id}"
    )]
    InconsistentTimeSteps {
        case_id: String,
        scenario_time_steps: usize,
        monte_carlo_time_steps: usize,
    },
    #[error("completed run for case {case_id} did not include an outcome")]
    MissingOutcome { case_id: String },
    #[error("failed to extract objective for case {case_id}: {message}")]
    ObjectiveExtraction { case_id: String, message: String },
    #[error("objective for case {case_id} must be finite, got {objective}")]
    NonFiniteObjective { case_id: String, objective: f64 },
    #[error("failed to add parameter set to bundle for case {case_id}: {source}")]
    BundleParameterSet {
        case_id: String,
        source: ExperimentError,
    },
    #[error("failed to add run to bundle for case {case_id}: {source}")]
    BundleRun {
        case_id: String,
        source: ExperimentError,
    },
    #[error("sensitivity analysis failed: {0}")]
    AnalyzeSensitivity(SensitivityError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, ActionType, MonteCarloConfig, ParameterValue, Scenario, SimState};

    struct DriftSim;

    impl Simulator for DriftSim {
        fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
            use rand::Rng;

            let mut next = state.clone();
            next.t += 1;
            for i in 0..next.z.len() {
                let noise = (rng.gen::<f64>() - 0.5) * 0.01;
                next.z[i] = (next.z[i] + action.magnitude * 0.02 + noise).clamp(0.0, 1.0);
            }
            next
        }
    }

    fn sweep_definition() -> SweepDefinition {
        let mut definition = SweepDefinition::new("dose-sweep", "Dose Sweep");
        definition.parameters.push(crate::SweepParameter {
            name: "dose".into(),
            values: vec![
                ParameterValue::Int(1),
                ParameterValue::Int(2),
                ParameterValue::Int(3),
            ],
        });
        definition.parameters.push(crate::SweepParameter {
            name: "protocol".into(),
            values: vec![
                ParameterValue::Text("a".into()),
                ParameterValue::Text("b".into()),
            ],
        });
        definition
    }

    fn build_parameter_set(case: &SweepCase) -> Result<ExperimentParameterSet, String> {
        let dose = match case.parameters.get("dose") {
            Some(ParameterValue::Int(value)) => *value as f64,
            _ => return Err("dose must be an integer".into()),
        };

        let mut scenario = Scenario::new(
            format!("scenario-{}", case.case_id),
            format!("Scenario {}", case.case_id),
            SimState::zeros(1),
            4,
        );
        scenario.failure_threshold = Some(0.3);
        scenario.actions.push(Action {
            dimension: Some(0),
            magnitude: dose,
            action_type: ActionType::Intervention,
            metadata: None,
        });

        let mut parameter_set = ExperimentParameterSet::new(
            format!("ps-{}", case.case_id),
            format!("Parameter Set {}", case.case_id),
            scenario,
        );
        parameter_set.monte_carlo = Some(MonteCarloConfig::with_seed(32, 4, 7));
        Ok(parameter_set)
    }

    fn experiment_spec() -> ExperimentSpec {
        let mut spec = ExperimentSpec::new(
            "exp-dose",
            "Dose Sweep",
            Scenario::new("baseline", "Baseline", SimState::zeros(1), 4),
        );
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(32, 4, 11));
        spec
    }

    #[test]
    fn test_execute_sweep_runs_all_cases() {
        let result = execute_sweep(
            &DriftSim,
            &sweep_definition(),
            &SweepRunnerConfig {
                run_id_prefix: "dose-run".into(),
                ..SweepRunnerConfig::default()
            },
            build_parameter_set,
            |_, _, _, summary| {
                Ok(summary
                    .monte_carlo
                    .as_ref()
                    .and_then(|monte_carlo| monte_carlo.end)
                    .unwrap_or(0.0))
            },
        )
        .unwrap();

        assert_eq!(result.executed_cases.len(), 6);
        assert_eq!(result.samples.len(), 6);
        assert!(result.failures.is_empty());
        assert!(result.bundle.is_none());
        assert_eq!(result.executed_cases[0].run.run_id, "dose-run-1");
        assert_eq!(
            result.sensitivity.as_ref().unwrap().rankings[0].parameter,
            "dose"
        );
        assert!(matches!(
            result.sensitivity.as_ref().unwrap().rankings[0].direction,
            crate::SensitivityDirection::Positive
        ));
    }

    #[test]
    fn test_execute_sweep_reports_parameter_mapping_error() {
        let err = execute_sweep(
            &DriftSim,
            &sweep_definition(),
            &SweepRunnerConfig::default(),
            |_| Err("bad mapping".into()),
            |_, _, _, _| Ok(0.0),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            SweepRunnerError::BuildParameterSet { case_id, .. } if case_id == "dose-sweep-1"
        ));
    }

    #[test]
    fn test_execute_sweep_rejects_non_finite_objective() {
        let err = execute_sweep(
            &DriftSim,
            &sweep_definition(),
            &SweepRunnerConfig::default(),
            build_parameter_set,
            |_, _, _, _| Ok(f64::NAN),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            SweepRunnerError::NonFiniteObjective { case_id, .. } if case_id == "dose-sweep-1"
        ));
    }

    #[test]
    fn test_execute_experiment_sweep_inherits_default_monte_carlo_and_records_bundle() {
        let spec = experiment_spec();

        let result = execute_experiment_sweep(
            &DriftSim,
            &spec,
            &sweep_definition(),
            &SweepRunnerConfig::default(),
            |spec, case| {
                let mut parameter_set = build_parameter_set(case)?;
                parameter_set.monte_carlo = None;
                parameter_set.scenario = spec.scenario.clone();
                parameter_set.scenario.id = format!("scenario-{}", case.case_id);
                parameter_set.scenario.name = format!("Scenario {}", case.case_id);
                parameter_set.scenario.actions.push(Action {
                    dimension: Some(0),
                    magnitude: match case.parameters.get("dose") {
                        Some(ParameterValue::Int(value)) => *value as f64,
                        _ => return Err("dose must be an integer".into()),
                    },
                    action_type: ActionType::Intervention,
                    metadata: None,
                });
                Ok(parameter_set)
            },
            |_, _, _, summary| {
                Ok(summary
                    .monte_carlo
                    .as_ref()
                    .and_then(|monte_carlo| monte_carlo.end))
            },
        )
        .unwrap();

        assert_eq!(result.executed_cases.len(), 6);
        assert_eq!(result.bundle.as_ref().unwrap().parameter_sets.len(), 6);
        assert_eq!(result.bundle.as_ref().unwrap().runs.len(), 6);
        assert!(result.sensitivity.is_some());
        assert!(result
            .executed_cases
            .iter()
            .all(|case| case.parameter_set.monte_carlo.is_some()));
    }

    #[test]
    fn test_execute_experiment_sweep_continue_mode_collects_failures() {
        let result = execute_experiment_sweep(
            &DriftSim,
            &experiment_spec(),
            &sweep_definition(),
            &SweepRunnerConfig {
                failure_mode: SweepFailureMode::Continue,
                ..SweepRunnerConfig::default()
            },
            |spec, case| {
                let dose = match case.parameters.get("dose") {
                    Some(ParameterValue::Int(value)) => *value,
                    _ => return Err("dose must be an integer".into()),
                };

                if dose == 2 {
                    return Err("dose 2 intentionally rejected".into());
                }

                let mut parameter_set = build_parameter_set(case)?;
                parameter_set.monte_carlo = spec.default_monte_carlo.clone();
                Ok(parameter_set)
            },
            |case, _, _, summary| {
                let protocol = case.parameters.get("protocol");
                if matches!(protocol, Some(ParameterValue::Text(value)) if value == "b") {
                    return Err("protocol b intentionally unscored".into());
                }

                Ok(summary
                    .monte_carlo
                    .as_ref()
                    .and_then(|monte_carlo| monte_carlo.end))
            },
        )
        .unwrap();

        assert_eq!(result.failures.len(), 4);
        assert_eq!(result.executed_cases.len(), 4);
        assert_eq!(result.samples.len(), 2);
        assert_eq!(result.bundle.as_ref().unwrap().parameter_sets.len(), 4);
        assert_eq!(result.bundle.as_ref().unwrap().runs.len(), 4);
        assert!(result.sensitivity.is_some());
    }

    #[test]
    fn test_execute_experiment_sweep_rejects_inconsistent_time_steps() {
        let err = execute_experiment_sweep(
            &DriftSim,
            &experiment_spec(),
            &sweep_definition(),
            &SweepRunnerConfig::default(),
            |_, case| {
                let mut parameter_set = build_parameter_set(case)?;
                parameter_set
                    .monte_carlo
                    .replace(MonteCarloConfig::with_seed(32, 5, 7));
                Ok(parameter_set)
            },
            |_, _, _, summary| {
                Ok(summary
                    .monte_carlo
                    .as_ref()
                    .and_then(|monte_carlo| monte_carlo.end))
            },
        )
        .unwrap_err();

        assert!(matches!(
            err,
            SweepRunnerError::InconsistentTimeSteps { case_id, .. } if case_id == "dose-sweep-1"
        ));
    }

    #[test]
    fn test_sweep_runner_config_requires_run_id_prefix() {
        let err = SweepRunnerConfig {
            run_id_prefix: "   ".into(),
            ..SweepRunnerConfig::default()
        }
        .validate()
        .unwrap_err();

        assert!(matches!(err, SweepRunnerError::EmptyRunIdPrefix));
    }
}
