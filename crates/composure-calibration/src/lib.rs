use composure_core::{
    analyze_composure_checked, build_deterministic_report, compare_trajectories,
    execute_parameter_set, generate_sweep_cases, summarize_composure, summarize_run,
    ComparisonConfig, DeterministicReport, ExperimentBundle, ExperimentError,
    ExperimentExecutionConfig, ExperimentExecutionError, ExperimentParameterSet,
    ExperimentRunRecord, ExperimentSpec, MonteCarloSummary, RunSummary, Simulator, SweepCase,
    SweepDefinition,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Observed trajectory used as the fitting target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedTrajectory {
    pub id: String,
    pub name: String,
    pub values: Vec<f64>,
    pub failure_threshold: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

impl ObservedTrajectory {
    pub fn new(id: impl Into<String>, name: impl Into<String>, values: Vec<f64>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            values,
            failure_threshold: None,
            metadata: None,
        }
    }

    pub fn validate(&self) -> Result<(), CalibrationError> {
        if self.id.trim().is_empty() {
            return Err(CalibrationError::EmptyObservedId);
        }
        if self.name.trim().is_empty() {
            return Err(CalibrationError::EmptyObservedName);
        }
        if self.values.is_empty() {
            return Err(CalibrationError::EmptyObservedValues);
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(CalibrationError::NonFiniteObservedValue);
        }
        if let Some(threshold) = self.failure_threshold {
            if !threshold.is_finite() {
                return Err(CalibrationError::InvalidObservedThreshold(threshold));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalibrationObjective {
    Rmse,
    MeanAbsDelta,
    EndAbsDelta,
    CumulativeAbsDelta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CalibrationFailureMode {
    FailFast,
    Continue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationConfig {
    pub run_id_prefix: String,
    pub execution: ExperimentExecutionConfig,
    pub comparison: ComparisonConfig,
    pub objective: CalibrationObjective,
    pub failure_mode: CalibrationFailureMode,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            run_id_prefix: "calibration-run".into(),
            execution: ExperimentExecutionConfig::default(),
            comparison: ComparisonConfig::default(),
            objective: CalibrationObjective::Rmse,
            failure_mode: CalibrationFailureMode::FailFast,
        }
    }
}

impl CalibrationConfig {
    pub fn validate(&self) -> Result<(), CalibrationError> {
        if self.run_id_prefix.trim().is_empty() {
            return Err(CalibrationError::EmptyRunIdPrefix);
        }
        self.comparison
            .validate()
            .map_err(CalibrationError::InvalidComparisonConfig)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationCandidate {
    pub case: SweepCase,
    pub parameter_set: ExperimentParameterSet,
    pub run: ExperimentRunRecord,
    pub summary: RunSummary,
    pub comparison: composure_core::TrajectoryComparison,
    pub report: DeterministicReport,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationCaseFailure {
    pub case: SweepCase,
    pub parameter_set_id: Option<String>,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub definition: SweepDefinition,
    pub observed: ObservedTrajectory,
    pub observed_summary: RunSummary,
    pub bundle: Option<ExperimentBundle>,
    pub candidates: Vec<CalibrationCandidate>,
    pub failures: Vec<CalibrationCaseFailure>,
    pub best_case_id: Option<String>,
    pub best_parameter_set_id: Option<String>,
    pub best_score: Option<f64>,
    pub config: CalibrationConfig,
}

pub fn calibrate<S, B>(
    sim: &S,
    observed: &ObservedTrajectory,
    definition: &SweepDefinition,
    config: &CalibrationConfig,
    mut build_parameter_set: B,
) -> Result<CalibrationResult, CalibrationError>
where
    S: Simulator,
    B: FnMut(&SweepCase) -> Result<ExperimentParameterSet, String>,
{
    calibrate_internal(sim, None, observed, definition, config, |_, case| {
        build_parameter_set(case)
    })
}

pub fn calibrate_experiment<S, B>(
    sim: &S,
    spec: &ExperimentSpec,
    observed: &ObservedTrajectory,
    definition: &SweepDefinition,
    config: &CalibrationConfig,
    mut build_parameter_set: B,
) -> Result<CalibrationResult, CalibrationError>
where
    S: Simulator,
    B: FnMut(&ExperimentSpec, &SweepCase) -> Result<ExperimentParameterSet, String>,
{
    spec.validate()
        .map_err(CalibrationError::InvalidExperimentSpec)?;

    calibrate_internal(
        sim,
        Some(spec),
        observed,
        definition,
        config,
        |spec, case| build_parameter_set(spec.expect("experiment spec present"), case),
    )
}

fn calibrate_internal<S, B>(
    sim: &S,
    spec: Option<&ExperimentSpec>,
    observed: &ObservedTrajectory,
    definition: &SweepDefinition,
    config: &CalibrationConfig,
    mut build_parameter_set: B,
) -> Result<CalibrationResult, CalibrationError>
where
    S: Simulator,
    B: FnMut(Option<&ExperimentSpec>, &SweepCase) -> Result<ExperimentParameterSet, String>,
{
    observed.validate()?;
    config.validate()?;

    let observed_summary = summarize_observed(observed)?;
    let cases = generate_sweep_cases(definition).map_err(CalibrationError::InvalidSweep)?;
    let mut bundle = spec.cloned().map(ExperimentBundle::new);
    let mut candidates = Vec::with_capacity(cases.len());
    let mut failures = Vec::new();

    for (index, case) in cases.into_iter().enumerate() {
        let mut parameter_set = match build_parameter_set(spec, &case) {
            Ok(parameter_set) => parameter_set,
            Err(message) => {
                let error = CalibrationError::BuildParameterSet {
                    case_id: case.case_id.clone(),
                    message,
                };
                if should_continue(&config.failure_mode) {
                    failures.push(CalibrationCaseFailure {
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

        if let Err(error) = validate_parameter_set(&case, &parameter_set) {
            if should_continue(&config.failure_mode) {
                failures.push(CalibrationCaseFailure {
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
                let error = CalibrationError::BundleParameterSet {
                    case_id: case.case_id.clone(),
                    source,
                };
                if should_continue(&config.failure_mode) {
                    failures.push(CalibrationCaseFailure {
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
                let error = CalibrationError::ExecuteCase {
                    case_id: case.case_id.clone(),
                    source,
                };
                if should_continue(&config.failure_mode) {
                    failures.push(CalibrationCaseFailure {
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
                let error = CalibrationError::BundleRun {
                    case_id: case.case_id.clone(),
                    source,
                };
                if should_continue(&config.failure_mode) {
                    failures.push(CalibrationCaseFailure {
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
                let error = CalibrationError::MissingOutcome {
                    case_id: case.case_id.clone(),
                };
                if should_continue(&config.failure_mode) {
                    failures.push(CalibrationCaseFailure {
                        case,
                        parameter_set_id: Some(parameter_set.id.clone()),
                        error: error.to_string(),
                    });
                    continue;
                }
                return Err(error);
            }
        };

        let monte_carlo =
            outcome
                .monte_carlo
                .as_ref()
                .ok_or_else(|| CalibrationError::MissingMonteCarlo {
                    case_id: case.case_id.clone(),
                })?;
        let summary = summarize_run(outcome.monte_carlo.as_ref(), outcome.composure.as_ref());
        let comparison = compare_trajectories(
            &observed.values,
            &monte_carlo.mean_trajectory,
            &config.comparison,
        )
        .map_err(|source| CalibrationError::CompareCase {
            case_id: case.case_id.clone(),
            source,
        })?;
        let report = build_deterministic_report(&observed_summary, &summary, Some(&comparison));
        let score = score_candidate(&comparison, &config.objective);

        candidates.push(CalibrationCandidate {
            case,
            parameter_set,
            run,
            summary,
            comparison,
            report,
            score,
        });
    }

    candidates.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.case.case_id.cmp(&b.case.case_id))
    });

    let best_case_id = candidates
        .first()
        .map(|candidate| candidate.case.case_id.clone());
    let best_parameter_set_id = candidates
        .first()
        .map(|candidate| candidate.parameter_set.id.clone());
    let best_score = candidates.first().map(|candidate| candidate.score);

    Ok(CalibrationResult {
        definition: definition.clone(),
        observed: observed.clone(),
        observed_summary,
        bundle,
        candidates,
        failures,
        best_case_id,
        best_parameter_set_id,
        best_score,
        config: config.clone(),
    })
}

fn summarize_observed(observed: &ObservedTrajectory) -> Result<RunSummary, CalibrationError> {
    let monte_carlo = MonteCarloSummary {
        time_steps: observed.values.len(),
        num_paths: 1,
        start: observed.values.first().copied(),
        end: observed.values.last().copied(),
        min: observed
            .values
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)),
        max: observed
            .values
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)),
        mean: Some(observed.values.iter().sum::<f64>() / observed.values.len() as f64),
        auc: if observed.values.len() < 2 {
            observed.values.first().copied()
        } else {
            Some(
                observed
                    .values
                    .windows(2)
                    .map(|window| (window[0] + window[1]) * 0.5)
                    .sum::<f64>(),
            )
        },
        p10_end: None,
        p50_end: None,
        p90_end: None,
        final_band_width: None,
    };

    let composure = match observed.failure_threshold {
        Some(threshold) => Some(summarize_composure(
            &analyze_composure_checked(&observed.values, threshold)
                .map_err(CalibrationError::ObservedComposure)?,
        )),
        None => None,
    };

    Ok(RunSummary {
        monte_carlo: Some(monte_carlo),
        composure,
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

fn validate_parameter_set(
    case: &SweepCase,
    parameter_set: &ExperimentParameterSet,
) -> Result<(), CalibrationError> {
    parameter_set
        .validate()
        .map_err(|source| CalibrationError::InvalidParameterSet {
            case_id: case.case_id.clone(),
            source,
        })?;

    let monte_carlo = parameter_set.monte_carlo.as_ref().ok_or_else(|| {
        CalibrationError::MissingMonteCarloConfig {
            case_id: case.case_id.clone(),
        }
    })?;

    if parameter_set.scenario.time_steps != monte_carlo.time_steps {
        return Err(CalibrationError::InconsistentTimeSteps {
            case_id: case.case_id.clone(),
            scenario_time_steps: parameter_set.scenario.time_steps,
            monte_carlo_time_steps: monte_carlo.time_steps,
        });
    }

    Ok(())
}

fn score_candidate(
    comparison: &composure_core::TrajectoryComparison,
    objective: &CalibrationObjective,
) -> f64 {
    match objective {
        CalibrationObjective::Rmse => comparison.metrics.rmse,
        CalibrationObjective::MeanAbsDelta => comparison.metrics.mean_abs_delta,
        CalibrationObjective::EndAbsDelta => comparison.metrics.end_delta.abs(),
        CalibrationObjective::CumulativeAbsDelta => comparison
            .deltas
            .iter()
            .map(|point| point.abs_delta)
            .sum::<f64>(),
    }
}

fn should_continue(mode: &CalibrationFailureMode) -> bool {
    matches!(mode, CalibrationFailureMode::Continue)
}

#[derive(Debug, Error)]
pub enum CalibrationError {
    #[error("observed trajectory ID cannot be empty")]
    EmptyObservedId,
    #[error("observed trajectory name cannot be empty")]
    EmptyObservedName,
    #[error("observed trajectory must contain at least one value")]
    EmptyObservedValues,
    #[error("observed trajectory contains a non-finite value")]
    NonFiniteObservedValue,
    #[error("observed failure threshold must be finite, got {0}")]
    InvalidObservedThreshold(f64),
    #[error("calibration run ID prefix cannot be empty")]
    EmptyRunIdPrefix,
    #[error("invalid comparison configuration: {0}")]
    InvalidComparisonConfig(composure_core::CompareError),
    #[error("invalid experiment spec: {0}")]
    InvalidExperimentSpec(ExperimentError),
    #[error("invalid sweep definition: {0}")]
    InvalidSweep(composure_core::SensitivityError),
    #[error("failed to build parameter set for case {case_id}: {message}")]
    BuildParameterSet { case_id: String, message: String },
    #[error("invalid parameter set for case {case_id}: {source}")]
    InvalidParameterSet {
        case_id: String,
        source: ExperimentError,
    },
    #[error("calibration case {case_id} requires a Monte Carlo configuration")]
    MissingMonteCarloConfig { case_id: String },
    #[error(
        "scenario time_steps ({scenario_time_steps}) and Monte Carlo time_steps ({monte_carlo_time_steps}) differ for case {case_id}"
    )]
    InconsistentTimeSteps {
        case_id: String,
        scenario_time_steps: usize,
        monte_carlo_time_steps: usize,
    },
    #[error("failed to execute case {case_id}: {source}")]
    ExecuteCase {
        case_id: String,
        source: ExperimentExecutionError,
    },
    #[error("completed run for case {case_id} did not include an outcome")]
    MissingOutcome { case_id: String },
    #[error("completed run for case {case_id} did not include Monte Carlo output")]
    MissingMonteCarlo { case_id: String },
    #[error(
        "failed to compare candidate against observed trajectory for case {case_id}: {source}"
    )]
    CompareCase {
        case_id: String,
        source: composure_core::CompareError,
    },
    #[error("failed to derive composure metrics for the observed trajectory: {0}")]
    ObservedComposure(composure_core::ComposureError),
    #[error("failed to add parameter set to calibration bundle for case {case_id}: {source}")]
    BundleParameterSet {
        case_id: String,
        source: ExperimentError,
    },
    #[error("failed to add run to calibration bundle for case {case_id}: {source}")]
    BundleRun {
        case_id: String,
        source: ExperimentError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use composure_core::{
        Action, ActionType, MonteCarloConfig, ParameterValue, Scenario, SimState, SweepParameter,
        SweepStrategy,
    };

    struct DriftSim;

    impl Simulator for DriftSim {
        fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
            use rand::Rng;

            let mut next = state.clone();
            next.t += 1;
            for index in 0..next.z.len() {
                let noise = (rng.gen::<f64>() - 0.5) * 0.01;
                next.z[index] = (next.z[index] + action.magnitude * 0.04 + noise).clamp(0.0, 1.0);
            }
            next
        }
    }

    fn sweep_definition() -> SweepDefinition {
        let mut definition = SweepDefinition::new("dose-sweep", "Dose Sweep");
        definition.strategy = SweepStrategy::Grid;
        definition.parameters.push(SweepParameter {
            name: "dose".into(),
            values: vec![
                ParameterValue::Int(1),
                ParameterValue::Int(2),
                ParameterValue::Int(3),
            ],
        });
        definition
    }

    fn build_parameter_set(case: &SweepCase) -> Result<ExperimentParameterSet, String> {
        let dose = match case.parameters.get("dose") {
            Some(ParameterValue::Int(value)) => *value as f64,
            _ => return Err("dose must be an int".into()),
        };

        let mut scenario = Scenario::new(
            format!("scenario-{}", case.case_id),
            format!("Scenario {}", case.case_id),
            SimState::new(vec![0.4], vec![0.0], vec![0.2]),
            4,
        );
        scenario.actions = vec![Action {
            dimension: Some(0),
            magnitude: dose,
            action_type: ActionType::Intervention,
            metadata: None,
        }];

        let mut parameter_set = ExperimentParameterSet::new(
            format!("ps-{}", case.case_id),
            format!("Parameter Set {}", case.case_id),
            scenario,
        );
        parameter_set.monte_carlo = Some(MonteCarloConfig::with_seed(32, 4, 9));
        Ok(parameter_set)
    }

    #[test]
    fn test_calibrate_ranks_best_candidate_first() {
        let observed = ObservedTrajectory::new("obs-1", "Observed", vec![0.52, 0.64, 0.76, 0.88]);
        let result = calibrate(
            &DriftSim,
            &observed,
            &sweep_definition(),
            &CalibrationConfig::default(),
            build_parameter_set,
        )
        .unwrap();

        assert_eq!(result.candidates.len(), 3);
        assert_eq!(result.best_case_id.as_deref(), Some("dose-sweep-3"));
        assert_eq!(result.candidates[0].case.case_id, "dose-sweep-3");
        assert!(result.candidates[0].score <= result.candidates[1].score);
    }

    #[test]
    fn test_calibrate_experiment_inherits_default_monte_carlo_and_records_bundle() {
        let mut spec = ExperimentSpec::new(
            "exp-1",
            "Calibration",
            Scenario::new("baseline", "Baseline", SimState::zeros(1), 4),
        );
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(32, 4, 11));
        let observed = ObservedTrajectory::new("obs-1", "Observed", vec![0.45, 0.52, 0.6, 0.7]);

        let result = calibrate_experiment(
            &DriftSim,
            &spec,
            &observed,
            &sweep_definition(),
            &CalibrationConfig::default(),
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
                        _ => return Err("dose must be an int".into()),
                    },
                    action_type: ActionType::Intervention,
                    metadata: None,
                });
                Ok(parameter_set)
            },
        )
        .unwrap();

        assert!(result.bundle.is_some());
        assert_eq!(result.bundle.as_ref().unwrap().parameter_sets.len(), 3);
        assert_eq!(result.bundle.as_ref().unwrap().runs.len(), 3);
        assert!(result
            .candidates
            .iter()
            .all(|candidate| candidate.parameter_set.monte_carlo.is_some()));
    }

    #[test]
    fn test_calibrate_continue_mode_collects_failures() {
        let observed = ObservedTrajectory::new("obs-1", "Observed", vec![0.45, 0.52, 0.6, 0.7]);
        let result = calibrate(
            &DriftSim,
            &observed,
            &sweep_definition(),
            &CalibrationConfig {
                failure_mode: CalibrationFailureMode::Continue,
                ..CalibrationConfig::default()
            },
            |case| {
                if matches!(case.parameters.get("dose"), Some(ParameterValue::Int(2))) {
                    return Err("dose 2 rejected".into());
                }
                build_parameter_set(case)
            },
        )
        .unwrap();

        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.candidates.len(), 2);
    }

    #[test]
    fn test_observed_summary_includes_composure_when_threshold_present() {
        let mut observed = ObservedTrajectory::new("obs-1", "Observed", vec![0.8, 0.6, 0.4, 0.7]);
        observed.failure_threshold = Some(0.5);

        let summary = summarize_observed(&observed).unwrap();
        assert!(summary.monte_carlo.is_some());
        assert!(summary.composure.is_some());
    }
}
