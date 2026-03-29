//! # composure-core
//!
//! Domain-agnostic simulation engine providing:
//!
//! - **State model** (`SimState`): `z_t` (functional state), `m_t` (accumulated memory),
//!   `u_t` (uncertainty) — reusable across health, biotech, marketing, wargaming, etc.
//! - **Monte Carlo engine**: Parallel (rayon) N-path simulation with seeded determinism.
//! - **Composure Curve**: Degradation/recovery analysis with archetype classification.
//! - **Event-sourced replay**: Deterministic, replayable simulation runs with state snapshots.
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

pub mod composure;
pub mod monte_carlo;
pub mod replay;
pub mod scenario;
pub mod simulator;
pub mod state;

pub use composure::{
    analyze_composure, analyze_composure_checked, classify_archetype, Archetype, ComposureCurve,
    ComposureError, ComposureMetrics, ComposurePoint,
};
pub use monte_carlo::{
    run_monte_carlo, run_monte_carlo_checked, MonteCarloConfig, MonteCarloError, MonteCarloResult,
    PathResult,
};
pub use replay::{EventEntry, EventKind, EventLog, ReplayRun, StateSnapshot};
pub use scenario::{Scenario, ScenarioError};
pub use simulator::Simulator;
pub use state::{Action, ActionType, SimState, SimStateError};
