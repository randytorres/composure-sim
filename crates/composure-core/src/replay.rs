//! Event-sourced replay: deterministic, replayable simulation runs.
//!
//! Pattern extracted from `wargame-engine`: every simulation run produces
//! an immutable log of events and state snapshots that can be replayed
//! for debugging, auditing, or visualization.

use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    analyze_composure_checked, build_deterministic_report, compare_monte_carlo_results,
    counterfactual::validate_branch_input,
    monte_carlo::{
        actions_for_step, schedule_conditional_actions, ConditionalActionState, MonteCarloConfig,
        MonteCarloResult, PathResult, PercentileBands,
    },
    summarize_run, Action, ComparisonConfig, CounterfactualBranch, CounterfactualBranchInput,
    CounterfactualError, CounterfactualResult, ExperimentExecutionConfig, ExperimentOutcome,
    Scenario, ScenarioError, SimState, Simulator,
};

pub type ReplayRng = ChaCha12Rng;

/// Serializable continuation state required to resume execution exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayContinuation {
    pub rng_state: ReplayRng,
    pub conditional_state: ConditionalActionState,
}

/// A complete replay of a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRun {
    /// Unique identifier for this run.
    pub run_id: String,
    /// Seed used for deterministic replay.
    pub seed: u64,
    /// Optional source scenario when the replay came from scenario execution.
    pub scenario: Option<Scenario>,
    /// Continuation state before the first step.
    pub initial_continuation: Option<ReplayContinuation>,
    /// Final state at end of run.
    pub final_state: SimState,
    /// State snapshots at each time step.
    pub state_snapshots: Vec<StateSnapshot>,
    /// Ordered event log.
    pub event_log: EventLog,
}

impl ReplayRun {
    pub fn checkpoint_at(&self, t: usize) -> Option<(&SimState, &ReplayContinuation)> {
        if t == 0 {
            return self
                .scenario
                .as_ref()
                .zip(self.initial_continuation.as_ref())
                .map(|(scenario, continuation)| (&scenario.initial_state, continuation));
        }

        self.state_snapshots
            .iter()
            .find(|snapshot| snapshot.t == t)
            .and_then(|snapshot| {
                snapshot
                    .continuation
                    .as_ref()
                    .map(|continuation| (&snapshot.state, continuation))
            })
    }
}

/// Snapshot of the full state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub t: usize,
    pub state: SimState,
    pub health_index: f64,
    pub continuation: Option<ReplayContinuation>,
}

/// Ordered log of events during a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLog {
    pub entries: Vec<EventEntry>,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, entry: EventEntry) {
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Filter events by kind.
    pub fn filter_kind(&self, kind: &EventKind) -> Vec<&EventEntry> {
        self.entries.iter().filter(|e| &e.kind == kind).collect()
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

/// A single event in the simulation log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    /// Monotonically increasing sequence number.
    pub sequence: u64,
    /// Time step when this event occurred.
    pub t: usize,
    /// What happened.
    pub kind: EventKind,
    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,
}

/// Types of events that can occur during simulation.
/// Domain-agnostic; consumers can use `Custom(String)` for domain-specific events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventKind {
    /// Simulation step started.
    StepStarted,
    /// An action was applied.
    ActionApplied,
    /// State transition occurred (with delta info).
    StateTransition,
    /// Threshold crossed (break point, recovery, etc.)
    ThresholdCrossed { dimension: usize, direction: String },
    /// Simulation step completed.
    StepCompleted,
    /// Run started.
    RunStarted,
    /// Run completed.
    RunCompleted,
    /// Domain-specific event.
    Custom(String),
}

/// Builder for constructing replay runs during simulation.
pub struct ReplayBuilder {
    run_id: String,
    seed: u64,
    scenario: Option<Scenario>,
    initial_continuation: Option<ReplayContinuation>,
    snapshots: Vec<StateSnapshot>,
    log: EventLog,
    sequence: u64,
}

impl ReplayBuilder {
    pub fn new(run_id: impl Into<String>, seed: u64) -> Self {
        let mut builder = Self {
            run_id: run_id.into(),
            seed,
            scenario: None,
            initial_continuation: None,
            snapshots: Vec::new(),
            log: EventLog::new(),
            sequence: 0,
        };
        builder.emit(0, EventKind::RunStarted, None);
        builder
    }

    pub fn with_scenario(run_id: impl Into<String>, seed: u64, scenario: Scenario) -> Self {
        let mut builder = Self::new(run_id, seed);
        builder.scenario = Some(scenario);
        builder
    }

    pub fn set_initial_continuation(&mut self, continuation: ReplayContinuation) {
        self.initial_continuation = Some(continuation);
    }

    /// Record a state snapshot.
    pub fn snapshot(&mut self, state: &SimState, health_index: f64) {
        self.snapshot_with_continuation(state, health_index, None);
    }

    /// Record a state snapshot including resumption state.
    pub fn snapshot_with_continuation(
        &mut self,
        state: &SimState,
        health_index: f64,
        continuation: Option<ReplayContinuation>,
    ) {
        self.snapshots.push(StateSnapshot {
            t: state.t,
            state: state.clone(),
            health_index,
            continuation,
        });
    }

    /// Emit an event.
    pub fn emit(&mut self, t: usize, kind: EventKind, metadata: Option<serde_json::Value>) {
        self.log.push(EventEntry {
            sequence: self.sequence,
            t,
            kind,
            metadata,
        });
        self.sequence += 1;
    }

    /// Finalize into a `ReplayRun`.
    pub fn finish(mut self, final_state: SimState) -> ReplayRun {
        let t = final_state.t;
        self.emit(t, EventKind::RunCompleted, None);
        ReplayRun {
            run_id: self.run_id,
            seed: self.seed,
            scenario: self.scenario,
            initial_continuation: self.initial_continuation,
            final_state,
            state_snapshots: self.snapshots,
            event_log: self.log,
        }
    }
}

/// Deterministic settings for replay-aware branching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayCounterfactualConfig {
    pub time_steps: usize,
    pub execution: ExperimentExecutionConfig,
    pub comparison: ComparisonConfig,
    pub analysis_failure_threshold: Option<f64>,
}

impl ReplayCounterfactualConfig {
    pub fn new(time_steps: usize) -> Self {
        Self {
            time_steps,
            execution: ExperimentExecutionConfig::default(),
            comparison: ComparisonConfig::default(),
            analysis_failure_threshold: None,
        }
    }
}

pub fn run_scenario_replay<S: Simulator>(
    run_id: impl Into<String>,
    sim: &S,
    scenario: &Scenario,
    seed: u64,
) -> ReplayRun {
    run_scenario_replay_checked(run_id, sim, scenario, seed).expect("invalid replay scenario")
}

pub fn run_scenario_replay_checked<S: Simulator>(
    run_id: impl Into<String>,
    sim: &S,
    scenario: &Scenario,
    seed: u64,
) -> Result<ReplayRun, ReplayError> {
    scenario.validate().map_err(ReplayError::InvalidScenario)?;

    let mut rng = ReplayRng::seed_from_u64(seed);
    let mut state = scenario.initial_state.clone();
    let mut conditional_state = ConditionalActionState::new(scenario.conditional_actions.len());
    let mut replay = ReplayBuilder::with_scenario(run_id, seed, scenario.clone());
    replay.set_initial_continuation(ReplayContinuation {
        rng_state: rng.clone(),
        conditional_state: conditional_state.clone(),
    });

    for step in 0..scenario.time_steps {
        replay.emit(step, EventKind::StepStarted, None);
        let actions = actions_for_step(scenario, step, &mut conditional_state);
        if !actions.is_empty() {
            replay.emit(
                step,
                EventKind::ActionApplied,
                Some(serde_json::json!({ "count": actions.len() })),
            );
        }

        let previous_state = state.clone();
        state = sim.step_actions(&state, &actions, &mut rng);
        let health_index = sim.health_index(&state);
        replay.emit(step, EventKind::StateTransition, None);
        schedule_conditional_actions(
            sim,
            scenario,
            &previous_state,
            &state,
            step,
            scenario.time_steps,
            &mut conditional_state,
        );
        replay.snapshot_with_continuation(
            &state,
            health_index,
            Some(ReplayContinuation {
                rng_state: rng.clone(),
                conditional_state: conditional_state.clone(),
            }),
        );
        replay.emit(step, EventKind::StepCompleted, None);
    }

    Ok(replay.finish(state))
}

pub fn run_replay_counterfactual<S: Simulator>(
    sim: &S,
    replay: &ReplayRun,
    branch_from_t: usize,
    baseline: &CounterfactualBranchInput,
    candidate: &CounterfactualBranchInput,
    config: &ReplayCounterfactualConfig,
) -> Result<CounterfactualResult, ReplayCounterfactualError> {
    if config.time_steps == 0 {
        return Err(ReplayCounterfactualError::ZeroTimeSteps);
    }
    config
        .comparison
        .validate()
        .map_err(ReplayCounterfactualError::Compare)?;

    let (checkpoint_state, continuation) = replay
        .checkpoint_at(branch_from_t)
        .ok_or(ReplayCounterfactualError::MissingCheckpoint(branch_from_t))?;
    let source_scenario = replay
        .scenario
        .as_ref()
        .ok_or(ReplayCounterfactualError::MissingScenario)?;

    validate_branch_input(checkpoint_state, baseline)
        .map_err(ReplayCounterfactualError::Counterfactual)?;
    validate_branch_input(checkpoint_state, candidate)
        .map_err(ReplayCounterfactualError::Counterfactual)?;

    if baseline.branch_id == candidate.branch_id {
        return Err(ReplayCounterfactualError::Counterfactual(
            CounterfactualError::DuplicateBranchId(baseline.branch_id.clone()),
        ));
    }

    let baseline_branch = execute_replay_branch(
        sim,
        replay.seed,
        source_scenario,
        checkpoint_state,
        continuation,
        baseline,
        config,
    )?;
    let candidate_branch = execute_replay_branch(
        sim,
        replay.seed,
        source_scenario,
        checkpoint_state,
        continuation,
        candidate,
        config,
    )?;

    let baseline_monte_carlo = baseline_branch
        .outcome
        .monte_carlo
        .as_ref()
        .expect("replay branch always records a trajectory");
    let candidate_monte_carlo = candidate_branch
        .outcome
        .monte_carlo
        .as_ref()
        .expect("replay branch always records a trajectory");

    let comparison = compare_monte_carlo_results(
        baseline_monte_carlo,
        candidate_monte_carlo,
        &config.comparison,
    )
    .map_err(ReplayCounterfactualError::Compare)?;
    let report = build_deterministic_report(
        &baseline_branch.summary,
        &candidate_branch.summary,
        Some(&comparison),
    );

    Ok(CounterfactualResult {
        baseline: baseline_branch,
        candidate: candidate_branch,
        comparison,
        report,
    })
}

fn execute_replay_branch<S: Simulator>(
    sim: &S,
    seed: u64,
    source_scenario: &Scenario,
    checkpoint_state: &SimState,
    continuation: &ReplayContinuation,
    input: &CounterfactualBranchInput,
    config: &ReplayCounterfactualConfig,
) -> Result<CounterfactualBranch, ReplayCounterfactualError> {
    let scenario = build_replay_branch_scenario(
        source_scenario,
        checkpoint_state,
        input,
        config.time_steps,
        config.analysis_failure_threshold,
    );
    scenario
        .validate()
        .map_err(|source| ReplayCounterfactualError::InvalidBranchScenario {
            branch_id: input.branch_id.clone(),
            source,
        })?;

    let mut rng = continuation.rng_state.clone();
    let mut conditional_state = continuation.conditional_state.clone();
    let additional_rules = scenario
        .conditional_actions
        .len()
        .saturating_sub(source_scenario.conditional_actions.len());
    if additional_rules > 0 {
        conditional_state.extend_rules(additional_rules);
    }

    let start_step = checkpoint_state.t;
    let end_step = checkpoint_state.t + config.time_steps;
    let mut state = checkpoint_state.clone();
    let mut health_indices = Vec::with_capacity(config.time_steps);
    let mut replay = ReplayBuilder::with_scenario(
        format!("replay-counterfactual-{}", input.branch_id),
        seed,
        scenario.clone(),
    );
    replay.set_initial_continuation(ReplayContinuation {
        rng_state: rng.clone(),
        conditional_state: conditional_state.clone(),
    });

    for step in start_step..end_step {
        replay.emit(step, EventKind::StepStarted, None);
        let actions = actions_for_step(&scenario, step, &mut conditional_state);
        if !actions.is_empty() {
            replay.emit(
                step,
                EventKind::ActionApplied,
                Some(serde_json::json!({ "count": actions.len() })),
            );
        }

        let previous_state = state.clone();
        state = sim.step_actions(&state, &actions, &mut rng);
        let health_index = sim.health_index(&state);
        health_indices.push(health_index);
        replay.emit(step, EventKind::StateTransition, None);
        schedule_conditional_actions(
            sim,
            &scenario,
            &previous_state,
            &state,
            step,
            scenario.time_steps,
            &mut conditional_state,
        );
        replay.snapshot_with_continuation(
            &state,
            health_index,
            Some(ReplayContinuation {
                rng_state: rng.clone(),
                conditional_state: conditional_state.clone(),
            }),
        );
        replay.emit(step, EventKind::StepCompleted, None);
    }

    let replay_run = replay.finish(state.clone());
    let monte_carlo = single_path_result(seed, state.clone(), health_indices);
    let composure = if config.execution.analyze_composure {
        Some(
            analyze_composure_checked(
                &monte_carlo.mean_trajectory,
                config.analysis_failure_threshold.unwrap_or(0.0),
            )
            .map_err(|source| ReplayCounterfactualError::Composure {
                branch_id: input.branch_id.clone(),
                source,
            })?,
        )
    } else {
        None
    };
    let outcome = ExperimentOutcome {
        monte_carlo: Some(monte_carlo),
        composure,
        replay: Some(replay_run),
        metadata: None,
    };
    let summary = summarize_run(outcome.monte_carlo.as_ref(), outcome.composure.as_ref());

    Ok(CounterfactualBranch {
        branch_id: input.branch_id.clone(),
        branch_from_t: checkpoint_state.t,
        intervention_label: input.intervention_label.clone(),
        outcome,
        summary,
    })
}

fn build_replay_branch_scenario(
    source_scenario: &Scenario,
    checkpoint_state: &SimState,
    input: &CounterfactualBranchInput,
    time_steps: usize,
    analysis_failure_threshold: Option<f64>,
) -> Scenario {
    let start_step = checkpoint_state.t;
    let final_time_steps = checkpoint_state.t + time_steps;
    let mut scenario = source_scenario.clone();
    scenario.initial_state = checkpoint_state.clone();
    scenario.time_steps = final_time_steps;
    scenario.failure_threshold = analysis_failure_threshold;
    if scenario.actions.len() < final_time_steps {
        scenario.actions.resize(final_time_steps, Action::default());
    }

    for (offset, action) in input.actions.iter().enumerate().take(time_steps) {
        scenario.actions[start_step + offset] = action.clone();
    }

    scenario
        .conditional_actions
        .extend(input.conditional_actions.clone());
    scenario
}

fn single_path_result(
    seed: u64,
    final_state: SimState,
    health_indices: Vec<f64>,
) -> MonteCarloResult {
    let time_steps = health_indices.len();
    MonteCarloResult {
        paths: vec![PathResult {
            health_indices: health_indices.clone(),
            final_state,
            seed,
        }],
        percentiles: PercentileBands {
            p10: health_indices.clone(),
            p25: health_indices.clone(),
            p50: health_indices.clone(),
            p75: health_indices.clone(),
            p90: health_indices.clone(),
        },
        mean_trajectory: health_indices,
        config: MonteCarloConfig::with_seed(1, time_steps, seed),
    }
}

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error("invalid replay scenario: {0}")]
    InvalidScenario(ScenarioError),
}

#[derive(Debug, Error)]
pub enum ReplayCounterfactualError {
    #[error("replay-aware counterfactual requires a source scenario in the replay artifact")]
    MissingScenario,
    #[error("replay checkpoint for t={0} was not found")]
    MissingCheckpoint(usize),
    #[error("replay-aware counterfactual requires time_steps > 0")]
    ZeroTimeSteps,
    #[error("counterfactual validation failed: {0}")]
    Counterfactual(CounterfactualError),
    #[error("replay branch {branch_id} is invalid: {source}")]
    InvalidBranchScenario {
        branch_id: String,
        source: ScenarioError,
    },
    #[error("replay composure analysis failed for branch {branch_id}: {source}")]
    Composure {
        branch_id: String,
        source: crate::ComposureError,
    },
    #[error("replay counterfactual comparison failed: {0}")]
    Compare(crate::CompareError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionType, ConditionalActionRule, ConditionalTrigger};

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

    struct NoisySim;

    impl Simulator for NoisySim {
        fn step(&self, state: &SimState, action: &Action, rng: &mut dyn rand::RngCore) -> SimState {
            use rand::Rng;

            let mut next = state.clone();
            next.t += 1;
            let noise = (rng.gen::<f64>() - 0.5) * 0.1;
            for value in &mut next.z {
                *value = (*value + action.magnitude + noise).clamp(0.0, 1.0);
            }
            next
        }
    }

    fn replay_scenario() -> Scenario {
        let mut scenario = Scenario::new(
            "replay",
            "Replay",
            SimState::new(vec![0.4], vec![0.0], vec![0.0]),
            4,
        );
        scenario.actions = vec![
            Action {
                dimension: Some(0),
                magnitude: -0.1,
                action_type: ActionType::Intervention,
                metadata: None,
            },
            Action {
                dimension: Some(0),
                magnitude: 0.0,
                action_type: ActionType::Hold,
                metadata: None,
            },
            Action {
                dimension: Some(0),
                magnitude: 0.0,
                action_type: ActionType::Hold,
                metadata: None,
            },
            Action {
                dimension: Some(0),
                magnitude: 0.0,
                action_type: ActionType::Hold,
                metadata: None,
            },
        ];
        scenario.conditional_actions.push(ConditionalActionRule {
            id: "rescue".into(),
            trigger: ConditionalTrigger::HealthIndexBelow { threshold: 0.35 },
            action: Action {
                dimension: Some(0),
                magnitude: 0.3,
                action_type: ActionType::Intervention,
                metadata: None,
            },
            delay_steps: 1,
            cooldown_steps: 10,
            priority: 1,
            max_fires: Some(1),
        });
        scenario
    }

    #[test]
    fn test_replay_builder() {
        let mut builder = ReplayBuilder::new("test-run-1", 42);

        let state = SimState::new(vec![0.5], vec![0.0], vec![0.5]);
        builder.snapshot(&state, 0.5);
        builder.emit(0, EventKind::ActionApplied, None);

        let replay = builder.finish(state);

        assert_eq!(replay.run_id, "test-run-1");
        assert_eq!(replay.seed, 42);
        assert_eq!(replay.state_snapshots.len(), 1);
        // RunStarted + ActionApplied + RunCompleted = 3
        assert_eq!(replay.event_log.len(), 3);
    }

    #[test]
    fn test_event_filter() {
        let mut log = EventLog::new();
        log.push(EventEntry {
            sequence: 0,
            t: 0,
            kind: EventKind::RunStarted,
            metadata: None,
        });
        log.push(EventEntry {
            sequence: 1,
            t: 0,
            kind: EventKind::ActionApplied,
            metadata: None,
        });
        log.push(EventEntry {
            sequence: 2,
            t: 1,
            kind: EventKind::ActionApplied,
            metadata: None,
        });
        log.push(EventEntry {
            sequence: 3,
            t: 1,
            kind: EventKind::RunCompleted,
            metadata: None,
        });

        let actions = log.filter_kind(&EventKind::ActionApplied);
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn test_event_filter_matches_full_variant() {
        let mut log = EventLog::new();
        log.push(EventEntry {
            sequence: 0,
            t: 0,
            kind: EventKind::Custom("foo".into()),
            metadata: None,
        });
        log.push(EventEntry {
            sequence: 1,
            t: 0,
            kind: EventKind::Custom("bar".into()),
            metadata: None,
        });

        let events = log.filter_kind(&EventKind::Custom("foo".into()));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Custom("foo".into()));
    }

    #[test]
    fn test_run_scenario_replay_records_continuation_state() {
        let scenario = replay_scenario();
        let replay =
            run_scenario_replay_checked("replay-run", &DeterministicAddSim, &scenario, 7).unwrap();

        assert!(replay.scenario.is_some());
        assert!(replay.initial_continuation.is_some());
        assert_eq!(replay.state_snapshots.len(), 4);
        assert!(replay
            .state_snapshots
            .iter()
            .all(|snapshot| snapshot.continuation.is_some()));
    }

    #[test]
    fn test_run_replay_counterfactual_preserves_pending_delayed_actions() {
        let scenario = replay_scenario();
        let replay =
            run_scenario_replay_checked("replay-run", &DeterministicAddSim, &scenario, 7).unwrap();
        let config = ReplayCounterfactualConfig {
            time_steps: 2,
            execution: ExperimentExecutionConfig {
                retain_paths: true,
                analyze_composure: false,
            },
            comparison: ComparisonConfig::default(),
            analysis_failure_threshold: None,
        };
        let baseline = CounterfactualBranchInput {
            branch_id: "baseline".into(),
            intervention_label: "baseline".into(),
            actions: Vec::new(),
            conditional_actions: Vec::new(),
        };
        let candidate = CounterfactualBranchInput {
            branch_id: "candidate".into(),
            intervention_label: "candidate".into(),
            actions: vec![Action {
                dimension: Some(0),
                magnitude: 0.1,
                action_type: ActionType::Intervention,
                metadata: None,
            }],
            conditional_actions: Vec::new(),
        };

        let result = run_replay_counterfactual(
            &DeterministicAddSim,
            &replay,
            1,
            &baseline,
            &candidate,
            &config,
        )
        .unwrap();

        let baseline_series = &result
            .baseline
            .outcome
            .monte_carlo
            .as_ref()
            .unwrap()
            .mean_trajectory;
        assert_eq!(baseline_series.len(), 2);
        assert!(baseline_series[0] > 0.3);
        assert!(result.comparison.metrics.end_delta >= 0.0);
    }

    #[test]
    fn test_run_replay_counterfactual_preserves_rng_continuity() {
        let mut scenario = Scenario::new(
            "replay-noise",
            "Replay Noise",
            SimState::new(vec![0.5], vec![0.0], vec![0.0]),
            5,
        );
        scenario.actions = vec![
            Action {
                dimension: Some(0),
                magnitude: 0.05,
                action_type: ActionType::Intervention,
                metadata: None,
            };
            5
        ];

        let replay = run_scenario_replay_checked("replay-noise", &NoisySim, &scenario, 11).unwrap();
        let config = ReplayCounterfactualConfig {
            time_steps: 2,
            execution: ExperimentExecutionConfig {
                retain_paths: true,
                analyze_composure: false,
            },
            comparison: ComparisonConfig::default(),
            analysis_failure_threshold: None,
        };
        let baseline = CounterfactualBranchInput {
            branch_id: "baseline".into(),
            intervention_label: "baseline".into(),
            actions: Vec::new(),
            conditional_actions: Vec::new(),
        };
        let candidate = CounterfactualBranchInput {
            branch_id: "candidate".into(),
            intervention_label: "candidate".into(),
            actions: Vec::new(),
            conditional_actions: Vec::new(),
        };

        let branched =
            run_replay_counterfactual(&NoisySim, &replay, 3, &baseline, &candidate, &config)
                .unwrap();
        let expected: Vec<f64> = replay
            .state_snapshots
            .iter()
            .filter(|snapshot| snapshot.t > 3)
            .map(|snapshot| snapshot.health_index)
            .collect();

        assert_eq!(
            branched
                .baseline
                .outcome
                .monte_carlo
                .as_ref()
                .unwrap()
                .mean_trajectory,
            expected
        );
    }
}
