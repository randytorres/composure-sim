use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    analyze_composure_checked, build_deterministic_report, compare_monte_carlo_results,
    run_scenario_monte_carlo_checked, summarize_run, Action, ComparisonConfig, ComposureError,
    ConditionalActionRule, DeterministicReport, ExperimentExecutionConfig, ExperimentOutcome,
    MonteCarloConfig, MonteCarloError, RunSummary, Scenario, ScenarioError, SimState, Simulator,
    TrajectoryComparison,
};

/// Input describing one branch in a counterfactual comparison.
///
/// The caller provides the shared `state_at_branch` separately to
/// [`run_counterfactual`]. This first slice does not fork from replay logs or
/// resume an in-flight RNG stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualBranchInput {
    pub branch_id: String,
    pub intervention_label: String,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub conditional_actions: Vec<ConditionalActionRule>,
}

/// Executed branch outcome in a counterfactual comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualBranch {
    pub branch_id: String,
    pub branch_from_t: usize,
    pub intervention_label: String,
    pub outcome: ExperimentOutcome,
    pub summary: RunSummary,
}

/// Complete deterministic counterfactual result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualResult {
    pub baseline: CounterfactualBranch,
    pub candidate: CounterfactualBranch,
    pub comparison: TrajectoryComparison,
    pub report: DeterministicReport,
}

/// Execution and comparison settings for a counterfactual run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualConfig {
    pub monte_carlo: MonteCarloConfig,
    pub execution: ExperimentExecutionConfig,
    pub comparison: ComparisonConfig,
    pub analysis_failure_threshold: Option<f64>,
}

impl CounterfactualConfig {
    pub fn new(monte_carlo: MonteCarloConfig) -> Self {
        Self {
            monte_carlo,
            execution: ExperimentExecutionConfig::default(),
            comparison: ComparisonConfig::default(),
            analysis_failure_threshold: None,
        }
    }

    pub fn validate(&self) -> Result<(), CounterfactualError> {
        self.monte_carlo
            .validate()
            .map_err(|source| CounterfactualError::InvalidMonteCarloConfig { source })?;
        self.comparison
            .validate()
            .map_err(CounterfactualError::Compare)?;
        Ok(())
    }
}

/// Run two matched-seed branches from the same explicit state and compare the
/// outcomes.
///
/// Both branches reuse the same Monte Carlo seed base so the comparison isolates
/// the effect of branch-local actions and conditional rules rather than sampling
/// noise. `comparison.failure_threshold` only affects the comparison artifact;
/// branch execution and composure summaries use `analysis_failure_threshold`.
pub fn run_counterfactual<S: Simulator>(
    sim: &S,
    state_at_branch: &SimState,
    baseline: &CounterfactualBranchInput,
    candidate: &CounterfactualBranchInput,
    config: &CounterfactualConfig,
) -> Result<CounterfactualResult, CounterfactualError> {
    config.validate()?;
    validate_branch_input(state_at_branch, baseline)?;
    validate_branch_input(state_at_branch, candidate)?;

    if baseline.branch_id == candidate.branch_id {
        return Err(CounterfactualError::DuplicateBranchId(
            baseline.branch_id.clone(),
        ));
    }

    let baseline_branch = execute_branch(
        sim,
        state_at_branch,
        baseline,
        &config.monte_carlo,
        &config.execution,
        config.analysis_failure_threshold,
    )?;
    let candidate_branch = execute_branch(
        sim,
        state_at_branch,
        candidate,
        &config.monte_carlo,
        &config.execution,
        config.analysis_failure_threshold,
    )?;

    let baseline_monte_carlo = baseline_branch
        .outcome
        .monte_carlo
        .as_ref()
        .expect("counterfactual branch always records Monte Carlo");
    let candidate_monte_carlo = candidate_branch
        .outcome
        .monte_carlo
        .as_ref()
        .expect("counterfactual branch always records Monte Carlo");

    let trajectory_comparison = compare_monte_carlo_results(
        baseline_monte_carlo,
        candidate_monte_carlo,
        &config.comparison,
    )
    .map_err(CounterfactualError::Compare)?;

    let report = build_deterministic_report(
        &baseline_branch.summary,
        &candidate_branch.summary,
        Some(&trajectory_comparison),
    );

    Ok(CounterfactualResult {
        baseline: baseline_branch,
        candidate: candidate_branch,
        comparison: trajectory_comparison,
        report,
    })
}

fn execute_branch<S: Simulator>(
    sim: &S,
    state_at_branch: &SimState,
    input: &CounterfactualBranchInput,
    monte_carlo: &MonteCarloConfig,
    execution: &ExperimentExecutionConfig,
    analysis_failure_threshold: Option<f64>,
) -> Result<CounterfactualBranch, CounterfactualError> {
    let scenario = Scenario {
        id: format!("counterfactual-{}", input.branch_id),
        name: format!("Counterfactual {}", input.intervention_label),
        initial_state: state_at_branch.clone(),
        actions: input.actions.clone(),
        time_steps: monte_carlo.time_steps,
        conditional_actions: input.conditional_actions.clone(),
        failure_threshold: analysis_failure_threshold,
        metadata: None,
    };
    scenario
        .validate()
        .map_err(|source| CounterfactualError::InvalidScenario {
            branch_id: input.branch_id.clone(),
            source,
        })?;

    let monte_carlo_result =
        run_scenario_monte_carlo_checked(sim, &scenario, monte_carlo, execution.retain_paths)
            .map_err(|source| CounterfactualError::MonteCarlo {
                branch_id: input.branch_id.clone(),
                source,
            })?;

    let composure = if execution.analyze_composure {
        Some(
            analyze_composure_checked(
                &monte_carlo_result.mean_trajectory,
                analysis_failure_threshold.unwrap_or(0.0),
            )
            .map_err(|source| CounterfactualError::Composure {
                branch_id: input.branch_id.clone(),
                source,
            })?,
        )
    } else {
        None
    };

    let outcome = ExperimentOutcome {
        monte_carlo: Some(monte_carlo_result),
        composure,
        replay: None,
        metadata: None,
    };
    let summary = summarize_run(outcome.monte_carlo.as_ref(), outcome.composure.as_ref());

    Ok(CounterfactualBranch {
        branch_id: input.branch_id.clone(),
        branch_from_t: state_at_branch.t,
        intervention_label: input.intervention_label.clone(),
        outcome,
        summary,
    })
}

pub(crate) fn validate_branch_input(
    state_at_branch: &SimState,
    input: &CounterfactualBranchInput,
) -> Result<(), CounterfactualError> {
    if input.branch_id.trim().is_empty() {
        return Err(CounterfactualError::EmptyBranchId);
    }
    if input.intervention_label.trim().is_empty() {
        return Err(CounterfactualError::EmptyInterventionLabel {
            branch_id: input.branch_id.clone(),
        });
    }

    let dimensions = state_at_branch.dimensions();
    if let Some((action_index, dimension)) =
        input.actions.iter().enumerate().find_map(|(idx, action)| {
            action
                .dimension
                .filter(|&value| value >= dimensions)
                .map(|value| (idx, value))
        })
    {
        return Err(CounterfactualError::InvalidActionDimension {
            branch_id: input.branch_id.clone(),
            action_index,
            dimension,
            dimensions,
        });
    }

    if let Some(action_index) = input
        .actions
        .iter()
        .enumerate()
        .find_map(|(idx, action)| (!action.magnitude.is_finite()).then_some(idx))
    {
        return Err(CounterfactualError::InvalidActionMagnitude {
            branch_id: input.branch_id.clone(),
            action_index,
        });
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum CounterfactualError {
    #[error("counterfactual branch ID cannot be empty")]
    EmptyBranchId,
    #[error("counterfactual branch IDs must be unique, got {0}")]
    DuplicateBranchId(String),
    #[error("counterfactual branch {branch_id} intervention label cannot be empty")]
    EmptyInterventionLabel { branch_id: String },
    #[error(
        "counterfactual branch {branch_id} action {action_index} targets dimension {dimension}, but state has {dimensions} dimensions"
    )]
    InvalidActionDimension {
        branch_id: String,
        action_index: usize,
        dimension: usize,
        dimensions: usize,
    },
    #[error("counterfactual branch {branch_id} action {action_index} magnitude must be finite")]
    InvalidActionMagnitude {
        branch_id: String,
        action_index: usize,
    },
    #[error("invalid counterfactual Monte Carlo configuration: {source}")]
    InvalidMonteCarloConfig { source: MonteCarloError },
    #[error("counterfactual branch {branch_id} is invalid: {source}")]
    InvalidScenario {
        branch_id: String,
        source: ScenarioError,
    },
    #[error("counterfactual Monte Carlo failed for branch {branch_id}: {source}")]
    MonteCarlo {
        branch_id: String,
        source: MonteCarloError,
    },
    #[error("counterfactual composure analysis failed for branch {branch_id}: {source}")]
    Composure {
        branch_id: String,
        source: ComposureError,
    },
    #[error("counterfactual comparison failed: {0}")]
    Compare(crate::CompareError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionType, ConditionalTrigger, FailureComparisonOutcome, ScenarioError};

    struct DeterministicSim;

    impl Simulator for DeterministicSim {
        fn step(
            &self,
            state: &SimState,
            action: &Action,
            _rng: &mut dyn rand::RngCore,
        ) -> SimState {
            let mut next = state.clone();
            next.t += 1;
            for i in 0..next.z.len() {
                let effect = if action.dimension.map(|value| value == i).unwrap_or(true) {
                    action.magnitude
                } else {
                    0.0
                };
                next.z[i] = (next.z[i] + effect).clamp(0.0, 1.0);
            }
            next
        }
    }

    struct NoisySim;

    impl Simulator for NoisySim {
        fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
            let mut next = state.clone();
            next.t += 1;
            let noise = (rng.next_u32() % 100) as f64 / 10_000.0;
            for i in 0..next.z.len() {
                let effect = if action.dimension.map(|value| value == i).unwrap_or(true) {
                    action.magnitude + noise
                } else {
                    noise
                };
                next.z[i] = (next.z[i] + effect).clamp(0.0, 1.0);
            }
            next
        }
    }

    fn branch_input(id: &str, magnitude: f64) -> CounterfactualBranchInput {
        CounterfactualBranchInput {
            branch_id: id.into(),
            intervention_label: id.into(),
            actions: vec![Action {
                dimension: Some(0),
                magnitude,
                action_type: ActionType::Intervention,
                metadata: None,
            }],
            conditional_actions: Vec::new(),
        }
    }

    fn test_config(monte_carlo: MonteCarloConfig) -> CounterfactualConfig {
        CounterfactualConfig::new(monte_carlo)
    }

    #[test]
    fn test_run_counterfactual_builds_comparison_and_report() {
        let mut state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);
        state.t = 5;
        let mut config = test_config(MonteCarloConfig::with_seed(1, 3, 7));
        config.execution = ExperimentExecutionConfig {
            retain_paths: true,
            analyze_composure: true,
        };
        config.comparison = ComparisonConfig {
            failure_threshold: Some(0.4),
            ..ComparisonConfig::default()
        };
        config.analysis_failure_threshold = Some(0.4);

        let result = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &branch_input("candidate", 0.2),
            &config,
        )
        .unwrap();

        assert_eq!(result.baseline.branch_from_t, 5);
        assert_eq!(result.candidate.branch_from_t, 5);
        assert_eq!(
            result
                .baseline
                .outcome
                .monte_carlo
                .as_ref()
                .unwrap()
                .config
                .seed_base,
            7
        );
        assert_eq!(
            result
                .candidate
                .outcome
                .monte_carlo
                .as_ref()
                .unwrap()
                .config
                .seed_base,
            7
        );
        assert!(result.comparison.metrics.end_delta > 0.0);
        assert!(result.report.end_delta.delta.unwrap() > 0.0);
    }

    #[test]
    fn test_run_counterfactual_is_deterministic() {
        let state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);
        let mut config = test_config(MonteCarloConfig::with_seed(8, 4, 11));
        config.execution = ExperimentExecutionConfig {
            retain_paths: false,
            analyze_composure: true,
        };

        let first = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &branch_input("candidate", 0.2),
            &config,
        )
        .unwrap();
        let second = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &branch_input("candidate", 0.2),
            &config,
        )
        .unwrap();

        assert_eq!(
            first
                .candidate
                .outcome
                .monte_carlo
                .as_ref()
                .unwrap()
                .mean_trajectory,
            second
                .candidate
                .outcome
                .monte_carlo
                .as_ref()
                .unwrap()
                .mean_trajectory
        );
    }

    #[test]
    fn test_run_counterfactual_matches_branch_seeds_for_equivalent_inputs() {
        let state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);
        let mut config = test_config(MonteCarloConfig::with_seed(4, 4, 11));
        config.execution = ExperimentExecutionConfig {
            retain_paths: true,
            analyze_composure: false,
        };

        let result = run_counterfactual(
            &NoisySim,
            &state,
            &branch_input("baseline", 0.1),
            &branch_input("candidate", 0.1),
            &config,
        )
        .unwrap();

        let baseline = result.baseline.outcome.monte_carlo.as_ref().unwrap();
        let candidate = result.candidate.outcome.monte_carlo.as_ref().unwrap();

        let baseline_seeds: Vec<u64> = baseline.paths.iter().map(|path| path.seed).collect();
        let candidate_seeds: Vec<u64> = candidate.paths.iter().map(|path| path.seed).collect();

        assert_eq!(baseline_seeds, candidate_seeds);
        assert_eq!(baseline.mean_trajectory, candidate.mean_trajectory);
        assert_eq!(baseline.paths.len(), candidate.paths.len());
        for (base, cand) in baseline.paths.iter().zip(candidate.paths.iter()) {
            assert_eq!(base.health_indices, cand.health_indices);
        }
    }

    #[test]
    fn test_run_counterfactual_rejects_duplicate_branch_ids() {
        let state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);

        let err = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("branch", 0.0),
            &branch_input("branch", 0.2),
            &test_config(MonteCarloConfig::with_seed(1, 3, 7)),
        )
        .unwrap_err();

        assert!(matches!(err, CounterfactualError::DuplicateBranchId(id) if id == "branch"));
    }

    #[test]
    fn test_run_counterfactual_rejects_empty_branch_id() {
        let state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);
        let invalid = CounterfactualBranchInput {
            branch_id: "  ".into(),
            intervention_label: "baseline".into(),
            actions: Vec::new(),
            conditional_actions: Vec::new(),
        };

        let err = run_counterfactual(
            &DeterministicSim,
            &state,
            &invalid,
            &branch_input("candidate", 0.2),
            &test_config(MonteCarloConfig::with_seed(1, 3, 7)),
        )
        .unwrap_err();

        assert!(matches!(err, CounterfactualError::EmptyBranchId));
    }

    #[test]
    fn test_run_counterfactual_rejects_empty_intervention_label() {
        let state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);
        let invalid = CounterfactualBranchInput {
            branch_id: "candidate".into(),
            intervention_label: " ".into(),
            actions: Vec::new(),
            conditional_actions: Vec::new(),
        };

        let err = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &invalid,
            &test_config(MonteCarloConfig::with_seed(1, 3, 7)),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            CounterfactualError::EmptyInterventionLabel { branch_id } if branch_id == "candidate"
        ));
    }

    #[test]
    fn test_run_counterfactual_rejects_invalid_action_dimension() {
        let state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);
        let invalid = CounterfactualBranchInput {
            branch_id: "candidate".into(),
            intervention_label: "candidate".into(),
            actions: vec![Action {
                dimension: Some(1),
                magnitude: 0.2,
                action_type: ActionType::Intervention,
                metadata: None,
            }],
            conditional_actions: Vec::new(),
        };

        let err = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &invalid,
            &test_config(MonteCarloConfig::with_seed(1, 3, 7)),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            CounterfactualError::InvalidActionDimension {
                branch_id,
                action_index: 0,
                dimension: 1,
                dimensions: 1,
            } if branch_id == "candidate"
        ));
    }

    #[test]
    fn test_run_counterfactual_rejects_non_finite_action_magnitude() {
        let state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);
        let invalid = CounterfactualBranchInput {
            branch_id: "candidate".into(),
            intervention_label: "candidate".into(),
            actions: vec![Action {
                dimension: Some(0),
                magnitude: f64::NAN,
                action_type: ActionType::Intervention,
                metadata: None,
            }],
            conditional_actions: Vec::new(),
        };

        let err = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &invalid,
            &test_config(MonteCarloConfig::with_seed(1, 3, 7)),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            CounterfactualError::InvalidActionMagnitude {
                branch_id,
                action_index: 0,
            } if branch_id == "candidate"
        ));
    }

    #[test]
    fn test_run_counterfactual_reports_failure_shift_when_comparison_threshold_is_set() {
        let state = SimState::new(vec![0.7], vec![0.0], vec![0.0]);
        let mut config = test_config(MonteCarloConfig::with_seed(4, 3, 9));
        config.execution = ExperimentExecutionConfig {
            retain_paths: false,
            analyze_composure: true,
        };
        config.comparison = ComparisonConfig {
            failure_threshold: Some(0.5),
            ..ComparisonConfig::default()
        };
        let baseline = CounterfactualBranchInput {
            branch_id: "baseline".into(),
            intervention_label: "baseline".into(),
            actions: vec![
                Action {
                    dimension: Some(0),
                    magnitude: -0.3,
                    action_type: ActionType::Intervention,
                    metadata: None,
                };
                3
            ],
            conditional_actions: Vec::new(),
        };
        let candidate = CounterfactualBranchInput {
            branch_id: "candidate".into(),
            intervention_label: "candidate".into(),
            actions: vec![
                Action {
                    dimension: Some(0),
                    magnitude: -0.15,
                    action_type: ActionType::Intervention,
                    metadata: None,
                };
                3
            ],
            conditional_actions: Vec::new(),
        };

        let result =
            run_counterfactual(&DeterministicSim, &state, &baseline, &candidate, &config).unwrap();

        let failure = result.comparison.metrics.failure.as_ref().unwrap();
        assert_eq!(failure.outcome, FailureComparisonOutcome::BothFailed);
        assert_eq!(failure.baseline_break_t, Some(0));
        assert_eq!(failure.candidate_break_t, Some(1));
        assert_eq!(failure.shift, Some(1));
        assert_eq!(
            result.report.comparison.as_ref().unwrap().failure_shift,
            Some(1)
        );
        assert!(result.baseline.summary.composure.is_some());
        assert!(result.candidate.summary.composure.is_some());
    }

    #[test]
    fn test_run_counterfactual_without_composure_still_builds_report() {
        let state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);
        let mut config = test_config(MonteCarloConfig::with_seed(1, 3, 7));
        config.execution = ExperimentExecutionConfig {
            retain_paths: false,
            analyze_composure: false,
        };

        let result = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &branch_input("candidate", 0.2),
            &config,
        )
        .unwrap();

        assert!(result.baseline.outcome.composure.is_none());
        assert!(result.candidate.outcome.composure.is_none());
        assert!(result.baseline.summary.composure.is_none());
        assert!(result.candidate.summary.composure.is_none());
        assert!(result.report.comparison.is_some());
        assert_eq!(result.report.residual_damage_delta.delta, None);
    }

    #[test]
    fn test_run_counterfactual_accepts_comparison_threshold_outside_unit_range() {
        let state = SimState::new(vec![0.3], vec![0.0], vec![0.0]);
        let mut config = test_config(MonteCarloConfig::with_seed(1, 3, 7));
        config.execution = ExperimentExecutionConfig {
            retain_paths: false,
            analyze_composure: false,
        };
        config.comparison = ComparisonConfig {
            failure_threshold: Some(2.0),
            ..ComparisonConfig::default()
        };

        let result = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &branch_input("candidate", 0.2),
            &config,
        )
        .unwrap();

        let failure = result.comparison.metrics.failure.as_ref().unwrap();
        assert_eq!(failure.threshold, 2.0);
        assert_eq!(failure.outcome, FailureComparisonOutcome::BothFailed);
    }

    #[test]
    fn test_run_counterfactual_supports_conditional_actions() {
        let state = SimState::new(vec![0.2], vec![0.0], vec![0.0]);
        let mut config = test_config(MonteCarloConfig::with_seed(1, 4, 7));
        config.execution = ExperimentExecutionConfig {
            retain_paths: true,
            analyze_composure: false,
        };
        let candidate = CounterfactualBranchInput {
            branch_id: "candidate".into(),
            intervention_label: "candidate".into(),
            actions: Vec::new(),
            conditional_actions: vec![ConditionalActionRule {
                id: "rescue".into(),
                trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.3 },
                action: Action {
                    dimension: Some(0),
                    magnitude: 0.4,
                    action_type: ActionType::Intervention,
                    metadata: None,
                },
                delay_steps: 1,
                cooldown_steps: 10,
                priority: 1,
                max_fires: Some(1),
            }],
        };

        let result = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &candidate,
            &config,
        )
        .unwrap();

        let series =
            &result.candidate.outcome.monte_carlo.as_ref().unwrap().paths[0].health_indices;
        assert_eq!(series.len(), 4);
        assert!((series[0] - 0.2).abs() < 1e-9);
        assert!((series[1] - series[0]).abs() < 1e-9);
        assert!(series[2] > series[1]);
    }

    #[test]
    fn test_run_counterfactual_surfaces_invalid_conditional_actions() {
        let state = SimState::new(vec![0.2], vec![0.0], vec![0.0]);
        let candidate = CounterfactualBranchInput {
            branch_id: "candidate".into(),
            intervention_label: "candidate".into(),
            actions: Vec::new(),
            conditional_actions: vec![ConditionalActionRule {
                id: "bad-rule".into(),
                trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.3 },
                action: Action {
                    dimension: Some(1),
                    magnitude: 0.4,
                    action_type: ActionType::Intervention,
                    metadata: None,
                },
                delay_steps: 0,
                cooldown_steps: 0,
                priority: 0,
                max_fires: Some(1),
            }],
        };

        let err = run_counterfactual(
            &DeterministicSim,
            &state,
            &branch_input("baseline", 0.0),
            &candidate,
            &test_config(MonteCarloConfig::with_seed(1, 4, 7)),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            CounterfactualError::InvalidScenario {
                branch_id,
                source: ScenarioError::InvalidConditionalActionDimension {
                    rule_index: 0,
                    dimension: 1,
                    dimensions: 1,
                },
            } if branch_id == "candidate"
        ));
    }
}
