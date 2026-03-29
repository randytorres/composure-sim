//! # composure-core
//!
//! Domain-agnostic simulation engine providing:
//!
//! - **State model** (`SimState`): `z_t` (functional state), `m_t` (accumulated memory),
//!   `u_t` (uncertainty) — reusable across health, biotech, marketing, wargaming, etc.
//! - **Monte Carlo engine**: Parallel (rayon) N-path simulation with seeded determinism.
//! - **Composure Curve**: Degradation/recovery analysis with archetype classification.
//! - **Event-sourced replay**: Deterministic, replayable simulation runs with state snapshots.
//! - **Trajectory comparison**: Counterfactual/baseline-vs-candidate comparison artifacts.
//! - **Experiment bundles**: Portable specs, parameter sets, and run records.
//! - **Sensitivity analysis**: Sweep definitions, generated cases, and parameter influence ranking.
//! - **Run summaries**: Deterministic scalar summaries for report-ready downstream use.
//! - **Sweep execution**: Case generation, execution, scalar objective extraction, and ranking.
//!
//! # Design Principles
//!
//! This library provides the **orchestration and math**. Domain-specific logic
//! (what "health" means, what "marketing engagement" means) is injected via the
//! `Simulator` trait. The library never knows what domain it's simulating.
//!
//! Extracted from patterns in:
//! - `forge-sim` (marketing-os) — Monte Carlo runner, seeded engine
//! - `wargame-engine` — Event-sourced replay, deterministic seeding
//! - `sim` ProfileBuilder.ts — Composure Curve archetype classification

pub mod compare;
pub mod composure;
pub mod execution;
pub mod experiment;
pub mod monte_carlo;
pub mod replay;
pub mod run_summary;
pub mod scenario;
pub mod sensitivity;
pub mod simulator;
pub mod state;
pub mod sweep_runner;

pub use compare::{
    compare_monte_carlo_results, compare_trajectories, CompareError, ComparisonConfig,
    ComparisonMetrics, DivergenceWindow, FailureComparison, FailureComparisonOutcome, PointDelta,
    PointDeltaSummary, TrajectoryComparison,
};
pub use composure::{
    analyze_composure, analyze_composure_checked, classify_archetype, Archetype, ComposureCurve,
    ComposureError, ComposureMetrics, ComposurePoint,
};
pub use execution::{
    execute_experiment_spec, execute_parameter_set, ExperimentExecutionConfig,
    ExperimentExecutionError,
};
pub use experiment::{
    ExperimentBundle, ExperimentError, ExperimentOutcome, ExperimentParameterSet,
    ExperimentRunRecord, ExperimentRunStatus, ExperimentSpec,
};
pub use monte_carlo::{
    run_monte_carlo, run_monte_carlo_checked, MonteCarloConfig, MonteCarloError, MonteCarloResult,
    PathResult,
};
pub use replay::{EventEntry, EventKind, EventLog, ReplayRun, StateSnapshot};
pub use run_summary::{
    summarize_composure, summarize_monte_carlo, summarize_run, ComposureSummary, MonteCarloSummary,
    RunSummary,
};
pub use scenario::{Scenario, ScenarioError};
pub use sensitivity::{
    analyze_sensitivity, generate_sweep_cases, CategoricalBucketSummary,
    CategoricalSensitivityStats, NumericSensitivityStats, ObjectiveSummary, ParameterSensitivity,
    ParameterValue, SensitivityConfig, SensitivityDirection, SensitivityError, SensitivityKind,
    SensitivityReport, SweepCase, SweepDefinition, SweepParameter, SweepSample, SweepStrategy,
};
pub use simulator::Simulator;
pub use state::{Action, ActionType, SimState, SimStateError};
pub use sweep_runner::{
    execute_sweep, ExecutedSweepCase, SweepExecutionResult, SweepRunnerConfig, SweepRunnerError,
};
