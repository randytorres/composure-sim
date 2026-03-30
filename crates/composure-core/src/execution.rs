use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    analyze_composure_checked, run_scenario_monte_carlo_checked, ComposureError, ExperimentOutcome,
    ExperimentParameterSet, ExperimentRunRecord, ExperimentSpec, MonteCarloConfig, MonteCarloError,
    ScenarioError, Simulator,
};

/// Controls which artifacts are produced when executing an experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentExecutionConfig {
    /// Keep individual Monte Carlo paths in the stored output.
    pub retain_paths: bool,
    /// Derive composure metrics from the mean trajectory after Monte Carlo execution.
    pub analyze_composure: bool,
}

impl Default for ExperimentExecutionConfig {
    fn default() -> Self {
        Self {
            retain_paths: false,
            analyze_composure: true,
        }
    }
}

/// Execute an experiment spec using its default Monte Carlo configuration.
pub fn execute_experiment_spec<S: Simulator>(
    run_id: impl Into<String>,
    sim: &S,
    spec: &ExperimentSpec,
    execution: &ExperimentExecutionConfig,
) -> Result<ExperimentRunRecord, ExperimentExecutionError> {
    spec.validate()
        .map_err(ExperimentExecutionError::InvalidExperimentSpec)?;

    let monte_carlo = spec
        .default_monte_carlo
        .as_ref()
        .ok_or(ExperimentExecutionError::MissingMonteCarloConfig)?;

    execute(
        run_id.into(),
        sim,
        None,
        &spec.scenario,
        monte_carlo,
        execution,
    )
}

/// Execute a named experiment parameter set.
pub fn execute_parameter_set<S: Simulator>(
    run_id: impl Into<String>,
    sim: &S,
    parameter_set: &ExperimentParameterSet,
    execution: &ExperimentExecutionConfig,
) -> Result<ExperimentRunRecord, ExperimentExecutionError> {
    parameter_set
        .validate()
        .map_err(ExperimentExecutionError::InvalidParameterSet)?;

    let monte_carlo = parameter_set
        .monte_carlo
        .as_ref()
        .ok_or(ExperimentExecutionError::MissingMonteCarloConfig)?;

    execute(
        run_id.into(),
        sim,
        Some(parameter_set.id.clone()),
        &parameter_set.scenario,
        monte_carlo,
        execution,
    )
}

fn execute<S: Simulator>(
    run_id: String,
    sim: &S,
    parameter_set_id: Option<String>,
    scenario: &crate::Scenario,
    monte_carlo: &MonteCarloConfig,
    execution: &ExperimentExecutionConfig,
) -> Result<ExperimentRunRecord, ExperimentExecutionError> {
    scenario
        .validate()
        .map_err(ExperimentExecutionError::InvalidScenario)?;

    let result =
        run_scenario_monte_carlo_checked(sim, scenario, monte_carlo, execution.retain_paths)
            .map_err(ExperimentExecutionError::MonteCarlo)?;

    let composure = if execution.analyze_composure {
        Some(
            analyze_composure_checked(
                &result.mean_trajectory,
                scenario.failure_threshold.unwrap_or(0.0),
            )
            .map_err(ExperimentExecutionError::Composure)?,
        )
    } else {
        None
    };

    let outcome = ExperimentOutcome {
        monte_carlo: Some(result),
        composure,
        replay: None,
        metadata: None,
    };

    Ok(ExperimentRunRecord::running(
        run_id,
        parameter_set_id.as_deref(),
        Some(monte_carlo.seed_base),
    )
    .mark_completed(outcome))
}

#[derive(Debug, Error)]
pub enum ExperimentExecutionError {
    #[error("invalid experiment spec: {0}")]
    InvalidExperimentSpec(crate::ExperimentError),
    #[error("invalid parameter set: {0}")]
    InvalidParameterSet(crate::ExperimentError),
    #[error("invalid scenario: {0}")]
    InvalidScenario(ScenarioError),
    #[error("experiment execution requires a Monte Carlo configuration")]
    MissingMonteCarloConfig,
    #[error("Monte Carlo execution failed: {0}")]
    MonteCarlo(MonteCarloError),
    #[error("composure analysis failed: {0}")]
    Composure(ComposureError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, ActionType, ConditionalActionRule, ConditionalTrigger, SimState};

    struct DriftSim;

    impl Simulator for DriftSim {
        fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
            use rand::Rng;

            let mut next = state.clone();
            next.t += 1;
            for i in 0..next.z.len() {
                let noise = (rng.gen::<f64>() - 0.5) * 0.02;
                next.z[i] = (next.z[i] + action.magnitude * 0.01 + noise).clamp(0.0, 1.0);
            }
            next
        }
    }

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

    #[test]
    fn test_execute_experiment_spec_produces_completed_run() {
        let mut spec = ExperimentSpec::new(
            "exp-1",
            "Baseline",
            crate::Scenario::new("baseline", "Baseline", SimState::zeros(1), 4),
        );
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(8, 4, 7));

        let run = execute_experiment_spec(
            "run-1",
            &DriftSim,
            &spec,
            &ExperimentExecutionConfig::default(),
        )
        .unwrap();

        assert_eq!(run.status, crate::ExperimentRunStatus::Completed);
        assert!(run.outcome.as_ref().unwrap().monte_carlo.is_some());
        assert!(run.outcome.as_ref().unwrap().composure.is_some());
    }

    #[test]
    fn test_execute_parameter_set_uses_parameter_set_id() {
        let mut set = ExperimentParameterSet::new(
            "variant-a",
            "Variant A",
            crate::Scenario::new("variant", "Variant", SimState::zeros(1), 3),
        );
        set.scenario.actions.push(Action {
            dimension: Some(0),
            magnitude: 1.0,
            action_type: ActionType::Intervention,
            metadata: None,
        });
        set.monte_carlo = Some(MonteCarloConfig::with_seed(4, 3, 11));

        let run = execute_parameter_set(
            "run-variant-a",
            &DriftSim,
            &set,
            &ExperimentExecutionConfig {
                analyze_composure: false,
                retain_paths: true,
            },
        )
        .unwrap();

        assert_eq!(run.parameter_set_id.as_deref(), Some("variant-a"));
        let outcome = run.outcome.unwrap();
        assert!(outcome.composure.is_none());
        assert_eq!(outcome.monte_carlo.unwrap().paths.len(), 4);
    }

    #[test]
    fn test_execute_parameter_set_requires_monte_carlo_config() {
        let set = ExperimentParameterSet::new(
            "variant-a",
            "Variant A",
            crate::Scenario::new("variant", "Variant", SimState::zeros(1), 3),
        );

        let err = execute_parameter_set(
            "run-variant-a",
            &DriftSim,
            &set,
            &ExperimentExecutionConfig::default(),
        )
        .unwrap_err();

        assert!(matches!(
            err,
            ExperimentExecutionError::MissingMonteCarloConfig
        ));
    }

    #[test]
    fn test_execute_experiment_spec_applies_conditional_actions() {
        let mut spec = ExperimentSpec::new(
            "exp-conditional",
            "Conditional",
            crate::Scenario::new(
                "conditional-scenario",
                "Conditional Scenario",
                SimState::new(vec![0.2], vec![0.0], vec![0.0]),
                4,
            ),
        );
        spec.scenario
            .conditional_actions
            .push(ConditionalActionRule {
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
            });
        spec.default_monte_carlo = Some(MonteCarloConfig::with_seed(1, 4, 7));

        let run = execute_experiment_spec(
            "run-conditional",
            &DeterministicSim,
            &spec,
            &ExperimentExecutionConfig {
                analyze_composure: false,
                retain_paths: true,
            },
        )
        .unwrap();

        let monte_carlo = run.outcome.unwrap().monte_carlo.unwrap();
        let series = &monte_carlo.paths[0].health_indices;
        assert_eq!(series.len(), 4);
        assert!((series[0] - 0.2).abs() < 1e-9);
        assert!((series[1] - series[0]).abs() < 1e-9);
        assert!(series[2] > series[1]);
        assert!((series[3] - series[2]).abs() < 1e-9);
    }
}
