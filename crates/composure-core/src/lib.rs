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

pub mod state;
pub mod simulator;
pub mod monte_carlo;
pub mod composure;
pub mod replay;
pub mod scenario;

pub use state::{SimState, Action, ActionType};
pub use simulator::Simulator;
pub use monte_carlo::{MonteCarloConfig, MonteCarloResult, PathResult, run_monte_carlo};
pub use composure::{
    ComposureCurve, ComposurePoint, Archetype, ComposureMetrics,
    analyze_composure, classify_archetype,
};
pub use replay::{ReplayRun, EventLog, EventEntry, EventKind, StateSnapshot};
pub use scenario::{Scenario, ScenarioError};
