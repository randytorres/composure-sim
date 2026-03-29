use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::{
    analyze_sensitivity, execute_parameter_set, generate_sweep_cases, summarize_run,
    ExperimentError, ExperimentExecutionConfig, ExperimentExecutionError, ExperimentParameterSet,
    ExperimentRunRecord, RunSummary, SensitivityConfig, SensitivityError, SensitivityReport,
    Simulator, SweepCase, SweepDefinition, SweepSample,
};

/// Configuration for executing a sweep and analyzing the resulting samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepRunnerConfig {
    pub run_id_prefix: String,
    pub execution: ExperimentExecutionConfig,
    pub sensitivity: SensitivityConfig,
}

impl Default for SweepRunnerConfig {
    fn default() -> Self {
        Self {
            run_id_prefix: "sweep-run".into(),
            execution: ExperimentExecutionConfig::default(),
            sensitivity: SensitivityConfig::default(),
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

/// Fully executed record for one generated sweep case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedSweepCase {
    pub case: SweepCase,
    pub parameter_set: ExperimentParameterSet,
    pub run: ExperimentRunRecord,
    pub summary: RunSummary,
    pub objective: f64,
}

/// Portable artifact bundle produced by the sweep runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepExecutionResult {
    pub definition: SweepDefinition,
    pub executed_cases: Vec<ExecutedSweepCase>,
    pub samples: Vec<SweepSample>,
    pub sensitivity: SensitivityReport,
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
    config.validate()?;

    let cases = generate_sweep_cases(definition).map_err(SweepRunnerError::InvalidSweep)?;
    let mut executed_cases = Vec::with_capacity(cases.len());
    let mut samples = Vec::with_capacity(cases.len());

    for (index, case) in cases.into_iter().enumerate() {
        let parameter_set =
            build_parameter_set(&case).map_err(|message| SweepRunnerError::BuildParameterSet {
                case_id: case.case_id.clone(),
                message,
            })?;

        parameter_set
            .validate()
            .map_err(|source| SweepRunnerError::InvalidParameterSet {
                case_id: case.case_id.clone(),
                source,
            })?;

        let run = execute_parameter_set(
            format!("{}-{}", config.run_id_prefix, index + 1),
            sim,
            &parameter_set,
            &config.execution,
        )
        .map_err(|source| SweepRunnerError::ExecuteCase {
            case_id: case.case_id.clone(),
            source,
        })?;

        let outcome = run
            .outcome
            .as_ref()
            .ok_or_else(|| SweepRunnerError::MissingOutcome {
                case_id: case.case_id.clone(),
            })?;
        let summary = summarize_run(outcome.monte_carlo.as_ref(), outcome.composure.as_ref());

        let objective =
            extract_objective(&case, &parameter_set, &run, &summary).map_err(|message| {
                SweepRunnerError::ObjectiveExtraction {
                    case_id: case.case_id.clone(),
                    message,
                }
            })?;
        if !objective.is_finite() {
            return Err(SweepRunnerError::NonFiniteObjective {
                case_id: case.case_id.clone(),
                objective,
            });
        }

        samples.push(SweepSample {
            case_id: case.case_id.clone(),
            parameters: case.parameters.clone(),
            objective,
            metadata: Some(json!({
                "run_id": run.run_id,
                "parameter_set_id": parameter_set.id,
            })),
        });

        executed_cases.push(ExecutedSweepCase {
            case,
            parameter_set,
            run,
            summary,
            objective,
        });
    }

    let sensitivity = analyze_sensitivity(&samples, &config.sensitivity)
        .map_err(SweepRunnerError::AnalyzeSensitivity)?;

    Ok(SweepExecutionResult {
        definition: definition.clone(),
        executed_cases,
        samples,
        sensitivity,
        config: config.clone(),
    })
}

#[derive(Debug, Error)]
pub enum SweepRunnerError {
    #[error("sweep run ID prefix cannot be empty")]
    EmptyRunIdPrefix,
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
    #[error("completed run for case {case_id} did not include an outcome")]
    MissingOutcome { case_id: String },
    #[error("failed to extract objective for case {case_id}: {message}")]
    ObjectiveExtraction { case_id: String, message: String },
    #[error("objective for case {case_id} must be finite, got {objective}")]
    NonFiniteObjective { case_id: String, objective: f64 },
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
        assert_eq!(result.executed_cases[0].run.run_id, "dose-run-1");
        assert_eq!(result.sensitivity.rankings[0].parameter, "dose");
        assert!(matches!(
            result.sensitivity.rankings[0].direction,
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
