use rand::rngs::StdRng;
use rand::SeedableRng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    scenario::{ConditionalTrigger, Scenario},
    simulator::Simulator,
    state::{Action, ActionType, SimState},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloConfig {
    pub num_paths: usize,
    pub time_steps: usize,
    pub seed_base: u64,
}

impl MonteCarloConfig {
    pub fn new(num_paths: usize, time_steps: usize) -> Self {
        Self {
            num_paths,
            time_steps,
            seed_base: 42,
        }
    }

    pub fn with_seed(num_paths: usize, time_steps: usize, seed: u64) -> Self {
        Self {
            num_paths,
            time_steps,
            seed_base: seed,
        }
    }

    pub fn validate(&self) -> Result<(), MonteCarloError> {
        if self.num_paths == 0 {
            return Err(MonteCarloError::ZeroPaths);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    pub health_indices: Vec<f64>,
    pub final_state: SimState,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResult {
    pub paths: Vec<PathResult>,
    pub percentiles: PercentileBands,
    pub mean_trajectory: Vec<f64>,
    pub config: MonteCarloConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileBands {
    pub p10: Vec<f64>,
    pub p25: Vec<f64>,
    pub p50: Vec<f64>,
    pub p75: Vec<f64>,
    pub p90: Vec<f64>,
}

pub fn run_monte_carlo<S: Simulator>(
    sim: &S,
    initial_state: &SimState,
    actions: &[Action],
    config: &MonteCarloConfig,
    retain_paths: bool,
) -> MonteCarloResult {
    run_monte_carlo_checked(sim, initial_state, actions, config, retain_paths)
        .expect("invalid Monte Carlo configuration")
}

pub fn run_monte_carlo_checked<S: Simulator>(
    sim: &S,
    initial_state: &SimState,
    actions: &[Action],
    config: &MonteCarloConfig,
    retain_paths: bool,
) -> Result<MonteCarloResult, MonteCarloError> {
    config.validate()?;

    let path_results: Vec<PathResult> = (0..config.num_paths)
        .into_par_iter()
        .map(|path_idx| {
            let seed = config.seed_base.wrapping_add(path_idx as u64);
            let mut rng = StdRng::seed_from_u64(seed);
            let mut state = initial_state.clone();
            let mut health_indices = Vec::with_capacity(config.time_steps);

            for t in 0..config.time_steps {
                let action = actions.get(t).cloned().unwrap_or_default();
                state = sim.step(&state, &action, &mut rng);
                health_indices.push(sim.health_index(&state));
            }

            PathResult {
                health_indices,
                final_state: state,
                seed,
            }
        })
        .collect();

    Ok(build_result(path_results, config, retain_paths))
}

pub fn run_scenario_monte_carlo<S: Simulator>(
    sim: &S,
    scenario: &Scenario,
    config: &MonteCarloConfig,
    retain_paths: bool,
) -> MonteCarloResult {
    run_scenario_monte_carlo_checked(sim, scenario, config, retain_paths)
        .expect("invalid scenario Monte Carlo configuration")
}

pub fn run_scenario_monte_carlo_checked<S: Simulator>(
    sim: &S,
    scenario: &Scenario,
    config: &MonteCarloConfig,
    retain_paths: bool,
) -> Result<MonteCarloResult, MonteCarloError> {
    config.validate()?;
    scenario
        .validate()
        .map_err(MonteCarloError::InvalidScenario)?;

    if scenario.time_steps != config.time_steps {
        return Err(MonteCarloError::TimeStepsMismatch {
            scenario_time_steps: scenario.time_steps,
            monte_carlo_time_steps: config.time_steps,
        });
    }

    let path_results: Vec<PathResult> = (0..config.num_paths)
        .into_par_iter()
        .map(|path_idx| {
            let seed = config.seed_base.wrapping_add(path_idx as u64);
            let mut rng = StdRng::seed_from_u64(seed);
            let mut state = scenario.initial_state.clone();
            let mut health_indices = Vec::with_capacity(config.time_steps);
            let mut conditional_state =
                ConditionalActionState::new(scenario.conditional_actions.len());

            for step in 0..config.time_steps {
                let actions = actions_for_step(scenario, step, &mut conditional_state);
                let previous_state = state.clone();
                state = sim.step_actions(&state, &actions, &mut rng);
                health_indices.push(sim.health_index(&state));
                schedule_conditional_actions(
                    sim,
                    scenario,
                    &previous_state,
                    &state,
                    step,
                    config.time_steps,
                    &mut conditional_state,
                );
            }

            PathResult {
                health_indices,
                final_state: state,
                seed,
            }
        })
        .collect();

    Ok(build_result(path_results, config, retain_paths))
}

#[derive(Debug, Clone)]
struct ConditionalActionState {
    next_eligible_step: Vec<usize>,
    fire_counts: Vec<usize>,
    scheduled_actions: Vec<ScheduledConditionalAction>,
}

impl ConditionalActionState {
    fn new(rule_count: usize) -> Self {
        Self {
            next_eligible_step: vec![0; rule_count],
            fire_counts: vec![0; rule_count],
            scheduled_actions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct ScheduledConditionalAction {
    apply_at: usize,
    priority: i32,
    rule_index: usize,
    action: Action,
}

#[derive(Debug, Clone)]
struct StepAction {
    priority: i32,
    order: usize,
    action: Action,
}

fn actions_for_step(
    scenario: &Scenario,
    step: usize,
    conditional_state: &mut ConditionalActionState,
) -> Vec<Action> {
    let mut actions = Vec::new();
    let base_action = scenario.actions.get(step).cloned().unwrap_or_default();
    if is_effective_action(&base_action) {
        actions.push(StepAction {
            priority: 0,
            order: 0,
            action: base_action,
        });
    }

    let due_actions = take_due_actions(step, conditional_state);
    actions.extend(due_actions.into_iter().map(|scheduled| StepAction {
        priority: scheduled.priority,
        order: scheduled.rule_index + 1,
        action: scheduled.action,
    }));

    actions.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.order.cmp(&right.order))
    });

    actions.into_iter().map(|entry| entry.action).collect()
}

fn schedule_conditional_actions<S: Simulator>(
    sim: &S,
    scenario: &Scenario,
    previous_state: &SimState,
    next_state: &SimState,
    step: usize,
    total_steps: usize,
    conditional_state: &mut ConditionalActionState,
) {
    for (rule_index, rule) in scenario.conditional_actions.iter().enumerate() {
        if conditional_state.next_eligible_step[rule_index] > step {
            continue;
        }
        if rule
            .max_fires
            .is_some_and(|limit| conditional_state.fire_counts[rule_index] >= limit)
        {
            continue;
        }
        if !trigger_matches(sim, previous_state, next_state, &rule.trigger) {
            continue;
        }

        let apply_at = step + rule.delay_steps + 1;
        conditional_state.fire_counts[rule_index] += 1;
        conditional_state.next_eligible_step[rule_index] = step + rule.cooldown_steps + 1;

        if apply_at < total_steps {
            conditional_state
                .scheduled_actions
                .push(ScheduledConditionalAction {
                    apply_at,
                    priority: rule.priority,
                    rule_index,
                    action: rule.action.clone(),
                });
        }
    }
}

fn take_due_actions(
    step: usize,
    conditional_state: &mut ConditionalActionState,
) -> Vec<ScheduledConditionalAction> {
    let mut remaining = Vec::with_capacity(conditional_state.scheduled_actions.len());
    let mut due = Vec::new();

    for scheduled in conditional_state.scheduled_actions.drain(..) {
        if scheduled.apply_at == step {
            due.push(scheduled);
        } else {
            remaining.push(scheduled);
        }
    }

    conditional_state.scheduled_actions = remaining;
    due
}

fn trigger_matches<S: Simulator>(
    sim: &S,
    previous_state: &SimState,
    next_state: &SimState,
    trigger: &ConditionalTrigger,
) -> bool {
    match trigger {
        ConditionalTrigger::HealthIndexBelow { threshold } => {
            sim.health_index(next_state) < *threshold
        }
        ConditionalTrigger::HealthIndexAbove { threshold } => {
            sim.health_index(next_state) > *threshold
        }
        ConditionalTrigger::HealthIndexCrossesBelow { threshold } => {
            sim.health_index(previous_state) > *threshold
                && sim.health_index(next_state) <= *threshold
        }
        ConditionalTrigger::HealthIndexCrossesAbove { threshold } => {
            sim.health_index(previous_state) < *threshold
                && sim.health_index(next_state) >= *threshold
        }
        ConditionalTrigger::DimensionBelow {
            dimension,
            threshold,
        } => next_state.z[*dimension] < *threshold,
        ConditionalTrigger::DimensionAbove {
            dimension,
            threshold,
        } => next_state.z[*dimension] > *threshold,
        ConditionalTrigger::DimensionCrossesBelow {
            dimension,
            threshold,
        } => previous_state.z[*dimension] > *threshold && next_state.z[*dimension] <= *threshold,
        ConditionalTrigger::DimensionCrossesAbove {
            dimension,
            threshold,
        } => previous_state.z[*dimension] < *threshold && next_state.z[*dimension] >= *threshold,
    }
}

fn is_effective_action(action: &Action) -> bool {
    !matches!(action.action_type, ActionType::Hold)
        || action.magnitude != 0.0
        || action.dimension.is_some()
        || action.metadata.is_some()
}

fn build_result(
    path_results: Vec<PathResult>,
    config: &MonteCarloConfig,
    retain_paths: bool,
) -> MonteCarloResult {
    let time_steps = config.time_steps;
    let num_paths = path_results.len();

    let mut mean_trajectory = vec![0.0; time_steps];
    let mut columns: Vec<Vec<f64>> = vec![Vec::with_capacity(num_paths); time_steps];

    for path in &path_results {
        for (t, &val) in path.health_indices.iter().enumerate() {
            mean_trajectory[t] += val;
            columns[t].push(val);
        }
    }

    for t in 0..time_steps {
        mean_trajectory[t] /= num_paths as f64;
        columns[t].sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }

    let percentiles = PercentileBands {
        p10: columns.iter().map(|c| percentile(c, 0.10)).collect(),
        p25: columns.iter().map(|c| percentile(c, 0.25)).collect(),
        p50: columns.iter().map(|c| percentile(c, 0.50)).collect(),
        p75: columns.iter().map(|c| percentile(c, 0.75)).collect(),
        p90: columns.iter().map(|c| percentile(c, 0.90)).collect(),
    };

    MonteCarloResult {
        paths: if retain_paths { path_results } else { vec![] },
        percentiles,
        mean_trajectory,
        config: config.clone(),
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[derive(Debug, Error)]
pub enum MonteCarloError {
    #[error("num_paths must be greater than zero")]
    ZeroPaths,
    #[error("invalid scenario: {0}")]
    InvalidScenario(crate::ScenarioError),
    #[error(
        "scenario time_steps ({scenario_time_steps}) must match Monte Carlo time_steps ({monte_carlo_time_steps})"
    )]
    TimeStepsMismatch {
        scenario_time_steps: usize,
        monte_carlo_time_steps: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConditionalActionRule, ConditionalTrigger, Scenario};

    struct DriftSim;

    impl Simulator for DriftSim {
        fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
            use rand::Rng;

            let mut next = state.clone();
            next.t += 1;
            for i in 0..next.z.len() {
                let noise = (rng.gen::<f64>() - 0.5) * 0.05;
                next.z[i] = (next.z[i] + action.magnitude * 0.01 + noise).clamp(0.0, 1.0);
            }
            next
        }
    }

    struct DeterministicAddSim;

    impl Simulator for DeterministicAddSim {
        fn step(
            &self,
            state: &SimState,
            action: &Action,
            _rng: &mut dyn rand::RngCore,
        ) -> SimState {
            let mut next = state.clone();
            next.t += 1;
            for value in &mut next.z {
                *value = (*value + action.magnitude).clamp(0.0, 1.0);
            }
            next
        }
    }

    #[test]
    fn test_monte_carlo_runs() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let actions = vec![Action {
            dimension: Some(0),
            magnitude: 1.0,
            action_type: ActionType::Intervention,
            metadata: None,
        }];
        let config = MonteCarloConfig::with_seed(100, 30, 42);

        let result = run_monte_carlo(&sim, &initial, &actions, &config, false);

        assert_eq!(result.mean_trajectory.len(), 30);
        assert_eq!(result.percentiles.p50.len(), 30);
        assert!(result.paths.is_empty());
    }

    #[test]
    fn test_deterministic() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let config = MonteCarloConfig::with_seed(50, 20, 123);

        let r1 = run_monte_carlo(&sim, &initial, &[], &config, false);
        let r2 = run_monte_carlo(&sim, &initial, &[], &config, false);

        assert_eq!(r1.mean_trajectory, r2.mean_trajectory);
    }

    #[test]
    fn test_retain_paths() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let config = MonteCarloConfig::with_seed(10, 5, 0);

        let result = run_monte_carlo(&sim, &initial, &[], &config, true);

        assert_eq!(result.paths.len(), 10);
        assert_eq!(result.paths[0].health_indices.len(), 5);
    }

    #[test]
    fn test_zero_paths_rejected() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let config = MonteCarloConfig::with_seed(0, 5, 0);

        let err = run_monte_carlo_checked(&sim, &initial, &[], &config, false).unwrap_err();
        assert!(matches!(err, MonteCarloError::ZeroPaths));
    }

    #[test]
    fn test_percentile_ordering() {
        let sim = DriftSim;
        let initial = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        let config = MonteCarloConfig::with_seed(32, 10, 3);

        let result = run_monte_carlo(&sim, &initial, &[], &config, false);

        for t in 0..config.time_steps {
            assert!(result.percentiles.p10[t] <= result.percentiles.p25[t]);
            assert!(result.percentiles.p25[t] <= result.percentiles.p50[t]);
            assert!(result.percentiles.p50[t] <= result.percentiles.p75[t]);
            assert!(result.percentiles.p75[t] <= result.percentiles.p90[t]);
        }
    }

    #[test]
    fn test_scenario_monte_carlo_applies_conditional_action_after_crossing() {
        let sim = DeterministicAddSim;
        let mut scenario = Scenario::new("scenario-1", "Reactive", SimState::zeros(1), 3);
        scenario.initial_state.z[0] = 0.5;
        scenario.actions.push(Action {
            dimension: Some(0),
            magnitude: -0.2,
            action_type: ActionType::StressorOnset,
            metadata: None,
        });
        scenario.conditional_actions.push(ConditionalActionRule {
            id: "recover".into(),
            trigger: ConditionalTrigger::DimensionCrossesBelow {
                dimension: 0,
                threshold: 0.4,
            },
            action: Action {
                dimension: Some(0),
                magnitude: 0.15,
                action_type: ActionType::Intervention,
                metadata: None,
            },
            delay_steps: 0,
            cooldown_steps: 0,
            priority: 1,
            max_fires: Some(1),
        });

        let result =
            run_scenario_monte_carlo(&sim, &scenario, &MonteCarloConfig::with_seed(1, 3, 7), true);

        let expected = [0.3, 0.45, 0.45];
        for (actual, expected) in result.paths[0].health_indices.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn test_scenario_monte_carlo_respects_cooldown() {
        let sim = DeterministicAddSim;
        let mut scenario = Scenario::new("scenario-1", "Reactive", SimState::zeros(1), 4);
        scenario.initial_state.z[0] = 0.5;
        scenario.conditional_actions.push(ConditionalActionRule {
            id: "nudge".into(),
            trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.6 },
            action: Action {
                dimension: None,
                magnitude: 0.05,
                action_type: ActionType::Intervention,
                metadata: None,
            },
            delay_steps: 0,
            cooldown_steps: 1,
            priority: 1,
            max_fires: None,
        });

        let result = run_scenario_monte_carlo(
            &sim,
            &scenario,
            &MonteCarloConfig::with_seed(1, 4, 11),
            true,
        );

        let trajectory = &result.paths[0].health_indices;
        assert_eq!(trajectory.len(), 4);
        assert!((trajectory[0] - 0.5).abs() < 1e-9);
        assert!(trajectory[1] > trajectory[0]);
        assert!((trajectory[2] - trajectory[1]).abs() < 1e-9);
        assert!(trajectory[3] > trajectory[2]);
    }

    #[test]
    fn test_scenario_monte_carlo_respects_max_fires() {
        let sim = DeterministicAddSim;
        let mut scenario = Scenario::new("scenario-1", "Reactive", SimState::zeros(1), 5);
        scenario.conditional_actions.push(ConditionalActionRule {
            id: "limited".into(),
            trigger: ConditionalTrigger::HealthIndexBelow { threshold: 1.0 },
            action: Action {
                dimension: Some(0),
                magnitude: 0.2,
                action_type: ActionType::Intervention,
                metadata: None,
            },
            delay_steps: 0,
            cooldown_steps: 0,
            priority: 0,
            max_fires: Some(2),
        });

        let result =
            run_scenario_monte_carlo(&sim, &scenario, &MonteCarloConfig::with_seed(1, 5, 7), true);

        let expected = [0.0, 0.2, 0.4, 0.4, 0.4];
        for (actual, expected) in result.paths[0].health_indices.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn test_scenario_monte_carlo_rejects_invalid_scenario() {
        let sim = DeterministicAddSim;
        let mut scenario = Scenario::new("scenario-1", "Reactive", SimState::zeros(1), 3);
        scenario.conditional_actions.push(ConditionalActionRule {
            id: "bad".into(),
            trigger: ConditionalTrigger::DimensionBelow {
                dimension: 1,
                threshold: 0.4,
            },
            action: Action::default(),
            delay_steps: 0,
            cooldown_steps: 0,
            priority: 0,
            max_fires: None,
        });

        let err = run_scenario_monte_carlo_checked(
            &sim,
            &scenario,
            &MonteCarloConfig::with_seed(1, 3, 0),
            false,
        )
        .unwrap_err();

        assert!(matches!(err, MonteCarloError::InvalidScenario(_)));
    }

    #[test]
    fn test_run_scenario_monte_carlo_rejects_time_step_mismatch() {
        let scenario = Scenario::new(
            "cond",
            "Conditional",
            SimState::new(vec![0.2], vec![0.0], vec![0.0]),
            4,
        );

        let err = run_scenario_monte_carlo_checked(
            &DeterministicAddSim,
            &scenario,
            &MonteCarloConfig::with_seed(1, 3, 7),
            true,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            MonteCarloError::TimeStepsMismatch {
                scenario_time_steps: 4,
                monte_carlo_time_steps: 3,
            }
        ));
    }
}
